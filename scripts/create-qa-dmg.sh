#!/usr/bin/env bash
# Internal build verification only. Never used by the publication job.
set -euo pipefail
SOURCE_APP=${1:?App bundle required}
OUTPUT=${2:?QA disk image output required}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
APP="$WORK/$(basename "$SOURCE_APP")"
ditto "$SOURCE_APP" "$APP"
for attribute in com.apple.FinderInfo com.apple.ResourceFork; do
  xattr -dr "$attribute" "$APP" 2>/dev/null || true
done
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"
"${DMGBUILD_PYTHON:-python3}" -m dmgbuild -s "$ROOT/branding/dmg-settings.py" -D "app=$APP" -D "root=$ROOT" MultiDroid "$OUTPUT"
