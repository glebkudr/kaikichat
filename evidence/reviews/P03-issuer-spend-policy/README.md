# Issuer-bound canonical spend policy

CanonicalPostageIssuer permits paid issuance only after its deployer permanently
binds one nonzero P256 selection policy. The binding authenticates both policy ID
and issuer domain and preserves the base funding slots, resource rules and leaf
format. A chain-ID change cannot issue under the old binding. The Rust verifier
checks genuine account/storage proofs and returns an opaque supplied-root result;
the CLI explicitly returns admission:false.

All 33 contract tests passed, including four new cases. Two actual Anvil chains
exercised real registry policies, exact purchases, failed bindings, branch rollback,
replacement and process restart. With both chains stopped, 57 policy CLI checks,
four funding checks and two independently expected finalizer-selection checks
passed. The same issuer address on different chains produced different domains and
policies. Cleanup reported no errors. The public fixtures preserve the real proofs.

The final ordinary gates passed 503 Rust tests, 7 model tests, 9 independent oracle tests,
formatting, workspace Clippy, 40 frontend tests, TypeScript and Vite. This module
adds no UI or owner/MCP spend endpoint. Native UI and the full remaining EVM/Linux
network matrix are not rerun by these gates.

Tests were accepted by a separate reviewer before production; see review.md and
accepted-inputs.json. validation.json records commands, actual exits and log hashes;
source-inputs.json freezes implementation and verification inputs. evm.json and
fixtures.json preserve the actual two-chain proof outcomes. All 410 frozen source
inputs and 14 accepted test/evidence inputs stayed unchanged during final validation.

The first release compatibility check found a changed zkVM image. A new permanent
test received separate ACCEPT before the fix; see failed-image.json and review.md.
Excluding the host-only policy module from the guest restored the previous fixed
image ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de.
The rebuilt ordinary app's verifier accepted both earlier genuine receipts, with
exact context, nullifier, resources and independently encoded full journals. Wrong
operations and expiry were rejected. Verification used the receipts' historical
time; no current authority or new proof generation is claimed. The verifier binary
is byte-identical to the preceding accepted package. The app passed deep/strict
ad-hoc signature verification and excludes tauri-plugin-wdio-webdriver; it is not
notarized. compatibility.json records exact receipt and packaged binary hashes.

This is a prerequisite for canonical spending, not P03 or V1 completion. A supplied
root does not establish current finality. The next consumer must match the proven
binding to the installed, fully selected current committee, use one issuer-global
spent log, and preserve spent state across epoch changes. Consensus spend admission,
paid custody/repair, UI/MCP spending and the rest of V1 remain open.

Reproduce from the canonical source checkout:

```sh
python3 scripts/build-storage.py run bash -c 'forge test --root contracts --use "$AIN_SOLC"'
python3 scripts/build-storage.py run python3 tests/evm/issuer_spend_policy.py
python3 scripts/build-storage.py run node scripts/build-desktop.mjs
```
