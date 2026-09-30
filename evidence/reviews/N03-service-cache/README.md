# N03: durable signed service addresses

Ordinary selected daemons now retain original signed service addresses in their encrypted
profile. The node reauthenticates both signatures, lookup scope, original issuance and
finite expiry on load. Sixteen bounded scope entries preserve version barriers through
restart and changes of local interest; expired hints grant no route. A shorter newer
lease does not revive an older signed address. New live scopes cannot evict existing
live floors. A newer binding can rotate transport after an old maximum address sequence.

Core supplies only a fixed-namespace opaque snapshot read/CAS API, bounded at 160KiB.
It atomically commits the snapshot and checkpoint clock before success. No owner IPC,
MCP or generic namespace mutation endpoint was added. Original record authentication
stays in the existing node codec. Cache bytes never replace current Core membership,
actual authenticated connection checks or the prior durable transport binding floor.
Runtime reloads before address use, persists network updates before installing them and
exposes loadedHints/cacheErrors. Load/write failure prevents using a new hint in that tick.

Seven new tests (two Core, five node) preceded implementation. Critic R1 required a
valid JSON snapshot with a damaged embedded signature and isolation of an alternate
bootstrap address source. R2 accepted both corrections. The first compilation also
exposed unsupported SQL u64 reads in test helpers. R3 accepted their narrow i64
compatibility correction before production. Corrected tests then failed to compile on
missing new Core/Cache APIs; this is compile RED, not executed behavioral RED. After
implementation, all 41 Core finalizer tests and five new node tests passed, and the
extended existing test helper built. An initial production StateValue move error and
module-order fmt failure were corrected; reports are retained, with logs referenced
by SHA in ignored output/service-discovery-planning.

The first full cache regression exposed an existing supervisor clock ordering defect:
TCP cold replay stopped at durable cursor 10 and both scopes repeatedly restarted;
QUIC was not run. Its failed report and trace are retained in network-attempt-1.
A new test-only slow effect crosses a real second during isolated cold replay. The
clock critic first required a bounded sibling-startup wait, then accepted the revised
test before any clock production edit. Against the old supervisor it failed with a
1104ms callback and a second service generation (clock-red evidence).
The supervisor now carries its fresh post-effect observation through subsequent slots
and discovery in the same pump. Fresh post-effect authority checks, real clock rollback
refusal, expiry and failure-before-ACK remain enforced. The exact reviewed test then
passed over both transports: both scopes stay in generation 1 and effect 11 replays
without a live quorum. This is executed behavioral RED for the supervisor fix; the
new cache API's earlier RED was compilation-only. A slow callback crossing actual
expiry remains a nonblocking suggested addition; ordinary actual expiry is tested.

The tests use actual SQLCipher reopen, cache/checkpoint INSERT/UPDATE failure,
whole l2-state equality, stale writer denial, duplicate idempotency, exact restored
signed bytes, transport rotation, signature/scope poisoning controls, cold corruption
rejection and exact restoration, short expiry/clock rollback, and actual 16→17 capacity
refusal followed by safe reuse. No persistence replacement mocks are used.

The live DHT gate proves unseeded cold recovery rather than just a cache counter:
all four selected processes and both ordinary DHT seeds stop. The test helper acquires
the normal profile lock and deletes only network/peer-records from each stopped test
profile, comparing every other state and reporting the exact retained service-cache
SHA/revision. Contacts, LAN, relay and AutoNAT address sources are absent; configured
bootstrap addresses point only to the stopped seeds. All four selected processes then
reopen their profiles, load at least three hints each and establish the exact selected
routes over real authenticated connections within their original finite leases.
The helper is excluded from the ordinary app and is not a production cache-clear API.

Final regression and packaging used the same 490 frozen source inputs; initial focused
Core/node checks predated the two-file supervisor correction. The successful focused full
TCP/QUIC gate was reused by exact manifest equality in the later aggregate run;
it was not redundantly rerun after unrelated checks:

- 581 Rust tests across 73 suites, zero failed/ignored; 40 frontend tests; fmt/Clippy.
- Actual bounded announcement TCP/QUIC preflight and DHT discovery, address move,
  genuine paid finality, seeded and isolated unseeded cold recovery. Two fresh genuine
  receipts, 12 verified QC signatures, 3365 owner calls,
  cleanup errors empty. No selected endpoint was supplied.
- Full TCP/Noise and QUIC finalizer regression retained every previous assertion and added the slow-effect check:
  13 durable effects per profile, 72/69 independently verified signatures,
  911 owner calls, cleanup errors empty.
- Ordinary Tauri package rebuilt from the same sources; deep/strict ad-hoc codesign,
  test driver/helper exclusion and both genuine historical receipts passed, including
  wrong-operation and expiry refusal. The fixed proof image remains unchanged.

Eight final test/spec/fixture files stayed byte-exact. runtime.rs exactly matches its
accepted test registration after removing one production module declaration. Ten of
eleven original reviewed helpers stayed exact; the full finalizer E2E helper
and its test-only Rust application were separately reviewed for the slow-effect
regression. All 84 original production baseline hashes match f5532b9. The later clock
review froze 128 production inputs before the two-file supervisor correction.
Native UI and Linux were not rerun; the app is not notarized. Exact 160KiB positive-bound
and runtime-level fault/no-dial controls remain nonblocking suggestions, not claimed
coverage. Reserved committee connections, DHT roles, ordinary client discovery inputs,
private paid ciphertext admission/storage, R=10 repair, UI/MCP and the remaining full
V1 product scenarios remain open. This checkpoint does not complete the product goal.
