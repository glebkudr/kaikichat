# AR2 — default legacy verifier build passed

This slice addresses the build part of review AR-R11. It does not close AR2,
legacy historical retrieval, UI launch or three-platform acceptance.

The original verifier/prover share one frozen image ID:
`ffe1fd205bb628ad689a63e80d765ff7ac68ac71be19f510f77fc491dceed6de`.
Default builds use the frozen ID without invoking the guest builder; explicit
proving still compiles the real guest and must match that identity at compile time.
No guest, receipt format, funding namespace or receipt validation rule changes.

## Tests first

- `tests/build/legacy_verifier.py` was accepted by independent
  `/root/legacy_verifier_test_critic` before Cargo/build.rs/lib.rs changes.
  The actual RED found guest/kernel build dependencies in the default workspace.
- The clean GREEN uses a new target on the managed external volume, forbids guest
  generation with both skip flags, and checks the two original paid receipts
  through the existing Rust compatibility test. Default and explicit-prove
  dependency graphs are checked separately.
- The three original evidence files were compared byte-for-byte with committed
  `a1ebaf9`; see `original-baseline.json`. They were not replaced or re-proved.
- `tests/build/desktop_verifier.py` separately exercises actual macOS packaging.
  Its actual RED reaches the old unconditional prover command and fails because
  the real guest build is forbidden. Review initially required a negative for a
  proving-capable worker mistakenly packaged under the verifier name. The revised
  test checks both proving commands with stdin left open and no private input.
  The revised tests received final ACCEPT before packaging changes. The actual
  macOS debug bundle now passes the complete gate (`desktop-check.json`).

## Reproduce

Run from `/Users/glebk/Code/chat` with managed build storage:

```sh
python3 scripts/build-storage.py run python3 tests/build/legacy_verifier.py
python3 scripts/build-storage.py run python3 tests/build/desktop_verifier.py
python3 scripts/build-storage.py run scripts/check-postage.sh
```

The desktop gate builds a debug bundle and starts only its daemon in the
background. It does not open an application window or claim UI acceptance.
The last command is the explicit legacy regression target and needs the genuine
RISC Zero proving toolchain. `scripts/check.sh` retains that regression as part
of its broader all-feature checks; ordinary desktop builds do not invoke it.

The exact additional validation scripts used for this revision are retained as
`verify-regression.py` and `verify-release.py`. To replay them after the two build
gates above, create `output/ar2-verifier-regression` through the storage wrapper,
then run `verify-regression.py`, `verify-regression.py --backend`, and
`verify-release.py`, each from the repository root through the same wrapper.

## Validated results

The separate legacy gate passed all 13 tests, including original-receipt
compatibility with `prove`, nine genuine-proof/direct-guest checks and three
actual process tests. The latter preserve paid rows after cancellation, parent
SIGKILL and cold retry, and verify a common receipt without the sender wallet.

Full regression passes: 871 Rust tests, zero failed/ignored in 59 nonempty suites;
59 frontend tests, TypeScript/Vite, 19 model tests, formatting and workspace
Clippy. All 569 build/test fingerprints stayed unchanged. See `checks.json` and
the compressed logs; the original paid evidence also matches committed `a1ebaf9`.

The default release build also passed with guest/kernel generation forbidden:
ad-hoc signature valid, prover absent, both proving commands unavailable, the
verifier byte-identical to the accepted debug bundle, original paid receipts
verified, and actual release-node availability accurate across restart. See
`release-check.json` for binary hashes and the 180.16-second total gate duration.

Full historical retrieval, neutral public types and AR2 user entry points remain
open. [Continuation](NEXT.md). No guest, lockfile version, original paid fixture or
funding/spent rule was changed by this build split.
