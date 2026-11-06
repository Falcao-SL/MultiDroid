#!/usr/bin/env python3
"""Prepare a disposable keychain and notarization key from Phase and MinIO."""
import datetime
import hashlib
import hmac
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import urllib.parse
import urllib.request


def require(data, name):
    value = data.get(name)
    if not isinstance(value, str) or not value or '\n' in value or '\r' in value:
        raise ValueError(f'Missing or invalid Phase secret: {name}')
    return value


def download(endpoint, bucket, obj, access, secret):
    parsed = urllib.parse.urlsplit(endpoint)
    if parsed.scheme not in ('http', 'https') or not parsed.netloc or parsed.path not in ('', '/'):
        raise ValueError('Invalid MinIO endpoint')
    path = '/' + urllib.parse.quote(bucket + '/' + obj, safe='/~-._')
    now = datetime.datetime.now(datetime.timezone.utc)
    day, timestamp = now.strftime('%Y%m%d'), now.strftime('%Y%m%dT%H%M%SZ')
    digest = hashlib.sha256(b'').hexdigest()
    signed = 'host;x-amz-content-sha256;x-amz-date'
    headers = f'host:{parsed.netloc}\nx-amz-content-sha256:{digest}\nx-amz-date:{timestamp}\n'
    canonical = f'GET\n{path}\n\n{headers}\n{signed}\n{digest}'
    scope = f'{day}/us-east-1/s3/aws4_request'
    message = f'AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{hashlib.sha256(canonical.encode()).hexdigest()}'
    def sign(key, value):
        return hmac.new(key, value.encode(), hashlib.sha256).digest()
    key = sign(sign(sign(sign(('AWS4' + secret).encode(), day), 'us-east-1'), 's3'), 'aws4_request')
    signature = hmac.new(key, message.encode(), hashlib.sha256).hexdigest()
    auth = f'AWS4-HMAC-SHA256 Credential={access}/{scope}, SignedHeaders={signed}, Signature={signature}'
    req = urllib.request.Request(endpoint.rstrip('/') + path, headers={
        'Authorization': auth, 'x-amz-content-sha256': digest, 'x-amz-date': timestamp,
    })
    with urllib.request.urlopen(req, timeout=30) as response:
        return response.read()


def security(*args):
    subprocess.run(['security', *args], check=True, stdout=subprocess.DEVNULL,
                   stderr=subprocess.PIPE, timeout=60)


def main():
    result = subprocess.run(['phase', 'secrets', 'export', '--app', 'multidroid', '--env', 'production', '--format', 'json'],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
    if result.returncode:
        message = (result.stderr + result.stdout).decode(errors='replace').lower()
        indicators = ('401', '403', '404', 'unauthorized', 'forbidden', 'permission',
                      'decrypt', 'connect', 'token', 'hostname', 'host', 'tls', 'ssl', 'timeout', 'not found')
        present = ','.join(term for term in indicators if term in message) or 'unclassified'
        raise RuntimeError('Phase export failed; indicators=' + present)
    values = json.loads(result.stdout)
    required = ('MINIO_ENDPOINT', 'MINIO_BUCKET', 'MINIO_ACCESS_KEY', 'MINIO_SECRET_KEY',
                'APPLE_DEVELOPER_ID_P12_PASSWORD', 'APPLE_DEVELOPER_ID_OBJECT', 'APPLE_API_KEY_OBJECT',
                'APPLE_API_KEY_ID', 'APPLE_API_ISSUER_ID', 'APPLE_TEAM_ID')
    v = {name: require(values, name) for name in required}
    temp = Path(os.environ['RUNNER_TEMP'])
    run_id = os.environ['GITHUB_RUN_ID'] + '-' + os.environ['GITHUB_RUN_ATTEMPT']
    if not all(part.isdigit() for part in run_id.split('-')):
        raise ValueError('Invalid run ID')
    work = temp / ('multidroid-signing-' + run_id)
    work.mkdir(mode=0o700)
    p12, api_key, keychain = work / 'developer-id.p12', work / 'AuthKey.p8', work / 'signing.keychain-db'
    for target, obj in ((p12, v['APPLE_DEVELOPER_ID_OBJECT']), (api_key, v['APPLE_API_KEY_OBJECT'])):
        target.write_bytes(download(os.environ.get('MINIO_TUNNEL_ENDPOINT', v['MINIO_ENDPOINT']), v['MINIO_BUCKET'], obj, v['MINIO_ACCESS_KEY'], v['MINIO_SECRET_KEY']))
        target.chmod(0o600)
    password = secrets.token_urlsafe(32)
    security('create-keychain', '-p', password, str(keychain))
    security('set-keychain-settings', '-lut', '21600', str(keychain))
    security('unlock-keychain', '-p', password, str(keychain))
    security('import', str(p12), '-k', str(keychain), '-P', v['APPLE_DEVELOPER_ID_P12_PASSWORD'], '-T', '/usr/bin/codesign', '-T', '/usr/bin/security')
    security('set-key-partition-list', '-S', 'apple-tool:,apple:,codesign:', '-s', '-k', password, str(keychain))
    p12.unlink()
    identities = subprocess.run(['security', 'find-identity', '-v', '-p', 'codesigning', str(keychain)],
                                check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True).stdout
    if 'Developer ID Application' not in identities or '(' + v['APPLE_TEAM_ID'] + ')' not in identities:
        raise RuntimeError('Developer ID identity was not found in the temporary keychain')
    env = {'CI_KEYCHAIN': str(keychain), 'APPLE_NOTARY_KEY_PATH': str(api_key),
           'APPLE_API_KEY_ID': v['APPLE_API_KEY_ID'], 'APPLE_API_ISSUER_ID': v['APPLE_API_ISSUER_ID'],
           'APPLE_TEAM_ID': v['APPLE_TEAM_ID'], 'MULTIDROID_SIGNING_DIR': str(work)}
    with open(os.environ['GITHUB_ENV'], 'a', encoding='utf-8') as output:
        for name, value in env.items():
            output.write(f'{name}={value}\n')
    print('Temporary Developer ID keychain and Apple notarization key prepared.')


if __name__ == '__main__':
    try:
        main()
    except Exception as exc:
        detail = f' ({exc.cmd[1]})' if isinstance(exc, subprocess.CalledProcessError) and len(exc.cmd) > 1 else ''
        message = str(exc) if isinstance(exc, RuntimeError) and str(exc).startswith('Phase export failed;') else type(exc).__name__ + detail
        print('Failed to prepare Apple signing: ' + message, file=sys.stderr)
        sys.exit(1)
