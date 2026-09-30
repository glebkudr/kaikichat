#!/bin/bash
# Which signatures read a keychain item another binary created, without a
# dialog: `run-probe.sh NEW_WORK_DIR`. Builds probe.swift twice (different
# code), signs the copies like the release does, and runs every scenario on a
# new keychain file in WORK_DIR with interaction off. The login keychain is not
# read or written; the user's keychain search list is restored at once.
set -euo pipefail
work=$1
[ ! -e "$work" ] || { echo "use a new directory: $work" >&2; exit 2; }
here=$(cd "$(dirname "$0")" && pwd)
identity=${APPLE_SIGNING_IDENTITY:-$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p')}
mkdir -p "$work"
xcrun swiftc -O -suppress-warnings -o "$work/probe-a" "$here/probe.swift"
xcrun swiftc -O -suppress-warnings -D VARIANT_B -o "$work/probe-b" "$here/probe.swift"
sign() { # sign SRC NAME IDENTIFIER|adhoc
  mkdir -p "$work/$2"
  cp "$work/$1" "$work/$2/bin"
  if [ "$3" = adhoc ]; then
    codesign --force --sign - "$work/$2/bin"
  else
    perl -e 'alarm 120; exec @ARGV' codesign --force --options runtime --timestamp=none \
      --identifier "$3" --sign "$identity" "$work/$2/bin"
  fi
}
sign probe-a app net.agenticinternet.desktop            # the app's main binary
sign probe-b cli-app-id net.agenticinternet.desktop     # kaiki signed with the app's identifier
sign probe-b cli-kaiki kaiki                            # kaiki as the 0.2.1 bundle signs it
sign probe-b cli-named net.agenticinternet.kaiki        # kaiki with its own stable identifier
sign probe-b cli-adhoc adhoc                            # kaiki as install.sh shipped it
keychain="$work/probe.keychain-db"
password=probe-$RANDOM$RANDOM
before=$(security list-keychains -d user | tr -d '"' | xargs)
security create-keychain -p "$password" "$keychain"
# shellcheck disable=SC2086
security list-keychains -d user -s $before
after=$(security list-keychains -d user | tr -d '"' | xargs)
[ "$before" = "$after" ] || { echo "keychain search list changed: $after" >&2; exit 1; }
run() { printf '%-11s %-5s %-10s -> %s\n' "$1" "$2" "$3" "$("$work/$1/bin" "$2" "$keychain" "$password" "$3" ${4:-})"; }
for who in app cli-app-id cli-kaiki cli-named cli-adhoc; do
  codesign -dvv "$work/$who/bin" 2>&1 | sed -n "s/^Identifier=/$who identifier=/p; s/^Authority=Developer ID Application.*/$who authority=Developer ID/p; s/^Signature=adhoc/$who authority=adhoc/p; s/^CDHash=/$who cdhash=/p"
done
echo "--- the app creates the key, the command line reads it"
run app write app-first secret-1
for who in cli-app-id cli-kaiki cli-named cli-adhoc; do run "$who" read app-first; done
echo "--- the command line creates the key, the app reads it"
for who in cli-app-id cli-kaiki cli-named cli-adhoc; do
  run "$who" write "$who-first" "secret-$who"
  run app read "$who-first"
done
echo "--- the creator itself"
run app read app-first
