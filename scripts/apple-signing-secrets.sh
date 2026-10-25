#!/usr/bin/env bash
# Stores what GitHub needs to sign and notarize the macOS builds as secrets
# of the repository. Run it yourself in a terminal: passwords are typed here
# and go straight to GitHub, never into a file or a chat.
#
# 1. In Keychain Access, under My Certificates, right-click
#    "Developer ID Application: …", choose Export…, and save it as a .p12
#    file with a password of your choosing.
# 2. Create an app-specific password for notarization at
#    https://account.apple.com → Sign-In and Security → App-Specific Passwords.
# 3. Run: scripts/apple-signing-secrets.sh ~/Desktop/Certificates.p12
set -euo pipefail

repo="${REPO:-dobrinov/shape}"
p12="${1:-}"
if [ -z "$p12" ] || [ ! -f "$p12" ]; then
  sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
fi
command -v gh >/dev/null || { echo "Install the GitHub CLI (gh) and run gh auth login first."; exit 1; }

identity="$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -n 1)"
[ -n "$identity" ] || { echo "No Developer ID Application certificate in your keychain."; exit 1; }
team="$(printf %s "$identity" | sed -n 's/.*(\([A-Z0-9]*\))$/\1/p')"
echo "Signing as: $identity (team $team)"

read -rsp "Password of the .p12 file: " p12_password; echo
openssl pkcs12 -in "$p12" -passin "pass:$p12_password" -noout -legacy 2>/dev/null \
  || openssl pkcs12 -in "$p12" -passin "pass:$p12_password" -noout 2>/dev/null \
  || { echo "That password does not open the .p12 file."; exit 1; }
read -rp "Apple ID (email) for notarization: " apple_id
read -rsp "App-specific password: " app_password; echo
xcrun notarytool history --apple-id "$apple_id" --password "$app_password" --team-id "$team" >/dev/null 2>&1 \
  || { echo "Apple does not accept that Apple ID and app-specific password."; exit 1; }

base64 -i "$p12" | gh secret set MACOS_CERT_P12 -R "$repo"
printf %s "$p12_password" | gh secret set MACOS_CERT_PASSWORD -R "$repo"
printf %s "$apple_id" | gh secret set APPLE_ID -R "$repo"
printf %s "$app_password" | gh secret set APPLE_APP_PASSWORD -R "$repo"
# the same login in this Mac's keychain, to notarize a test build locally
xcrun notarytool store-credentials shape-notary --apple-id "$apple_id" --password "$app_password" --team-id "$team" >/dev/null

echo "Done: the next release of $repo is signed and notarized. You can delete $p12 now."
