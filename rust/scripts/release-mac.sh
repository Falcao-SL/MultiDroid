#!/usr/bin/env bash
# .dmg da versão Rust assinado com Developer ID e notarizado, gerado no Mac do
# dono, para quando o repositório ainda não tem os segredos da Apple
# (.github/workflows/release.yml). Usa o mesmo perfil do mac/scripts/release.sh:
#   xcrun notarytool store-credentials notary --apple-id <e-mail> --team-id 6EZ89E2F5S
# Uso: rust/scripts/release-mac.sh [tag]
#   com a tag (v0.3.0-alpha.1), troca o .dmg da release pelo assinado.
# NOTARY_PROFILE troca o nome do perfil; NOTARIZE=0 só assina.
set -euo pipefail
cd "$(dirname "$0")/../app"
TAG=${1:-}
PROFILE=${NOTARY_PROFILE:-notary}
ID=$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application:.*\)"/\1/p' | head -1)
[ -n "$ID" ] || { echo "erro: nenhum certificado Developer ID no Keychain" >&2; exit 1; }
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' ../Cargo.toml | head -1)
NOME=${TAG#v}; NOME=${NOME:-$VERSION}
rm -rf ../target/packages
# O packager monta o .app; a assinatura (hardened runtime + carimbo de tempo,
# que a notarização exige) e o .dmg são feitos aqui, como no mac/scripts/release.sh.
cargo packager --release -f app
APP="../target/packages/MultiDroid Preview.app"
codesign --force --options runtime --timestamp --sign "$ID" "$APP"
codesign --verify --strict --deep "$APP"
DMG=../target/packages/MultiDroid-Preview-$NOME-mac-arm64.dmg
WORK=$(mktemp -d)
trap 'hdiutil detach "$WORK/mnt" -force -quiet 2>/dev/null || true; rm -rf "$WORK"' EXIT
MB=$(( $(du -sm "$APP" | cut -f1) + 40 ))
hdiutil create -size "${MB}m" -fs APFS -volname "MultiDroid Preview" "$WORK/rw.dmg" >/dev/null 2>&1
mkdir "$WORK/mnt"
hdiutil attach "$WORK/rw.dmg" -mountpoint "$WORK/mnt" -nobrowse -quiet
ditto "$APP" "$WORK/mnt/MultiDroid Preview.app"
ln -s /Applications "$WORK/mnt/Applications"
for i in 1 2 3 4 5 6 7 8 9 10; do
  hdiutil detach "$WORK/mnt" -quiet 2>/dev/null && break
  [ $i = 10 ] && hdiutil detach "$WORK/mnt" -force -quiet
  sleep 2
done
hdiutil convert "$WORK/rw.dmg" -format ULFO -o "$DMG" >/dev/null
codesign --force --timestamp --sign "$ID" "$DMG"
if [ "${NOTARIZE:-1}" != 0 ]; then
  xcrun notarytool submit "$DMG" --keychain-profile "$PROFILE" --wait
  xcrun stapler staple "$DMG"
  spctl --assess --type open --context context:primary-signature -v "$DMG"
fi
if [ -n "$TAG" ]; then
  gh release upload "$TAG" "$DMG" --clobber -R iramarfalcao/MultiDroid
fi
echo "ok: $DMG"
