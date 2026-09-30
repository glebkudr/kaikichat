#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python3 scripts/build-storage.py --profile "${AIN_BUILD_STORAGE_PROFILE:-mac-apfs}" check
node_bin="${AIN_NODE:-node}"
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
scripts/check-evm.sh
python3 -m unittest discover -s tests/site -p 'test_*.py'
python3 -m unittest discover -s tests/welcome -p 'test_*.py'
cd apps/desktop
"$node_bin" node_modules/vitest/vitest.mjs run
"$node_bin" node_modules/typescript/bin/tsc --noEmit
"$node_bin" node_modules/vite/bin/vite.js build
