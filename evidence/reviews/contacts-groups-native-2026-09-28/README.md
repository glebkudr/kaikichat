# Contact by ID and groups, native — 2026-09-28

`native_contacts_and_groups` in `crates/node/tests/support/swarm_native.rs`
(ignored; needs anvil, forge and cast from the build wrapper):

```
python3 scripts/build-storage.py run cargo test -p agentic-node --test processes -- --ignored native_contacts_and_groups --nocapture
```

Real daemons on a local anvil chain with the contracts deployed and ten
bonded holders. Alice, Bob and Carol never met: no invitation.

| Step | Seconds |
|---|---|
| holder directories complete | 59.9 |
| every card published | 27.6 |
| Bob has Alice after `request_contact` by his id (his intro mailbox is read every 30 s) | 30.0 |
| Bob reads Alice's first message | 0.5 |
| Bob and Carol joined the group Alice made of their ids | 30.2 |
| both read the group's first message | 5.1 |
| the removal of Carol decided by the notaries at Alice and Bob | 5.7 |
| Bob reads the message written after it; Carol never does | 4.1 |

Alice's node: 80 stores, 70 notary requests, 40 card reads, no failed
requests; one group decision, no renewed round. The full report is
[native.json](native.json).

The same run also moved the native spike's setup into shared helpers; the
spike still passes (20 messages: stored 2.6 s, three lost disks repaired
25 s, read 0.6 s).
