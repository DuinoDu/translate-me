#!/usr/bin/env bash
#
# Build and install a local development copy of translate-me.
#
# This intentionally uses an ad-hoc signature with a stable designated
# requirement, so local rebuilds keep the same bundle identity.
set -euo pipefail
cd "$(dirname "$0")/.."

APP_NAME="translate-me"
BUNDLE_ID="com.duino.translateme"
BUILT_APP="src-tauri/target/release/bundle/macos/${APP_NAME}.app"
INSTALLED_APP="/Applications/${APP_NAME}.app"

npm run tauri -- build --bundles app

if [[ ! -d "$BUILT_APP" ]]; then
  echo "Missing app bundle: $BUILT_APP" >&2
  exit 1
fi

osascript -e "quit app \"${APP_NAME}\"" >/dev/null 2>&1 || true
sleep 1
pkill -x "$APP_NAME" >/dev/null 2>&1 || true

rm -rf "$INSTALLED_APP"
ditto "$BUILT_APP" "$INSTALLED_APP"
xattr -dr com.apple.quarantine "$INSTALLED_APP" >/dev/null 2>&1 || true
codesign --force --deep --sign - \
  --requirements "=designated => identifier \"${BUNDLE_ID}\"" \
  "$INSTALLED_APP"
codesign --verify --deep --strict --verbose=2 "$INSTALLED_APP"

open -a "$INSTALLED_APP"
echo "Installed and launched: $INSTALLED_APP"
