# L02 durable registry selection in Core

Baseline: 9f18c4ce4fba54de95130ec9fccabecaaea02614. This increment connects the
registry verifier to the owner's persistent application profile. Daemon/UI registry adapters,
role possession, placement, paid admission and actual R=10 custody/repair remain required.

## Tests before production

Eight new Core integration tests use the existing independently generated Anvil MPT proofs
and signed checkpoint corpus on two chains. Exact complete selection and membership DTOs,
both gapped-index members, byte-preserving semantic retries/reopen, SQL failure rollback,
missing prerequisites, invalid proof/opening, stale expected head, lease/admission expiry,
corrupt saved state and actual MLS outbox/receipt preservation are covered. The existing
signed-agent rejection test additionally denies all three new owner-only method names.
The RED log contains only absent Core APIs/input type, before production changes.

The separate context-free backend-test-critic returned REVISE: the SQL test could pass an
installation that wrote the old clock. The revised test successfully installs at now+1,
reopens, checks the exact saved observation through the read-only checkpoint anchor and
requires denial at the old time. Advancing time through the existing checkpoint API also
constrains registry verification. The reviewer then returned FINAL ACCEPT. Its nonblocking
suggestions were addressed too: complete selection DTO equality, independently computed
Solidity ABI config hash, and well-formed foreign-chain/genesis persisted-profile fixtures.

## Implementation

RegistryStored is bounded/versioned and retains the exact original selected profile JSON
and checkpoint-profile binding in SQLCipher. Reads revalidate schema, profile, chain/genesis
and trust binding. A first selection requires the existing owner identity/checkpoint trust;
semantic retries preserve bytes/revision. A different domain or runtime code hash cannot
replace the selection. There is no separate registry clock.

Installation reuses the existing multi-state checkpoint CAS transaction. The profile and
higher observation succeed or roll back together. Verification loads the current saved head
and selected registry, checks the requested checkpoint ID, and reuses both existing proof
verifiers. Successful and denied checks persist higher observed time before a response;
a failed clock write cannot return a successful proof. Same-time checks perform no writes.
The result is finite membership under explicit attestors, never keeper/spend authority.

## Verification

The eight new Core tests passed. Four isolated compiling negative controls were killed,
after an unmodified eight-test baseline: lost install clock advancement, ignored verification
clock-write error, omitted restored chain binding and ignored code-hash replacement. The
shared mutation runner records source hashes, requires an actual named test failure and
cleans only its own temporary APFS workspace. Cleanup errors are empty.

The complete native gate finished with exit 0: 308 Rust tests, 24 Solidity tests (256
fuzz runs each), 40 frontend tests, TypeScript/production frontend, real Anvil funding/registry
proof gates, all five actual hidden packaged WKWebView flows, release bundle, deep/strict
ad-hoc signing verification and production WebDriver exclusion. Current chat and restored
checkpoint screenshots were viewed alongside the preceding committed native images: layout
and text remain intact, with only expected runtime timestamps changing.

The current-source Linux network gate finished with exit 0, all seven outcomes and empty
cleanup errors: `ain-nat-4867876d`, source hash `30129ed7a2b50a7e35a56eaf6b5d7f0c214930f79bc91c4bfb20b30354ef90a7`.
The added read-only AutoNAT observation retains a public→private→public transition after
actual dropped callback packets and recovery. The earlier isolated withdrawal timeout remains
recorded in the preceding increment as unreproduced; no transport fix is claimed here.
No dependency was installed. Full V1 remains open, including the daemon registry integration.
