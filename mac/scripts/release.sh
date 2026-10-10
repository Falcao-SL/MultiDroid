#!/usr/bin/env bash
# Release para distribuir fora da Mac App Store (DISTRIBUICAO.md):
# app assinado com Developer ID + hardened runtime, dentro de um .dmg
# assinado, notarizado pela Apple e grampeado.
#   scripts/release.sh
# Uma vez por Mac, guarde a credencial da notarização no Keychain (pede uma
# senha de app criada em appleid.apple.com):
#   xcrun notarytool store-credentials notary --apple-id <e-mail> --team-id 6EZ89E2F5S
# NOTARY_PROFILE troca o nome do perfil; NOTARIZE=0 só monta e assina.
set -euo pipefail
cd "$(dirname "$0")/.."
PROFILE=${NOTARY_PROFILE:-notary}
scripts/build-app.sh release
APP=build/MultiDroid.app
codesign --verify --strict --deep "$APP"
ID=$(codesign -dvv "$APP" 2>&1 | sed -n 's/^Authority=\(Developer ID Application.*\)/\1/p')
[ -n "$ID" ] || { echo "erro: o app não está assinado com Developer ID" >&2; exit 1; }
VERSION=$(/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" "$APP/Contents/Info.plist")
DMG=build/MultiDroid-$VERSION.dmg

# .dmg com o app e um atalho para Aplicativos, para arrastar. Criar direto de
# uma pasta (hdiutil -srcfolder / diskutil image create from) falha no macOS 27
# com "Resource busy"; imagem vazia, montar, copiar e converter funciona.
WORK=$(mktemp -d)
trap 'hdiutil detach "$WORK/mnt" -force -quiet 2>/dev/null || true; rm -rf "$WORK"' EXIT
MB=$(( $(du -sm "$APP" | cut -f1) + 40 ))
hdiutil create -size "${MB}m" -fs APFS -volname MultiDroid "$WORK/rw.dmg" >/dev/null 2>&1
mkdir "$WORK/mnt"
hdiutil attach "$WORK/rw.dmg" -mountpoint "$WORK/mnt" -nobrowse -quiet
ditto "$APP" "$WORK/mnt/MultiDroid.app"
ln -s /Applications "$WORK/mnt/Applications"
# O Spotlight segura o volume recém-copiado por alguns segundos.
for i in 1 2 3 4 5 6 7 8 9 10; do
  hdiutil detach "$WORK/mnt" -quiet 2>/dev/null && break
  [ $i = 10 ] && hdiutil detach "$WORK/mnt" -force -quiet
  sleep 2
done
rm -f "$DMG"
hdiutil convert "$WORK/rw.dmg" -format ULFO -o "$DMG" >/dev/null
codesign --force --timestamp --sign "$ID" "$DMG"

if [ "${NOTARIZE:-1}" = 0 ]; then echo "ok: $DMG (assinado, sem notarizar)"; exit 0; fi
xcrun notarytool submit "$DMG" --keychain-profile "$PROFILE" --wait
xcrun stapler staple "$DMG"
spctl --assess --type open --context context:primary-signature -v "$DMG"
echo "ok: $DMG (notarizado)"
