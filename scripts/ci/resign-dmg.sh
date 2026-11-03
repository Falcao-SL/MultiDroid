#!/usr/bin/env bash
# Reassina o .app dentro de um .dmg (ou um .app solto) com Developer ID +
# hardened runtime, remonta o .dmg e assina o volume. Usado no CI da Preview
# Rust quando o cargo-packager gerou o .dmg sem certificado.
#
# Requer APPLE_SIGN_IDENTITY no ambiente (preencha com import-apple-cert.sh).
# Uso:
#   resign-dmg.sh <entrada.dmg|entrada.app> <saida.dmg> [nome-do-volume]
set -euo pipefail

IN=${1:?}
OUT=${2:?}
VOL=${3:-MultiDroid}
ID=${APPLE_SIGN_IDENTITY:?APPLE_SIGN_IDENTITY vazio — rode import-apple-cert.sh antes}

WORK=$(mktemp -d)
trap 'hdiutil detach "$WORK/mnt" -force -quiet 2>/dev/null || true; rm -rf "$WORK"' EXIT
APP="$WORK/App.app"

case "$IN" in
  *.dmg)
    mkdir -p "$WORK/mnt"
    hdiutil attach "$IN" -mountpoint "$WORK/mnt" -nobrowse -quiet
    SRC=$(find "$WORK/mnt" -maxdepth 1 -name '*.app' -print -quit)
    [ -n "$SRC" ] || { echo "erro: nenhum .app em $IN" >&2; exit 1; }
    ditto "$SRC" "$APP"
    hdiutil detach "$WORK/mnt" -quiet
    ;;
  *.app)
    ditto "$IN" "$APP"
    ;;
  *)
    echo "erro: entrada deve ser .dmg ou .app" >&2
    exit 1
    ;;
esac

codesign --force --options runtime --timestamp --sign "$ID" "$APP"
codesign --verify --strict --deep "$APP"

MB=$(( $(du -sm "$APP" | cut -f1) + 40 ))
hdiutil create -size "${MB}m" -fs APFS -volname "$VOL" "$WORK/rw.dmg" >/dev/null 2>&1
mkdir "$WORK/mnt"
hdiutil attach "$WORK/rw.dmg" -mountpoint "$WORK/mnt" -nobrowse -quiet
ditto "$APP" "$WORK/mnt/$(basename "$APP")"
ln -s /Applications "$WORK/mnt/Applications"
for i in 1 2 3 4 5 6 7 8 9 10; do
  hdiutil detach "$WORK/mnt" -quiet 2>/dev/null && break
  [ "$i" = 10 ] && hdiutil detach "$WORK/mnt" -force -quiet
  sleep 2
done
rm -f "$OUT"
hdiutil convert "$WORK/rw.dmg" -format ULFO -o "$OUT" >/dev/null
codesign --force --timestamp --sign "$ID" "$OUT"
codesign --verify --verbose=2 "$OUT"
echo "ok: $OUT"
