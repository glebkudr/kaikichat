#!/bin/bash
# The macOS release app bundle from a verification worktree, run through
# `build-storage.py --worktree W run bash scripts/macos-release.sh`: the same
# stages as scripts/build-desktop.mjs, without its nested storage check,
# which only the main checkout passes.
set -euo pipefail
cd "$(dirname "$0")/.."
(cd apps/desktop && node node_modules/typescript/bin/tsc --noEmit && node node_modules/vite/bin/vite.js build)
cargo build --locked --release -p agentic-node --bins
target=$(rustc --print host-tuple)
mkdir -p apps/desktop/src-tauri/binaries
for name in kaiki-agentic-node agentic-mcp agentic-cli kaiki; do
  cp "target/release/$name" "apps/desktop/src-tauri/binaries/$name-$target"
done
(cd apps/desktop && node node_modules/@tauri-apps/cli/tauri.js build --ci --bundles app --config src-tauri/tauri.bundle.conf.json -- --locked)
codesign --verify --deep --strict "target/release/bundle/macos/Kaiki Chat.app"
if cargo tree -p agentic-desktop -e normal --prefix none | grep -q tauri-plugin-wdio-webdriver; then
  echo "the automation plugin leaked into the release build" >&2; exit 1
fi
echo "release app: target/release/bundle/macos/Kaiki Chat.app"
