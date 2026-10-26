#!/usr/bin/env bash
# CI: the Developer ID certificate from MACOS_CERT_P12 (base64) and
# MACOS_CERT_PASSWORD, in a keychain of its own for this run. Writes
# SIGN_ID (the identity's hash), SIGN_TEAM and SIGN_KEYCHAIN to
# $RUNNER_TEMP/sign.env, to source.
set -euo pipefail
kc="$RUNNER_TEMP/sign.keychain-db"
kpw=$(openssl rand -hex 16)
echo "$MACOS_CERT_P12" | base64 --decode > "$RUNNER_TEMP/cert.p12"
security create-keychain -p "$kpw" "$kc"
security set-keychain-settings -lut 21600 "$kc"
security unlock-keychain -p "$kpw" "$kc"
security import "$RUNNER_TEMP/cert.p12" -k "$kc" -P "$MACOS_CERT_PASSWORD" -T /usr/bin/codesign
rm "$RUNNER_TEMP/cert.p12"
security set-key-partition-list -S apple-tool:,apple: -s -k "$kpw" "$kc" >/dev/null
security list-keychains -d user -s "$kc" $(security list-keychains -d user | tr -d '"')
line=$(security find-identity -v -p codesigning "$kc" | grep -m1 'Developer ID Application' || true)
[ -n "$line" ] || { echo "MACOS_CERT_P12 holds no Developer ID Application certificate"; exit 1; }
{
  echo "export SIGN_ID=$(echo "$line" | awk '{print $2}')"
  echo "export SIGN_TEAM=$(echo "$line" | sed -E 's/.*\(([A-Z0-9]+)\)"$/\1/')"
  echo "export SIGN_KEYCHAIN='$kc'"
} > "$RUNNER_TEMP/sign.env"
