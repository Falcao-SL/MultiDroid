#!/usr/bin/env bash
# Local entry point uses the same branded, notarized production pipeline as CI.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT/mac"
SKIP_SIGNING=1 scripts/build-app.sh release
VERSION=$(cat VERSION)
if [[ -z "${APPLE_ID:-}" && -z "${APPLE_NOTARY_PROFILE:-}" ]]; then
  export APPLE_NOTARY_PROFILE=${NOTARY_PROFILE:-notary}
fi
bash "$ROOT/scripts/sign-apple-release.sh" "$PWD/build/MultiDroid.app" "$PWD/build/MultiDroid-$VERSION.dmg"
