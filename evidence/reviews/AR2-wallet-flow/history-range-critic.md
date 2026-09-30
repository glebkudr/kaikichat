# Native live-history range test review — 2026-09-12

The independent, context-free `index_holder_test_critic` reviewed the new
130-original gate and changes to its existing paid CLI/network helpers before
any production changes for this gate. It returned **FINAL REVISE**, then
**FINAL ACCEPT** after these test corrections:

- The authority lifetime is 1800 seconds, within the fixture's existing maximum;
  funding, finalizer selection and operator setup select the same profile.
  Message retention remains 3600 seconds; replica/page allowances are unchanged.
- Every actual ordinary ticket is checked against 1..130, with an exact final
  set; the separate setup original consumes ticket zero from the real 131-ticket
  purchase.
- Leaves retain their internal sequence ordering. Across leaves the oracle
  permits publication order, while requiring globally unique sequences and the
  exact original operation/sequence mapping.
- An SQL AFTER UPDATE audit, installed while the recipient is stopped, observes
  real cold cursor claims. Acceptance requires every ordinal 1..130 in the same
  namespace/index/epoch, zero reimports/errors, idle workers and unchanged
  committed MLS/import state. A counter-only cycle through 1..128 cannot pass.

The critic also confirmed actual CLI sends/retries/budget exhaustion, independent
finality and holder receipt verification, physical loss of 1170 ciphertext and
1170 index copies, exact 129 originals before releasing the final SQL commit
fault, then all 130 original IDs/authors/text and operation/sequence claims.
The two-original fault/retry gate remains a required separate regression.

Accepted SHA256:

```text
public_history_range.py     0ed5194e468cb4cdb0d0faf75c61f947f0c87ace6505f8f0bc4db305010017bd
public_index_recipient.py   c0def3803826421c415fcc1c6471f9f1dee077d2480e01c213f96c4029a66a73
history_graph_oracle.py     3d8223353302e50bd241f8fc76504cc4d96ef44b199b4ef7a365a777297c1f37
support/history_graph.rs   76e78895826458274c31c9d8a6aaa979bbd904cc4ef0979050729ad7b46652fc
postage_spend_node.py       594785eef075aae214ff1fc93704eceff360b3278d9fac82a15f82ad2698c69f
p256_selection.py          19645a2791a6fd45d8ee056ec3e60037925ece875b576e7d420ac89bc559491a
public_wallet_cli.py       50580086fa91ed3b07374ea0a80feaa8217acd78bb7034e4e5c2b559d437fbdb
public_wallet_flow.py      8a3708102f59a98d7e8c25d44abc7169aaa767b2327b305546f0d6f3ed1c75db
public_paid_ciphertext.py  6bbb1313d1caa281196c5265e2c0867939f9683d13a59b27d90b837d7cb53578
```

This accepts the test contract for execution, not the native behavior. R1 exited
1 during fixture construction before funding: authority3600 exceeded the
generator maximum1800. It is not a product RED. Raw traces and temporary test
profiles contain secrets and are not published as evidence.

## Explicit release profile for the range gate

R2 failed to finish its first real 16-original batch within the existing 600-second
deadline. All 823 inputs stayed unchanged and teardown completed without errors.
[Sanitized R2 diagnostics](history-range-native-r2.json) preserve the failed run;
independent finality verification and range recovery were not reached.

Before changing production, the critic accepted a narrowly scoped test delta to
run the same range using freshly built release daemon/CLI/MCP binaries. The
profile is explicitly recorded, all three binaries come from the same build
profile, and original source/binary pinning remains. SQL helpers/probes and the
default two-original regression remain debug. No timeout, retention, network or
storage allowance, data, batch size or acceptance assertion changed. A release
pass would not prove that debug overhead alone caused R2's failure.

Second **FINAL ACCEPT** hashes (superseding only these three entries above):

```text
public_history_range.py    7ff18404437b19be295454361963c5341dcca004c088ae8a6012b944472135f8
postage_spend_node.py      33970507594da415c61b0495960d8cc7befdf4f77a84aa951d1da1b27eb5b3da
public_paid_ciphertext.py  b551004825317afb189077ac6b88920ba19593a2c64c5220cc06f0391b537aa3
```
