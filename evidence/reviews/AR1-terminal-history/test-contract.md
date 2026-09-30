# Terminal history boundary — test contract

Full V1 / AR1 remain open. Before issuer epoch handover can be enabled, a trusted
application needs a durable terminal operation in the old authenticated log.
This component does not authenticate a successor, transfer issuer-wide spent
membership to a new committee, or relax the epoch != 1 postage guard.

New APIs: HistoryIndex::open_with_terminal(&mut store, committee, operation) and
HistoryImport::begin_with_terminal(&mut store, committee, checkpoint, operation).
The nonzero operation is trusted local application policy, not a wire claim.
Binding is persisted even for an empty or existing history. Ordinary opens and
import resumes inherit it; conflicting bindings fail. Pre-binding handles become
stale. Existing nonterminal histories remain compatible.

A terminal operation may occur only at the final prefix tip. Its checkpoint must
finalize that entry directly, not a descendant. No child append or CheckedPrefix
child construction is allowed after a terminal entry, including one in the
unfinalized suffix. Original spent lookups and exact historical retries remain;
an alternate valid QC must not replace the original checkpoint. No larger limits.

Tests in history_index.rs use the existing genuine P-256 signatures and SQLCipher
store helpers: 130 ordinary entries + terminal 131; cold ordinary open; policy
binding failure; seven-entry snapshot pages, crash, failed last-page activation;
valid-QC terminal-before-tip and indirect-terminal proofs; current expiry remains
closed. They do not claim application authorization or cryptographic wall time.

p256_engine.rs / support/terminal_engine.rs reuse the existing Commonware simulated
network, four actual engine instances, signing application and archive. All four
applications authorize entry 9; at least one actually attempts it, but configured
terminal entry 8 must finalize
with its own QC and prevent entry 9. First exercise the unindexed suffix; then
persist actual archive proofs into SQL, destroy/recover the entire deterministic
runtime, reopen policy with the ordinary constructor and attempt again. Preserve
exact terminal archive proofs. No hand-authored QC is injected into this engine
test. The shared harness gains optional history and an attempted-height counter;
existing tests keep history None.
