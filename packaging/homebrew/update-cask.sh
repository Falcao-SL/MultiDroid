#!/usr/bin/env bash
# Atualiza version e sha256 do cask a partir de uma GitHub Release.
# Uso: packaging/homebrew/update-cask.sh [tag]
#   tag padrão: v + CFBundleShortVersionString de mac/scripts/build-app.sh
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
CASK=$ROOT/packaging/homebrew/Casks/multidroid.rb
SWIFT=$(sed -n 's/.*CFBundleShortVersionString<\/key><string>\([^<]*\)<\/string>.*/\1/p' \
  "$ROOT/mac/scripts/build-app.sh" | head -1)
TAG=${1:-v$SWIFT}
VER=${TAG#v}
REPO=${GITHUB_REPOSITORY:-iramarfalcao/MultiDroid}
ASSET="MultiDroid-${VER}.dmg"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
echo "baixando $URL"
curl -fsSL "$URL" -o "$TMP"
SUM=$(shasum -a 256 "$TMP" | awk '{print $1}')

perl -i -pe "s/^  version \".*\"/  version \"$VER\"/" "$CASK"
perl -i -pe "s/^  sha256 \".*\"/  sha256 \"$SUM\"/" "$CASK"
echo "ok: $CASK → version $VER sha256 $SUM"
echo "Próximo: copiar para o tap (ex. homebrew-falcaosl) e abrir PR, ou:"
echo "  brew install --cask $CASK"
