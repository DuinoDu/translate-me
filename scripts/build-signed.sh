#!/usr/bin/env bash
#
# Build a signed + notarized translate-me.app / .dmg.
#
# Prerequisites (see SIGNING.md):
#   1. A "Developer ID Application" certificate installed in your login keychain
#      (requires a paid Apple Developer Program membership).
#   2. Notarization credentials — either an App Store Connect API key (.p8)
#      or an Apple ID + app-specific password.
#
# Put your values in .env.signing (gitignored). This script loads them, then
# Tauri signs with APPLE_SIGNING_IDENTITY and notarizes when the notary
# credentials are present.
#
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -f .env.signing ]; then
  set -a
  # shellcheck disable=SC1091
  source .env.signing
  set +a
fi

: "${APPLE_SIGNING_IDENTITY:?Set APPLE_SIGNING_IDENTITY (e.g. 'Developer ID Application: NAME (TEAMID)') in .env.signing}"
if [[ "$APPLE_SIGNING_IDENTITY" != Developer\ ID\ Application:* ]]; then
  echo "APPLE_SIGNING_IDENTITY must be a Developer ID Application certificate for outside-App-Store distribution." >&2
  exit 1
fi

echo "==> Signing identity: $APPLE_SIGNING_IDENTITY"
if [ -n "${APPLE_API_ISSUER:-}" ] && [ -n "${APPLE_API_KEY:-}" ] && [ -n "${APPLE_API_KEY_PATH:-}" ]; then
  echo "==> Notarizing via App Store Connect API key"
elif [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
  echo "==> Notarizing via Apple ID + app-specific password"
else
  echo "Set notarization credentials in .env.signing: either APPLE_API_ISSUER/APPLE_API_KEY/APPLE_API_KEY_PATH or APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID." >&2
  exit 1
fi

export CI="${CI:-true}"
npm run tauri -- build --bundles dmg

APP="src-tauri/target/release/bundle/macos/translate-me.app"
DMG="$(ls -t src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null | head -1 || true)"
if [ ! -d "$APP" ]; then
  echo "Missing app bundle: $APP" >&2
  exit 1
fi
if [ -z "$DMG" ] || [ ! -f "$DMG" ]; then
  echo "Missing DMG bundle under src-tauri/target/release/bundle/dmg" >&2
  exit 1
fi

echo
echo "==> codesign verification"
codesign --verify --deep --strict --verbose=2 "$APP"
echo "==> Stapler validation"
xcrun stapler validate "$APP"
xcrun stapler validate "$DMG"
echo "==> Gatekeeper assessment (should say: accepted, source=Notarized Developer ID)"
spctl -a -vvv -t execute "$APP"
spctl -a -vvv -t install "$DMG"
echo "==> DMG: $DMG"
