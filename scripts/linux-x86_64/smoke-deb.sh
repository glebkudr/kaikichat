#!/bin/bash
# Production smoke of the Linux x86_64 package in a clean ubuntu:24.04
# container (no build tools): install the .deb, start the app under Xvfb
# with a password-sealed profile, and use the same profile from the
# packaged CLI while the window is open.
#   smoke-deb.sh PACKAGE.deb OUTPUT_DIR
set -euo pipefail
deb=$(realpath "$1"); out=$(realpath "$2"); mkdir -p "$out"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends xvfb xauth imagemagick fonts-dejavu-core "$deb" >/dev/null
dpkg -s "$(dpkg-deb -f "$deb" Package)" | grep -E '^(Package|Version|Architecture|Depends):' > "$out/package.txt"
dpkg -L "$(dpkg-deb -f "$deb" Package)" > "$out/files.txt"
Xvfb :99 -screen 0 1400x900x24 >/dev/null 2>&1 &
export DISPLAY=:99
home=$(mktemp -d)
export HOME=$home AGENTIC_DATA_DIR=$home/profile AGENTIC_SECRETS=file
printf 'smoke password\n' > "$home/password"; chmod 600 "$home/password"
export AGENTIC_PASSWORD_FILE=$home/password
cli() { kaiki "$@"; }
agentic-desktop > "$out/app.log" 2>&1 &
app=$!
for _ in $(seq 120); do [ -S "$AGENTIC_DATA_DIR/node.sock" ] && break; sleep 1; done
[ -S "$AGENTIC_DATA_DIR/node.sock" ] || { echo "the window did not start the daemon"; cat "$out/app.log"; exit 1; }
sleep 8
import -window root "$out/linux-release-onboarding.png"
cli daemon status | tee "$out/status-before.json"
cli network | tee "$out/network.json"
grep -q '"state":"current"' "$out/network.json" || { echo "the app did not load its signed network preset" >&2; exit 1; }
cli init --name "Linux smoke" | tee "$out/init.json"
sleep 5
import -window root "$out/linux-release-after-cli-init.png"
cli contacts policy | tee "$out/policy.json"
test -f "$AGENTIC_DATA_DIR/secrets.json"
stat -c '%a %n' "$AGENTIC_DATA_DIR" "$AGENTIC_DATA_DIR/secrets.json" | tee "$out/modes.txt"
kill "$app"; wait "$app" || true
cli daemon status | tee "$out/status-after-window.json"
cli daemon stop | tee "$out/stop.json"
echo "smoke passed"
