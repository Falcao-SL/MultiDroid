#!/usr/bin/env bash
# Notariza um .dmg (ou .zip/.pkg) com notarytool via chave de API App Store Connect
# e grampeia o resultado. Sem os segredos, sai 0 com aviso (não falha o CI).
#
# Segredos:
#   APPLE_API_KEY      — conteúdo do .p8 (texto) OU o mesmo em base64
#   APPLE_API_KEY_ID   — Key ID
#   APPLE_API_ISSUER   — Issuer UUID
# Uso: scripts/ci/notarize.sh <arquivo.dmg>
set -euo pipefail

FILE=${1:?uso: notarize.sh <arquivo>}
[ -f "$FILE" ] || { echo "erro: não achei $FILE" >&2; exit 1; }

if [ -z "${APPLE_API_KEY:-}" ] || [ -z "${APPLE_API_KEY_ID:-}" ] || [ -z "${APPLE_API_ISSUER:-}" ]; then
  echo "aviso: APPLE_API_KEY/ID/ISSUER ausentes — $FILE sem notarizar" >&2
  exit 0
fi

KEY="${RUNNER_TEMP:-/tmp}/AuthKey_${APPLE_API_KEY_ID}.p8"
# Aceita PEM cru ou base64 do .p8.
if printf '%s' "$APPLE_API_KEY" | grep -q "BEGIN PRIVATE KEY"; then
  printf '%s\n' "$APPLE_API_KEY" >"$KEY"
else
  echo "$APPLE_API_KEY" | base64 --decode >"$KEY"
fi

xcrun notarytool submit "$FILE" \
  --key "$KEY" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER" \
  --wait
rm -f "$KEY"

case "$FILE" in
  *.dmg|*.pkg|*.app)
    xcrun stapler staple "$FILE"
    xcrun stapler validate "$FILE" || true
    ;;
esac
echo "ok: notarizado $FILE"
