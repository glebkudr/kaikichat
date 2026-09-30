# Continue the full V1 goal

Do not close AR1, AR-R01 or the user goal on this gate. Preserve 67 mandatory
cards, 22 E2E and macOS arm64 / Windows x86_64 / Linux x86_64.

1. Complete durable historical reconciliation: a lagging validator that did not
   receive original receipt evidence must recover it/authenticated original
   spend evidence after other replicas retire their active candidates. Cold
   pending reads now fail closed; expired/unverifiable retained inputs still
   need an explicit lifecycle, preserving known spent facts and never refunding
   exposed tickets. Add adversarial real peer/timing tests before that change.
2. Define a unique authenticated epoch closing boundary, transfer the complete
   issuer spent state into its successor, fence old spending authority and
   refresh authority without resetting history. Keep epoch !=1 refusal until
   real multi-epoch crash/partition/same-ticket/lease tests and an independent
   backend-test-critic accept the contract and implementation passes.
3. Finish successful sender retirement and bounded ready scheduling while
   preserving discoverable paid history. The earlier 129-admission test is not
   a spent-history gate, and this 144-spend gate is not a sender-history gate.
4. Continue AR2–AR5: ordinary user/agent entry points, network paid index and
   automated R10 repair, groups/device recovery, independent testnet and all
   required platforms. No preview scope reduction or new quota substitute.

Use /Users/glebk/Code/chat and the build-storage wrapper. A yielded test command
is still running; preserve its handle and collect its final exit. Use tests first,
separate critic with no inherited context, then production, backend and frontend
regression. Do not touch unrelated user media or maintenance notes.

## Existing recovery components to reuse

`crates/node/src/postage_submissions.rs` already transports exact SpendRecord
bytes to ordinary clients; `postage_client.rs::receive` re-authenticates them,
and `agentic_postage_spend::verify_record` binds original QC/journal/nullifier
and verification time to locally authenticated historical authority. The current
submission request also requires the original candidate input, so it cannot
serve a validator that missed that input. Design a bounded authenticated lookup
for an actual pending archive delivery, rather than fabricating a new verified_at
or permanently retaining every terminal receipt in the active queue. Serving a
historical record is not new spend admission. Test a validator offline before
candidate distribution, returning only after all other queues are empty, plus
wrong-scope/altered responses, disk failure and recovery without owner resubmission.

The finalizer service currently revokes/restarts on any effect error. A recovery
path must account for that lifecycle: keep at most one deferred delivery or use
an independently fenced read-only recovery capability; do not ACK early or rely
on a transport permit immediately revoked by the failed effect. Broad API changes
require tests and critic acceptance before production.

## First concrete test for the next implementation

Keep one genuinely selected daemon offline before owner submission. Finalize a
fresh paid spend on the other three and require their active queues to be empty.
Restart/connect the lagging daemon without giving it the owner's receipt. Require
it to recover the exact authenticated original SpendRecord through the network,
commit its history and consumer cursor, then restart alone and read the same QC.
Observe its actual failed/deferred delivery before testing recovery; never infer
its durable archive solely from progress on other nodes. Add wrong-nullifier/
wrong-journal/foreign-committee and post-receive SQL failure assertions at the
appropriate policy/storage layers. Preserve the sender's and all replicas'
existing spent facts. This is an evidence recovery path, not another vote or
new ticket admission.
