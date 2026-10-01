#!/bin/bash
# Signs the command line's macOS binaries with the Developer ID before
# publish-cli.sh packs them, or checks that a build is signed so:
#   macos-sign-cli.sh DIR                 sign DIR's kaiki, kaiki-agentic-node, agentic-cli, agentic-mcp
#   macos-sign-cli.sh --check PATH...     check such a DIR, or an app bundle's executables
#   macos-sign-cli.sh --identifier NAME   print the identifier NAME is signed with
# Each binary gets the hardened runtime, a timestamp and a stable identifier
# from the table below; the linker's ad-hoc signature names it after the file
# and a build hash, so the Keychain would ask again after every update. The
# check refuses ad-hoc and unsigned code, an authority other than Developer ID
# Application, another identifier or team, and code without the runtime or a
# timestamp. APPLE_SIGNING_IDENTITY selects a Developer ID when more than one
# exists, as in macos-notarize.sh; the check needs none.
set -euo pipefail
usage() { echo "usage: $0 DIR | --check DIR|APP.app... | --identifier NAME" >&2; exit 2; }
cli=(kaiki kaiki-agentic-node agentic-cli agentic-mcp)
# The one table of identifiers, for macos-notarize.sh too. kaiki reads the
# profile key the app keeps in the Keychain; only code with the app's
# identifier and team reads it without the password dialog. The daemon's file
# was renamed kaiki-agentic-node for Activity Monitor; its identifier stays.
identifier() {
  case $1 in
    agentic-desktop|kaiki) echo net.agenticinternet.desktop ;;
    kaiki-agentic-node) echo net.agenticinternet.agentic-node ;;
    agentic-cli|agentic-mcp) echo "net.agenticinternet.$1" ;;
    *) return 1 ;;
  esac
}
failed=0 team=
refuse() { echo "refused $1: $2" >&2; failed=1; }
check_binary() { # PATH NAME
  local want info got
  want=$(identifier "$2") || { refuse "$1" "no identifier for $2 in $0"; return; }
  [ -f "$1" ] || { refuse "$1" "missing"; return; }
  info=$(codesign -dvv "$1" 2>&1) || { refuse "$1" "not signed"; return; }
  if grep -qx 'Signature=adhoc' <<<"$info"; then refuse "$1" "ad-hoc signature, not the Developer ID"; return; fi
  grep -q '^Authority=Developer ID Application: ' <<<"$info" || { refuse "$1" "no Developer ID Application authority"; return; }
  got=$(sed -n 's/^Identifier=//p' <<<"$info")
  [ "$got" = "$want" ] || { refuse "$1" "identifier $got, not $want"; return; }
  got=$(sed -n 's/^TeamIdentifier=//p' <<<"$info")
  grep -qx "Authority=Developer ID Application: .* ($got)" <<<"$info" || { refuse "$1" "team $got is not the authority's"; return; }
  [ "${team:=$got}" = "$got" ] || { refuse "$1" "team $got, not $team like the others"; return; }
  grep -q '^CodeDirectory .*flags=0x[0-9a-f]*([^)]*runtime' <<<"$info" || { refuse "$1" "no hardened runtime"; return; }
  grep -q '^Timestamp=' <<<"$info" || { refuse "$1" "no secure timestamp"; return; }
  codesign --verify --strict "$1" 2>/dev/null || refuse "$1" "codesign --verify --strict fails"
}
check() { # PATH...
  local path file name
  for path; do
    if [ -d "$path/Contents/MacOS" ]; then
      codesign --verify --deep --strict "$path" 2>/dev/null || refuse "$path" "codesign --verify --deep --strict fails"
      for file in "$path/Contents/MacOS/"*; do check_binary "$file" "$(basename "$file")"; done
    elif [ -d "$path" ]; then
      for name in "${cli[@]}"; do check_binary "$path/$name" "$name"; done
    else
      refuse "$path" "no such directory"
    fi
  done
  [ "$failed" -eq 0 ] || { echo "refusing: sign with the Developer ID first ($0 DIR, macos-notarize.sh)" >&2; exit 1; }
  echo "signed with the Developer ID, team $team: $*"
}
case ${1:-} in
  --identifier) [ $# -eq 2 ] || usage; identifier "$2" || { echo "no identifier for $2" >&2; exit 2; } ;;
  --check) shift; [ $# -ge 1 ] || usage; check "$@" ;;
  -*|'') usage ;;
  *)
    [ $# -eq 1 ] || usage
    identity=${APPLE_SIGNING_IDENTITY:-$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p')}
    if [[ "$identity" != "Developer ID Application:"* || "$identity" == *$'\n'* ]]; then
      echo "one Developer ID Application identity is required; select it with APPLE_SIGNING_IDENTITY" >&2
      exit 1
    fi
    for name in "${cli[@]}"; do
      [ -x "$1/$name" ] || { echo "missing executable: $1/$name" >&2; exit 1; }
    done
    for name in "${cli[@]}"; do
      codesign --force --options runtime --timestamp --identifier "$(identifier "$name")" --sign "$identity" "$1/$name"
    done
    check "$1" ;;
esac
