#!/usr/bin/env bash
# Importa o certificado Developer ID (.p12 em base64) num keychain temporário
# do runner. Usado pelos workflows de release (SwiftUI e Rust Preview).
#
# Segredos esperados no ambiente:
#   APPLE_CERTIFICATE_P12       — conteúdo do .p12 em base64
#   APPLE_CERTIFICATE_PASSWORD  — senha do .p12
# Opcional:
#   APPLE_SIGN_IDENTITY         — "Developer ID Application: …"
#                                 (senão usa a primeira Developer ID do keychain)
#
# Exporta APPLE_SIGN_IDENTITY e CI_KEYCHAIN no GITHUB_ENV quando roda no Actions.
set -euo pipefail

if [ -z "${APPLE_CERTIFICATE_P12:-}" ] || [ -z "${APPLE_CERTIFICATE_PASSWORD:-}" ]; then
  echo "aviso: APPLE_CERTIFICATE_P12/PASSWORD ausentes — sem importar certificado" >&2
  exit 0
fi

KEYCHAIN="${RUNNER_TEMP:-/tmp}/multidroid-signing.keychain-db"
KEYCHAIN_PASSWORD="${APPLE_KEYCHAIN_PASSWORD:-$(uuidgen)}"
P12="${RUNNER_TEMP:-/tmp}/multidroid-cert.p12"

security delete-keychain "$KEYCHAIN" 2>/dev/null || true
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security set-keychain-settings -lut 21600 "$KEYCHAIN"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"

echo "$APPLE_CERTIFICATE_P12" | base64 --decode >"$P12"
security import "$P12" -k "$KEYCHAIN" -P "$APPLE_CERTIFICATE_PASSWORD" \
  -T /usr/bin/codesign -T /usr/bin/security -T /usr/bin/productsign
security set-key-partition-list -S apple-tool:,apple:,codesign: \
  -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
security list-keychains -d user -s "$KEYCHAIN" $(security list-keychains -d user | tr -d '"')
rm -f "$P12"

if [ -z "${APPLE_SIGN_IDENTITY:-}" ]; then
  APPLE_SIGN_IDENTITY=$(security find-identity -v -p codesigning "$KEYCHAIN" \
    | sed -n 's/.*"\(Developer ID Application:.*\)"/\1/p' | head -1)
fi
[ -n "$APPLE_SIGN_IDENTITY" ] || {
  echo "erro: nenhuma identidade Developer ID Application no keychain" >&2
  exit 1
}

echo "ok: certificado importado ($APPLE_SIGN_IDENTITY)"
if [ -n "${GITHUB_ENV:-}" ]; then
  {
    echo "APPLE_SIGN_IDENTITY=$APPLE_SIGN_IDENTITY"
    echo "CI_KEYCHAIN=$KEYCHAIN"
    echo "CI_KEYCHAIN_PASSWORD=$KEYCHAIN_PASSWORD"
  } >>"$GITHUB_ENV"
fi
