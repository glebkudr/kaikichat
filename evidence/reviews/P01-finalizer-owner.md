# P01 finalizer owner state and daemon IPC

Baseline42d1747. The Core/daemon now retain the selected public P-256 finalizer policy,
encrypted enrollment keys and explicit owner intent. The owner can register keys and verify
a full committee against the daemon's actual durable checkpoint and clock. The new release
contains these Core/node changes. It does not yet start the selected daemon voting service
or expose its native operator panel. Full P01/V1 and E01–E26 acceptance remain incomplete.

## Tests before production

Core tests came first. Existing32 registry scenarios were preserved by extracting their
generic profile/SQL helpers; they passed before production and again afterward. Twelve new
tests use retained actual two-chain FinalizerRegistry fixtures, n4/7/10/16 roster oracles,
independent OpenSSL signatures, real SQLCipher faults and durable restart/clock outcomes.
The separate no-fork Core critic initially required two stronger controls: old witness with
the new head ID, and mismatched chain/genesis with a recomputed matching policy domain.
Both were added before FINAL ACCEPT. RED then contained only41 absent Core API errors.

Accepted Core test SHA256:
e9075fbb9740f0efb737beb3b38946c91cb1c887b2f61e95bfd39feac19c0b96.
Production added a separate profile and encrypted key module, reused checkpoint CAS/time
transactions and RegistrySelection's public summary, factored shared profile validation
inside finalizer selection, and factored the registry commitment/enrollment ABI helpers.
All12 tests passed, as did the old32 and Core Clippy.

Next came two real daemon process tests and the fresh two-chain EVM runner, while node
production was still unchanged. They failed on absent owner methods. The independent
no-fork node critic required valid JSON with exact/+1 size boundaries and an EVM report
reset before fingerprint calculation. Those changes and exact restored message/outbox
assertions received FINAL ACCEPT before the owner routing/adapter implementation.

Accepted node test SHA256:
075802e4615c02d256aa8d0146351f50934d307207fe891c9eb0c84437b3fb41.
Accepted new EVM runner SHA256:
7f9056b5b64d3eb9e4d647e1bdd8949dac96e12890ce558bc19cdee968d0e290.
RED/GREEN, the legacy controls, Core Clippy and the source manifest are retained beside this
report. No production implementation preceded the relevant critic's final acceptance.

## Observable behavior

The finalizer policy coexists with custody's registry without replacing it. Installation
binds network, checkpoint profile, registry domain/code and issuer chain/genesis; retries
retain the same configuration. Core loads its own current head for every selection, checks
the complete authenticated public roster and returns a finite typed authority only after
the shared clock commit. Public JSON is diagnostic data and cannot be loaded as authority.

Registration generates a private P-256 scalar in the encrypted profile, separate from owner,
transport and custody keys. Public metadata contains only the enrollment coordinates,
signature and commitment. Reload derives and validates the persisted key/commitment.
Keys start disabled; owner intent uses CAS and survives restart. Disabling is permitted
during clock rollback without moving the saved time backward. Enabling does not establish
membership or start voting. Storage failure never acknowledges a partial key/profile/role.

Seven strict owner IPC methods delegate to Core. They reject caller time, secrets and root
overrides, malformed members and excessive sizes. A valid messaging agent first exercises
its allowed conversation access, then is denied every owner method. The actual queued MLS
message survives owner setup and daemon restart; corrupt/failing finalizer storage does
not break the established chat.

RustCrypto p2560.14.0 was already used by the locked Commonware dependency; Core now uses
that same stable release directly. Tests use the already locked sha3 0.12.0. Their current
stable versions were checked against [official p256 docs](https://docs.rs/p256/latest/p256/)
and [official sha3 docs](https://docs.rs/sha3/latest/sha3/) before adding the dependency edges.
No external package version changed in Cargo.lock. There is no handwritten curve code.

## Fresh real EVM and daemon evidence

For each of two fresh Anvil chains, one actual daemon generates17 random keys. Independent
Python/OpenSSL checks compressed coordinates, SHA-256 ECDSA, low-S, wrong-subject refusal
and Solidity ABI commitments. All17 paid registrations are accepted by FinalizerRegistry.
The existing scenario preserves the exited gap and post-freeze live changes, yielding a
frozen population of16. An independent full-list/CBOR oracle supplies the complete expected
committee and provenance. No test accesses the daemon's private scalars.

The owner IPC rejects selection without a saved head even after owner enablement, rejects
substituted/missing/duplicate members, and returns the exact positive committee report.
Renewal preserves canonical committee identity while replacing proof authority. Old head
requests and old witnesses with the new head ID fail. The chain is stopped during checks;
daemon restart retains policy, public metadata and role. Actual wall-clock expiry rejects
the previously valid request, both before and after another restart. A caller-supplied
time override is refused. Disable intent remains durable.

Both focused and aggregate fresh runs pass34 independent PoP verifications,24 committee
checks and96 actual owner calls, with empty cleanup. The full reports, current requests,
actual witnesses/registration receipts and results are under output/evm-e2e/finalizer-node/.
P01-finalizer-owner-evm-reports.json retains every aggregate report and its hash/freshness.
This proves enrollment and owner verification through one daemon per chain. It does not
prove four independent voting services or production chain consensus.

## Complete regression and packaged app

`scripts/check-native.mjs` exited0. Its nested aggregate passed436 Rust,29 Solidity,7 model
and40 frontend test functions:512 total. All17 EVM reports are fresh and successful, with
empty cleanup where applicable. Formatting, all-targets workspace Clippy, TypeScript and
frontend build passed. The actual custody resolver verified290 selected positions through
2628 owner calls. The legacy and P-256 selection CLIs passed36 and28 checks.

Fresh isolated OrbStack Linux acceptance passed7 outcomes: LAN discovery/opt-out, direct
messages, separate NATs with relay, failed/replaced relay, successful and blocked hole
punching, and AutoNAT withdrawal/recovery. Cleanup was independently checked by run label:
no test containers or networks remain. These are messaging/network regressions of the
new Linux build; the new finalizer owner EVM flow ran on macOS.

Five actual hidden macOS WKWebView flows passed: two-client conversation and receipts,
daemon delivery while UI is closed, native scoped MCP provisioning/revocation, relay/network
preferences with restored queued work, and trusted checkpoint selection/catch-up/restart.
Four before/current screenshot pairs were inspected together by own vision: chat, agent
permissions, restored checkpoint and restored network settings. Layout and controls remain
readable; generated IDs, addresses and times differ. Native screenshots are retained in
output/native-e2e; comparison hashes are in P01-finalizer-owner-visual.json.

A new macOS arm64 release `.app` includes desktop, node and MCP. `codesign --verify --deep
--strict` passed; the default dependency graph excludes the WebDriver plugin. Packaging
re-signs Mach-O files, so signed bundle byte hashes are recorded separately from standalone
test binaries. The package remains ad-hoc signed, without notarization. The33 recorded
code/test/build source hashes stayed unchanged through all gates. Validation JSON records
the finished log hash, artifact hashes and exact scope.

## Remaining work

Connect enabled encrypted keys and typed current authority to the shared Simplex runtime,
with actual selected peer routing, durable progress and stop/revalidation on head, lease or
role changes. Keep quorum and canonical journal identity through missing peers/renewal.
Native operator controls, historical funding retention, spend authorization, custody/R=10
repair, groups, jobs/artifacts, trust/economy and remaining platform/product acceptance all
remain required. This increment is not a completed V1 application.
