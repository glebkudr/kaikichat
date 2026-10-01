#!/bin/bash
# Sign the macos-release.sh app, notarize it, staple its ticket, then make
# the ZIP for GitHub Releases. Run through build-storage.py, like the build:
#   macos-notarize.sh "target/release/bundle/macos/Kaiki Chat.app" output/release [PROFILE]
# PROFILE defaults to kaiki-notary (notarytool store-credentials).
# APPLE_SIGNING_IDENTITY selects a Developer ID when more than one exists.
set -euo pipefail
[ $# -ge 2 ] && [ $# -le 3 ] || { echo "usage: $0 APP.app NEW_OUTPUT_DIR [NOTARY_PROFILE]" >&2; exit 2; }
app=$1 out=$2 profile=${3:-kaiki-notary}
[ -d "$app/Contents/MacOS" ] || { echo "$app is not an app bundle" >&2; exit 2; }
[ ! -e "$out" ] || { echo "use a new output directory: $out" >&2; exit 2; }
identity=${APPLE_SIGNING_IDENTITY:-$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p')}
if [[ "$identity" != "Developer ID Application:"* || "$identity" == *$'\n'* ]]; then
  echo "one Developer ID Application identity is required; select it with APPLE_SIGNING_IDENTITY" >&2
  exit 1
fi
xcrun notarytool history --keychain-profile "$profile" >/dev/null
for name in agentic-desktop kaiki-agentic-node agentic-cli agentic-mcp kaiki; do
  [ -x "$app/Contents/MacOS/$name" ] || { echo "missing executable: $name" >&2; exit 1; }
done
mkdir -p "$out"
# Kaiki's native code is these five executables; sign inside-out, without
# --deep, with the identifiers of macos-sign-cli.sh (kaiki shares the app's).
for name in agentic-desktop kaiki-agentic-node agentic-cli agentic-mcp kaiki; do
  codesign --force --options runtime --timestamp --identifier "$(bash "$(dirname "$0")/macos-sign-cli.sh" --identifier "$name")" \
    --sign "$identity" "$app/Contents/MacOS/$name"
done
codesign --force --options runtime --timestamp --sign "$identity" "$app"
codesign --verify --deep --strict --verbose=2 "$app"
codesign -dv --verbose=4 "$app" 2> "$out/signature.txt"
ditto -c -k --keepParent "$app" "$out/notarization.zip"
xcrun notarytool submit "$out/notarization.zip" --keychain-profile "$profile" --wait --output-format json > "$out/submission.json"
status=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["status"])' "$out/submission.json")
if [ "$status" != Accepted ]; then
  id=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["id"])' "$out/submission.json")
  xcrun notarytool log "$id" --keychain-profile "$profile" "$out/notary-log.json" || true
  echo "notarization $status; see $out/submission.json and $out/notary-log.json" >&2
  exit 1
fi
xcrun stapler staple "$app"
xcrun stapler validate "$app"
codesign --verify --deep --strict --verbose=2 "$app"
spctl --assess --type execute --verbose=4 "$app"
# ZIP files cannot hold a ticket: repackage the app after stapling.
ditto -c -k --keepParent "$app" "$out/kaiki-chat-macos-arm64.zip"
(cd "$out" && shasum -a 256 kaiki-chat-macos-arm64.zip > kaiki-chat-macos-arm64.zip.sha256)
echo "signed, notarized release: $out/kaiki-chat-macos-arm64.zip"
