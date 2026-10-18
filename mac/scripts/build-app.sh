#!/usr/bin/env bash
# Monta build/MultiDroid.app: executável do SwiftPM + Info.plist + ícone do Icon Composer
# (docs/design/icone/AppIcon.icon, compilado pelo actool para macOS).
#   scripts/build-app.sh [debug|release]
# Assina com o Developer ID e o hardened runtime quando o certificado está no
# Keychain (exigência da notarização); senão, ad hoc. SIGN_ID força outra
# identidade; SIGN_ID=- força ad hoc.
set -euo pipefail
cd "$(dirname "$0")/.."
export DEVELOPER_DIR=${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}
CONF=${1:-release}
swift build -c "$CONF"
APP=build/MultiDroid.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp ".build/$CONF/MultiDroid" "$APP/Contents/MacOS/MultiDroid"
xcrun actool ../docs/design/icone/AppIcon.icon --compile "$APP/Contents/Resources" \
  --platform macosx --minimum-deployment-target 26.0 --app-icon AppIcon \
  --output-partial-info-plist build/icon-info.plist >/dev/null
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>MultiDroid</string>
  <key>CFBundleDisplayName</key><string>MultiDroid</string>
  <key>CFBundleIdentifier</key><string>com.falcaosl.multidroid</string>
  <key>CFBundleExecutable</key><string>MultiDroid</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.2</string>
  <key>CFBundleVersion</key><string>2</string>
  <key>CFBundleIconName</key><string>AppIcon</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>LSMinimumSystemVersion</key><string>26.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
</dict>
</plist>
PLIST
SIGN_ID=${SIGN_ID:-$(security find-identity -v -p codesigning 2>/dev/null \
  | awk -F'"' '/Developer ID Application/{print $2; exit}')}
if [ -n "$SIGN_ID" ] && [ "$SIGN_ID" != "-" ]; then
  codesign --force --options runtime --timestamp --sign "$SIGN_ID" "$APP"
  echo "ok: $APP (assinado: $SIGN_ID)"
else
  codesign --force --sign - "$APP" >/dev/null 2>&1
  echo "ok: $APP (ad hoc: só abre neste Mac)"
fi
