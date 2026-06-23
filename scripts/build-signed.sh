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

echo "==> Signing identity: $APPLE_SIGNING_IDENTITY"
if [ -n "${APPLE_API_KEY_PATH:-}" ]; then
  echo "==> Notarizing via App Store Connect API key"
elif [ -n "${APPLE_ID:-}" ]; then
  echo "==> Notarizing via Apple ID + app-specific password"
else
  echo "!! No notary credentials found — the build will be SIGNED but NOT notarized."
fi

npm run tauri build

APP="src-tauri/target/release/bundle/macos/translate-me.app"
DMG="$(ls -t src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null | head -1 || true)"

echo
echo "==> codesign verification"
codesign --verify --deep --strict --verbose=2 "$APP" || true
echo "==> Gatekeeper assessment (should say: accepted, source=Notarized Developer ID)"
spctl -a -vvv -t install "$APP" || true
if [ -n "$DMG" ]; then
  echo "==> Stapler validation"
  xcrun stapler validate "$DMG" || true
  echo "==> DMG: $DMG"
fi
