# L02 operator role bindings

Baseline: 20c9b1a2b879ada44e6a31411793ed2ee16fc14c. This increment authenticates a
finite delegation from a committed registry operator key to a particular transport key and
role. Actual connection authentication, operator key provisioning, selection, paid custody
and R=10 repair remain integrations. It does not complete V1.

## Tests before production

Six new adapter integration tests precede implementation. Existing actual Anvil registry
members on two chains provide independently authenticated snapshots/openings. Python/OpenSSL
generates eight exact role wires (both members and both roles) and correctly re-signs altered
context, purpose, lifetime and canonical-body examples. The generator reuses the existing
independent CBOR reference and stores the source registry corpus hash. It does not import
the Rust producer or verifier. Existing registry fixture/context/CLI helpers are reused;
all four original registry tests passed after extracting the shared helpers.

Tests assert exact producer bytes, IDs, full proven context, both gapped-index ordinals,
separate binding/checkpoint/snapshot expiry, orphaned state and selected chain/genesis.
Otherwise-valid signatures cannot change author, role, transport, registry domain/root,
member index/commitment, outer network/kind/epoch or time. Noncanonical/trailing bodies,
extensions, missing expiry and over-limit frames fail. Exact CLI DTO equality and raw nested
JSON duplicate-key rejection preserve the existing trust/proof boundary.

The separate context-free backend-test-critic initially returned REVISE because a weak
transport key could fail merely by differing from the caller's strong expected key. The
revised test supplies the same weak key as the expectation and retains independent proof
of a valid outer signature. A correctly signed replacement strong key with a matching
expectation additionally succeeds with exact key/ID assertions. FINAL ACCEPT followed.
Rust RED exited101 solely on missing new APIs. The fresh Anvil RED passed the existing
membership path and then failed on the missing operator CLI command; owned resources were
cleaned with no errors.

## Implementation

The new adapter module reuses SignedDocument and the current checkpoint→registry→member
verification path. The checkpoint retains its already authenticated network domain privately.
A verifier cannot pair an externally supplied proven member with another snapshot. It checks
the exact node-key author, role document kind/epoch and mandatory finite lifetime, then
compares the body with canonical bytes derived from authenticated context and local role/key
expectations. This also rejects alternate body encodings without a second CBOR decoder.
Both signing and verification reject weak transport keys. Effective validity is bounded by
both the signature and current snapshot/checkpoint lease. Results have no Deserialize or
public constructor and confer no placement, independent-operator count or spend authority.

The CLI command verify-checkpoint-operator preserves raw nested registry requests and shares
the prior context/provenance formatter. It adds exact operator role/key/binding provenance
and the effective expiry. Its expected transport key is caller input, not evidence of a
network handshake. scripts/check-evm.sh retains all previous gates through registry_operator.py,
which extends the existing actual daemon scenario with a bounded fresh-state callback.

## Targeted verification

All six new and four prior registry tests passed; strict Clippy passed. The fresh two-chain
Anvil/daemon/CLI gate passed 32 actual operator CLI calls with 8 successes. Both roles and
members are verified against the current canonical snapshot already checked by the daemon;
otherwise-correct signatures by another author, for another key or another role are refused.
All prior64 daemon calls,10 successful memberships and original offline expiry checks remain.
Reports explicitly retain transportHandshakeVerified=false and operatorIndependenceVerified=false.

Five isolated compiling negative controls were killed after an unmodified six-test baseline:
omitted role body comparison, wrong operator author, allowed weak transport key, future
issuance and lost shorter checkpoint lease. The shared runner preserved original source,
recorded hashes/named failures, and removed only its disposable APFS workspace; cleanup[]
and passed=true are in L02-operator-binding-mutations.json. No dependency was installed.

## Aggregate verification

The native gate finished with exit 0: 317 Rust tests, 24 Solidity tests (256 fuzz runs), 40
frontend tests, TypeScript/production frontend, every fresh Anvil/daemon/CLI gate and all
five actual hidden packaged WKWebView scenarios. The macOS arm64 release bundle passed
deep/strict ad-hoc signature verification; the production dependency graph excludes the
WebDriver plugin. Current chat and restored checkpoint screenshots were viewed beside the
prior committed 20c9b1a images: layout/text remain intact, with expected timestamp changes.
Notarization and supported-platform release work remain open.

The current-source Linux gate passed all seven outcomes with empty cleanup errors:
ain-nat-cdfc262b, source hash 9fbec165ca0458be691c231842a4b3ff5665f4d090b98d91324c4b81a2eff77e.
The previously unreproduced AutoNAT withdrawal timeout did not recur. No transport fix is
claimed. The full V1 goal remains active, including durable operator keys, actual authenticated
connection integration, committed-before-beacon selection, spend admission and R=10 repair.
