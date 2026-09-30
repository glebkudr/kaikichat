#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
python3 scripts/build-storage.py --profile "${AIN_BUILD_STORAGE_PROFILE:-mac-apfs}" check
forge_bin="${AIN_FOUNDRY_BIN:+${AIN_FOUNDRY_BIN}/}forge"
solc_options=()
if [[ -n "${AIN_SOLC:-}" ]]; then
  solc_options=(--use "$AIN_SOLC")
fi
"$forge_bin" fmt --root contracts --check
"$forge_bin" test --root contracts "${solc_options[@]}"
