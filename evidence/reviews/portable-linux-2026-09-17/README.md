# Portable Linux verification, 17 September 2026

Task `01a09fc7-b11a-7df0-876b-4890ae19a78b`; archived development PR 3
(published source: <https://github.com/glebkudr/kaikichat>), targeting
`implementation/v1@252e0f5e1fc09b4b7ae5cba128cfe584c6a6eb9a`.

**This is partial verification, not a release candidate or complete V1 PASS.**
The prepared and executed application source is
`89841e0afb6e2d4e013feeac931c8fea7b67cc49`, tree
`ff54e12a3225baba1a62cf3662db8966933f0dab`. Later changes in this PR are the
Unix-IPC doctor, one test assertion and documentation/evidence. No application
rebuild followed those changes. Do not relabel the prepared manifests as the
latest PR commit or silently reuse them with changed sources.

[results.json](results.json) contains exact commands, exits, selectors,
counts, source/config/lock/artifact identities, failure records and limitations.
`delivery.metadataRecords` maps every original record to its preserved copy
under `records/`, with both SHA-256 values. Original JSON bytes are unchanged;
the three prepared manifests use lossless gzip with `mtime=0`.

| Verification | Observed result |
| --- | --- |
| Toolkit seal 006 | PASS; Rust/Cargo 1.91 and complete checksum-bound dependency payload |
| Application preparation on 89841e0 | `prepared=true`, wrapper exit 0; Clippy, debug, release, contracts and backend test executables prepared in one successful phase |
| Node library selectors | 13/14 PASS; one stale full-job assertion failed after terminal compaction |
| Core expiry/terminal GC selectors | 4/4 PASS |
| Recipient cold SQL snapshot selectors | 4/4 PASS |
| Daemon process selectors | 0/2 PASS; daemon startup returned EPERM before scenario assertions |
| Post-preparation Unix-IPC tooling | 36 targeted Python checks PASS: 7 new + 7 portable + 2 doctor CLI + 20 shared evidence |
| Actual runtime doctor | `blocked`: AF_UNIX socket creation returns EPERM; runtime tool/dependency requirements pass |
| Native A03/A04/H10/H11-portable | `blocked`; full scenarios were not launched after the shared IPC blocker was established |
| A05 | `unsupported`: real macOS app/codesign unavailable |

Earlier component runs recorded 98 build-tooling Python checks, 79 history
oracle checks, 84 Vitest checks and successful TypeScript validation. Their
wrapper reports lack full before/after source guards; they remain earlier
component evidence and are not retrospectively assigned to the latest PR
commit. The first Vitest failure is preserved.

Preparation succeeded on attempt 3 in 905.103159 seconds: Clippy 19.950942,
debug 181.021749, release 595.272277, contracts 2.986532, backend test
compilation 84.200227. Earlier SDK-linker and Clippy failures are retained.
The preparation report deliberately keeps `passed=false`: building artifacts
does not accept a scenario. All subsequent backend commands called the sealed
test executables directly. Four runtime guards recorded zero prohibited
producer invocations; this is not an OS-wide process audit.

## Prepared manifest identities

| Manifest | Original SHA-256 |
| --- | --- |
| [debug](records/public-v1-89841e0-prepared-001/debug/prepared-artifacts.json.gz) | `575beb91c9be6344cf4d356b7f8c2dbf4954fd702edd30f52e78dfde1a28a8f3` |
| [release / Diagnostic32](records/public-v1-89841e0-prepared-001/release/prepared-artifacts.json.gz) | `a5964bedd21719c7faeda327b7338d4e1da60ba1301b1ab9909a35a425532400` |
| [backend tests](records/public-v1-89841e0-prepared-001/backend-tests/prepared-artifacts.json.gz) | `8b56ba8ff826a55c44210d14d707c5d47946612689770864e12bd19fb3e0bab8` |

The toolkit manifest SHA-256 is
`9b5d2d00c5933116a74e78737ba26496b2101e5d71e9f8f279f58d4cf3b6a304`;
the 1,308,816,276-byte toolkit archive SHA-256 is
`a05d1621d12e91876c48ad4b93ad282c87b8260de78eb2b2a84557128354f882`.
Toolkit integrity and actual C/C++/OpenSSL/Rust SDK link probes passed.
The toolkit archive contains dependencies; application artifacts have their
own manifests above. Neither archive is an installer acceptance result.

## Open checks and continuation

- The failing A03 selector expected `Some(expired full job)` after a cold GC
  that its earlier assertions already required to remove that job. The final
  assertion now uses the existing exact compact-evidence helper. A separate
  no-context critic returned ACCEPT; rustfmt passed. The corrected selector
  has **not** been rebuilt/executed. Its old failure stopped the unexposed
  iteration and does not prove the exposed iteration.
- The executor denies `socket(AF_UNIX, SOCK_STREAM)` with errno 1. TCP and UDP
  loopback probes pass. Anvil loopback startup also passed. The node and CLI
  use Unix IPC; a Linux runner must permit ordinary local socket creation,
  bind, listen, connect and accept. No privileged capability, netlink access,
  IPC transport replacement or protocol-limit change is proposed. `ptrace`
  was denied separately, so the attempted strace produced no daemon trace.
- Check `doctor --suite native-spend --phase runtime` through the explicit
  portable wrapper **before** preparing the next candidate on a capable
  runner. Finish source changes and independently accepted tests, then prepare
  that exact revision once and reuse its manifests. Do not amend old
  manifests to accept a new source revision or build inside scenarios.
- A04 compact facts are retained until profile deletion, with a 4,096-byte
  per-row payload bound, measured totals and explicit growth with operation
  count. Full-job GC, SQL rollback and cold invariants are separate from two
  paid batches of 129. No actual daemon `A04_GC_METRICS` record was produced;
  runtime counts and bytes remain unknown.
- The user confirmed permanent loss of raw `output/hrt32-r1`. New/repeated
  semantic reads, admitted requests and full wall time have no retained
  baseline measurement. Sixty waits are not sixty admissions; 542 seconds
  measures publication only. Full baseline comparison stays
  `blocked-by-data-loss`, independently of a future current Diagnostic32 PASS.
- H11 native counts, the last-original SQL gate, 130-message full recovery and
  cold full-graph traversal are not observed here. Their required values
  remain targets, not measured results.

The complete proposed acceptance contract is
[verification-criteria-2026-09-17.md](../../../Docs/agentic_internet_v1_execution_plan/v1-plan-2026-09-14/verification-criteria-2026-09-17.md).
The read-only analyst helper [compare-surviving-h10.py](tools/compare-surviving-h10.py)
requires an actual current H10 output and a fresh output path outside that
run. It checks redundant counts/hashes, preserves missing historical values
as null and never claims native acceptance. Its retained-baseline extraction
was checked; no current H10 run has been compared with it.

## Original log archive

In accordance with AGENTS.md, raw run logs, binaries and local configuration
are not committed here. The separate `portable-v1-89841e0-evidence.zip`
contains 117 original files and an index, including the raw selected logs,
failed attempts, exact manifests and local harness identities. Every archived
file was read back and checked against its original byte count and SHA-256.
Archive size: 3,485,656 bytes; SHA-256:
`12be497159718672cf97294d517d9a8371e0f01569c00aa36ceb17f00edca40f`.
This name/hash is an artifact identity, not a claim of successful remote
publication. Artifact availability is reported separately in the PR.
