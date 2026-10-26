#!/usr/bin/env bash
# Shape.app (one universal binary for Apple silicon and Intel) in a DMG,
# signed, notarized and stapled:
#
#   scripts/macos-dmg.sh ARM64_BINARY X86_64_BINARY OUT.dmg
#
# SIGN_ID: the Developer ID identity (without it: signed ad hoc, not
# notarized). Notarized with APPLE_ID, APPLE_APP_PASSWORD and SIGN_TEAM, or
# with NOTARY_PROFILE (a keychain profile: `xcrun notarytool store-credentials`).
set -euo pipefail
arm="$1" intel="$2" out="$3"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

lipo -create "$arm" "$intel" -output "$work/shape"
chmod 755 "$work/shape"
mkdir "$work/dmg"
"$work/shape" make-app "$work/dmg/Shape.app" --binary "$work/shape"
ln -s /Applications "$work/dmg/Applications"

sign() { codesign --force --options runtime --timestamp --sign "$SIGN_ID" "$@"; }
if [ -n "${SIGN_ID:-}" ]; then
  sign "$work/dmg/Shape.app"
else
  codesign --force --sign - "$work/dmg/Shape.app"
fi
codesign --verify --strict --deep --verbose=2 "$work/dmg/Shape.app"

rm -f "$out"
hdiutil create -volname Shape -srcfolder "$work/dmg" -fs HFS+ -format UDZO -ov "$out" >/dev/null
[ -n "${SIGN_ID:-}" ] || { echo "made $out (not signed: no SIGN_ID)"; exit 0; }
sign "$out"

if [ -n "${NOTARY_PROFILE:-}" ]; then
  auth=(--keychain-profile "$NOTARY_PROFILE")
else
  auth=(--apple-id "$APPLE_ID" --password "$APPLE_APP_PASSWORD" --team-id "$SIGN_TEAM")
fi
xcrun notarytool submit "$out" "${auth[@]}" --wait --timeout 30m --output-format json | tee "$work/notary.json"
echo
grep -q '"status":"Accepted"' "$work/notary.json" || { echo "not notarized"; exit 1; }
# the ticket goes in the DMG, so it opens without asking Apple
xcrun stapler staple "$out"
spctl -a -vv -t open --context context:primary-signature "$out"
echo "made $out"
