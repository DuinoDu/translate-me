# Signing & Notarization (macOS)

To distribute `translate-me` without the Gatekeeper warning, the `.app` must be
signed with a **Developer ID Application** certificate and **notarized** by Apple.

## Prerequisites (one-time)

1. **Apple Developer Program membership** (paid, $99/yr) — required for a
   Developer ID certificate. The `Apple Development` certs already on this Mac
   (472365351@qq.com) are **revoked** and cannot be used for distribution.

2. **Developer ID Application certificate**
   - Xcode → Settings → Accounts → your team → *Manage Certificates* → `+`
     → **Developer ID Application**, or create it at
     <https://developer.apple.com/account/resources/certificates>.
   - Confirm it is installed:
     ```bash
     security find-identity -v -p codesigning
     # should list: "Developer ID Application: NAME (TEAMID)"
     ```

3. **Notarization credentials** — choose one:
   - **App Store Connect API key** (recommended): create a key at
     <https://appstoreconnect.apple.com/access/integrations/api>, download the
     `.p8`, note the **Key ID** and **Issuer ID**.
   - **Apple ID + app-specific password**: generate an app-specific password at
     <https://account.apple.com> → Sign-In & Security, plus your **Team ID**.

## Build

```bash
cp .env.signing.example .env.signing   # then fill in your values
./scripts/build-signed.sh
```

The script loads `.env.signing`, runs `npm run tauri build` (Tauri signs with
`APPLE_SIGNING_IDENTITY` and notarizes when notary credentials are present),
then verifies signing, Gatekeeper, and stapling.

A successful result shows:

```
spctl ... : accepted
source=Notarized Developer ID
```

## Notes

- `.env.signing` and `*.p8` keys are gitignored — never commit them.
- First notarization of a new app can take a few minutes.
- CI: set the same variables as secrets and base64-encode the cert via
  `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` (see Tauri docs).
