#!/usr/bin/env bash
# Local preview packaging shares the same icon, DMG and signing checks as CI.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TAG=${1:-}
if [[ "${DO_CI:-0}" == 1 ]]; then
  echo 'Generate a fresh signed release through release.yml; existing published installers are never replaced.' >&2
  exit 1
fi
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/rust/Cargo.toml" | head -1)
NAME=${TAG#v}; NAME=${NAME:-$VERSION}
cd "$ROOT/rust/app"
cargo packager --release -f app
if [[ -z "${APPLE_ID:-}" && -z "${APPLE_NOTARY_PROFILE:-}" ]]; then
  export APPLE_NOTARY_PROFILE=${NOTARY_PROFILE:-notary}
fi
DMG="$ROOT/rust/target/packages/MultiDroid-Preview-$NAME-mac-arm64.dmg"
bash "$ROOT/scripts/sign-apple-release.sh" "$ROOT/rust/target/packages/MultiDroid Preview.app" "$DMG"
if [[ -n "$TAG" ]]; then
  gh release upload "$TAG" "$DMG" -R Falcao-SL/MultiDroid
fi
