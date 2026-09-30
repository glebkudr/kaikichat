# Native epoch handover — native gate passed

The native scenario, funded in a local EVM, passed on actual daemon-owned keys: **133 spends,
two closings and 1605 independently verified signatures**. The genuine chosen
registry epochs were 1→3→4; no roster or signer was fabricated. See the
[native result](native-check.json), [public trace](native-trace.json.gz),
[source fingerprints](source-inputs.json), [test contract](test-contract.md) and
[independent test reviews](test-review.md).

- Pending selected configuration retains paid work before any successor signer.
  Configuration and its bootstrap cursor use one atomic state batch; the separate
  [selected/client failure gate](configuration-atomic-check.json) passes after an
  actual [partial-write RED](configuration-atomic-red.json).
- `/agentic-internet/postage-history/1` discovers authenticated original choices
  backward, then imports original history forward in at most four-entry pages.
  The 16 MiB carrier, processing budget and source request quota remain bounded.
- Exact original rows, QC and verification time survive partial-page SQL failure,
  revocation, crash/reopen and overlapping old validator history. No owner RPC
  installs a QC, canonical history page or spent record.
- The original archive supplies 130 old records after its checkpoint expires,
  without a signer and while actual checkpoint renewal persistence fails. A fresh
  third member then completes both histories, reopens cold and finalizes new work.
- Ordinary clients obtain only public chosen lineage. Exact old requests return
  the original evidence; a different operation using the same ticket is refused.

Full7 completed successfully. Earlier attempts are retained as failures, including
[full6](full6-check.json), whose public snapshots revealed that 33 pages require
three 16-request/60-second admission windows. Its 100-second test deadline was
insufficient. The separately reviewed correction permits 160 seconds for transfer
while preserving the 50/60-second client/consensus deadlines, real clocks and the
600-second maximum lease. No production quota changed.

The [full regression](checks.json) passes: **871 Rust tests, zero failed/ignored
in 59 nonempty suites**, **59 frontend tests**, TypeScript/Vite, 19 model tests
and workspace Clippy/fmt. All **582 source fingerprints remained unchanged**.
The check record includes compressed regression logs and their hashes. No UI changed.

This gate does not close AR-R01 or full V1. Hostile history ingress, outage closing,
automatic renewal, remaining sender lifecycle and AR2–AR5 are still required.
[Next work](NEXT.md).
