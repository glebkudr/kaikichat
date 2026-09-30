#!/bin/bash
# The desktop app's native chat case on Linux WebKitGTK, inside the
# ain-v1-amd64-base image after `build.sh e2e`: two windows under Xvfb with
# the isolated E2E vault, the same runner as on macOS.
set -euo pipefail
cd "$(dirname "$0")/../.."
Xvfb :99 -screen 0 1400x900x24 >/dev/null 2>&1 &
export DISPLAY=:99 WEBKIT_DISABLE_COMPOSITING_MODE=1 LIBGL_ALWAYS_SOFTWARE=1
sleep 1
exec dbus-run-session -- node apps/desktop/tests/native-e2e.mjs --case chat
