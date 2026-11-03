#!/usr/bin/env bash
# Production-only: Developer ID, notarization and Gatekeeper must all pass.
set -euo pipefail
APP=${1:?Usage: sign-apple-release.sh /path/App.app /path/output.dmg}
DMG=${2:?Output DMG required}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TEAM=${APPLE_TEAM_ID:-6EZ89E2F5S}
if [[ -z "${APPLE_NOTARY_PROFILE:-}" ]]; then
  : "${APPLE_ID:?Configure APPLE_ID in GitHub Actions secrets}"
  : "${APPLE_APP_PASSWORD:?Configure APPLE_APP_PASSWORD in GitHub Actions secrets}"
fi
ID=$(security find-identity -v -p codesigning | awk -F'"' -v team="$TEAM" '/Developer ID Application/ && index($2,"(" team ")") {print $2; exit}')
[ -n "$ID" ] || { echo 'Developer ID Application for the configured team is missing from the runner keychain.' >&2; exit 1; }
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
# Sign an isolated bundle so sync/file-provider metadata cannot mutate it.
ditto "$APP" "$WORK/$(basename "$APP")"
APP="$WORK/$(basename "$APP")"
# Finder metadata from file providers is not allowed in signed build products.
for attribute in com.apple.FinderInfo com.apple.ResourceFork; do
  xattr -dr "$attribute" "$APP" 2>/dev/null || true
done
# Sign nested native code first.
while IFS= read -r -d '' file; do
  if file -b "$file" | grep -q 'Mach-O'; then
    codesign --force --options runtime --timestamp --sign "$ID" "$file"
  fi
done < <(find "$APP/Contents" -type f -print0)
codesign --force --options runtime --timestamp --sign "$ID" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
submit() {
  if [[ -n "${APPLE_NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$1" --keychain-profile "$APPLE_NOTARY_PROFILE" --wait --output-format json > "$WORK/notary.json"
  else
    xcrun notarytool submit "$1" --apple-id "$APPLE_ID" --team-id "$TEAM" \
      --password "$APPLE_APP_PASSWORD" --wait --output-format json > "$WORK/notary.json"
  fi
  python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); print("Notarization:",r.get("status")); sys.exit(0 if r.get("status")=="Accepted" else 1)' "$WORK/notary.json"
}
# This ZIP is temporary Apple transport only; it is never a release/download asset.
ditto -c -k --keepParent "$APP" "$WORK/notarization.zip"
submit "$WORK/notarization.zip"
xcrun stapler staple "$APP"
xcrun stapler validate "$APP"
spctl --assess --type execute --verbose=2 "$APP"
mkdir -p "$(dirname "$DMG")"
"${DMGBUILD_PYTHON:-python3}" -m dmgbuild -s "$ROOT/branding/dmg-settings.py" -D "app=$APP" -D "root=$ROOT" MultiDroid "$DMG"
codesign --force --timestamp --sign "$ID" "$DMG"
submit "$DMG"
xcrun stapler staple "$DMG"
xcrun stapler validate "$DMG"
codesign --verify --strict --verbose=2 "$DMG"
spctl --assess --type open --context context:primary-signature --verbose=2 "$DMG"
echo 'Production DMG verified: Developer ID, stapled app, stapled DMG and Gatekeeper.'
