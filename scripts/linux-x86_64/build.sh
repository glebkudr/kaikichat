#!/bin/bash
# Builds the desktop app for Linux x86_64 inside the ain-v1-amd64-base image
# (scripts/linux-x86_64/Dockerfile), from the root of a Linux checkout:
#   build.sh release   the .deb (target/release/bundle/deb)
#   build.sh e2e       the debug build with the automation driver, for tests
set -euo pipefail
cd "$(dirname "$0")/../.."
mode=${1:-release}
storage() { python3 scripts/build-storage.py --profile portable-linux "$@"; }
storage setup
(cd apps/desktop && [ -d node_modules ] || npm ci --no-audit --no-fund)
storage run sh -c 'cd apps/desktop && node node_modules/typescript/bin/tsc --noEmit && node node_modules/vite/bin/vite.js build'
target=$(rustc --print host-tuple)
if [ "$mode" = release ]; then
  storage run cargo build --locked --release -p agentic-node --bins
  mkdir -p apps/desktop/src-tauri/binaries
  for name in agentic-node agentic-mcp agentic-cli kaiki; do
    cp "target/release/$name" "apps/desktop/src-tauri/binaries/$name-$target"
  done
  storage run sh -c 'cd apps/desktop && node node_modules/@tauri-apps/cli/tauri.js build --ci --bundles deb --config src-tauri/tauri.bundle.conf.json -- --locked'
  ls -la target/release/bundle/deb/
else
  storage run cargo build --locked -p agentic-node --bins
  storage run cargo build --locked -p agentic-desktop --features e2e
fi
