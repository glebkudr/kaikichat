#!/bin/bash
# Production smoke of the macOS release app: it starts with a temporary
# password-sealed profile (no login Keychain), starts the profile's daemon,
# and the bundled CLI uses the same profile while the window is open.
#   macos-smoke.sh "APP.app" OUTPUT_DIR
set -euo pipefail
app=$1; out=$(mkdir -p "$2" && cd "$2" && pwd)
bin="$app/Contents/MacOS"
# Keep the disposable profile inside the checkout, but short enough for IPC.
work=$(mktemp -d "$(pwd -P)/.smoke-XXXX"); chmod 700 "$work"
export AGENTIC_DATA_DIR=$work/p AGENTIC_SECRETS=file
printf 'smoke password\n' > "$work/password"; chmod 600 "$work/password"
export AGENTIC_PASSWORD_FILE=$work/password
trap '[ -z "${app_pid:-}" ] || { kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; }; "$bin/kaiki" daemon stop >/dev/null 2>&1 || true; rm -rf "$work"' EXIT
ls "$bin" > "$out/bundle-binaries.txt"
"$bin/kaiki" --version > "$out/cli-version.txt"
"$bin/agentic-desktop" > "$out/app.log" 2>&1 &
app_pid=$!
for _ in $(seq 60); do [ -S "$AGENTIC_DATA_DIR/node.sock" ] && break; sleep 1; done
[ -S "$AGENTIC_DATA_DIR/node.sock" ] || { echo "the window did not start the daemon"; cat "$out/app.log"; exit 1; }
"$bin/kaiki" daemon status | tee "$out/status-before.json"
"$bin/kaiki" network | tee "$out/network.json"
grep -q '"state":"current"' "$out/network.json" || { echo "the app did not load its signed network preset" >&2; exit 1; }
"$bin/kaiki" init --name "macOS smoke" | tee "$out/init.json"
"$bin/kaiki" contacts policy | tee "$out/policy.json"
stat -f '%Lp %N' "$AGENTIC_DATA_DIR" "$AGENTIC_DATA_DIR/secrets.json" | sed "s#$work#WORK#" | tee "$out/modes.txt"
sleep 3
kill -0 "$app_pid"
kill "$app_pid"; wait "$app_pid" 2>/dev/null || true
app_pid=
"$bin/kaiki" daemon status | tee "$out/status-after-window.json"
"$bin/kaiki" daemon stop | tee "$out/stop.json"
echo "smoke passed"
