# Independent backend test critic

Reviewer: `/root/index_holder_test_critic`, originally created without inherited
context. Read-only test review; no reviewer edits or production implementation.

Initial review: **REVISE**. Equal revision foreign checkpoints could be rejected
by ordinary equivocation handling without checking the checkpoint's scope.
Changed tests now require the local next revision 2 to fail with a foreign
revision 1 checkpoint, independently for another direction and another epoch;
the same local next revision succeeds without that checkpoint.

Earlier review messages also required a genuinely signed foreign-author incoming
manifest with the original body and separate candidate routes per reference.
Both are present. Four-key/32 KiB bounds preserve existing protocol limits.
Optional improvements were applied: duplicate keys within the four-key budget,
a unique 129th descriptor, and a descriptor that becomes valid at fetch time but
was issued after its referring manifest.

Final decision: **ACCEPT for the current crypto tests**. No mandatory gaps remain.
The review covers the signed bounded directory, exact references, retention and
checkpoints. Funded placement, persistence, completeness and runtime are outside
this acceptance. Baselines are compilation RED, not executed behavior failures.

Accepted SHA256:

- `crates/crypto/tests/custody_history.rs`: `8f0ef73df6f96b16f491ec153ed41e80cc7857f6f254648a4494af4189be1e26`
- `crates/crypto/tests/fixtures/custody-history/generate.py`: `4089fe20b3879bc85ac7e6ffdf35cd658adf8dd70e875d08ee071e64576f39fb`
- `crates/crypto/tests/fixtures/custody-history/vectors.json`: `cde7c8b288de0e80fc81ea1b6312cb23bf82a5fb936abc3a9880383afbbbd29f`

After ACCEPT, current `baseline-2.json` confirms nine missing-API compiler errors
against unchanged production at `31ab80d`, with no input changes. Implementation
began only after the critic completed and that baseline process exited.
