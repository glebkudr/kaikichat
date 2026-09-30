#!/bin/sh
# The owner CLI on Linux with a Secret Service answering: the profile secret
# goes to the keyring, no secrets file appears, and a restart reads it back.
set -u
export XDG_RUNTIME_DIR=$(mktemp -d /tmp/ain-xdg-XXXXXX); chmod 700 "$XDG_RUNTIME_DIR"
export HOME=$(mktemp -d /tmp/ain-home-XXXXXX)
printf 'keyring-test' | gnome-keyring-daemon --unlock --components=secrets >/dev/null
D=$(mktemp -d /tmp/ain-kr-XXXXXX); chmod 700 "$D"
A=/workspace/chat-arm64/target/debug/agentic
run() { env -u AGENTIC_SECRETS -u AGENTIC_PASSWORD -u AGENTIC_PASSWORD_FILE AGENTIC_DATA_DIR="$D" "$A" "$@"; echo " exit=$?"; }
echo "start:"; run daemon start --listen /ip4/127.0.0.1/tcp/0
echo "init:"; run init --name Linux
echo "stop:"; run daemon stop
echo "restart via contacts list:"; run contacts list
echo "status:"; run daemon status
echo "secrets.json present: $(test -e "$D/secrets.json" && echo yes || echo no)"
echo "keyring items for the desktop service: $(secret-tool search --all service net.agenticinternet.desktop 2>&1 | grep -c '^\[')"
echo "stop:"; run daemon stop
rm -rf "$D" "$HOME" "$XDG_RUNTIME_DIR"
