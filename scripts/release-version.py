import json,os,re,tomllib
from pathlib import Path
root=Path(__file__).resolve().parents[1]
tag=os.environ['REQUESTED_TAG']
if not re.fullmatch(r'v\d+\.\d+(?:\.\d+)?(?:-[A-Za-z0-9.]+)?',tag): raise SystemExit('Invalid release tag')
version=tag[1:]; base=version.split('-')[0]
rust=tomllib.loads((root/'rust/Cargo.toml').read_text())['workspace']['package']['version']
native=version==(root/'mac/VERSION').read_text().strip()
if not native and base!=rust: raise SystemExit('Version must match rust/Cargo.toml or mac/VERSION')
platform=os.environ.get('QA_PLATFORM','all')
if platform not in ('all','windows','linux','mac'):raise SystemExit('Unsupported QA platform')
if os.environ.get('PUBLISH')=='true' and platform!='all':raise SystemExit('Production publication requires all supported platforms')
if native and platform not in ('all','mac'):raise SystemExit('The native release is Mac only')
# Windows uses GitHub-hosted runners: the Sciensa self-hosted box blocks
# cargo build-script binaries under Application Control (os error 4551).
# Restore ['self-hosted','Windows','X64'] once WDAC allows target/debug/build/*.
runners=[{'runner':'windows-latest','platform':'windows','formats':'nsis'},{'runner':'ubuntu-22.04','platform':'linux','formats':'appimage,deb'}]
selected=[] if native else [item for item in runners if platform in ('all',item['platform'])]
with open(os.environ['GITHUB_OUTPUT'],'a') as output:
 for key,value in {'mac':str(platform in ('all','mac')).lower(),'desktop':str(bool(selected)).lower(),'matrix':json.dumps({'include':selected})}.items():output.write(f'{key}={value}\n')
 for key,value in {'tag':tag,'name':version,'native':str(native).lower(),'preview':str('-' in version).lower()}.items(): output.write(f'{key}={value}\n')
print('Validated',tag,'native Mac' if native else 'desktop preview')
