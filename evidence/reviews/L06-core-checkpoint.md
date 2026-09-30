# L06 durable Core checkpoint state

Date: 2026-09-05. This increment persists the explicit ADR-05 trusted-attestor profile,
issuer and last authenticated checkpoint in the application's SQLCipher store. It does not
complete V1, L06 or E21; owner IPC, live observation and admission remain integration work.

## Tests before implementation

Six Core integration tests were added before any production checkpoint state code.
`L06-core-checkpoint-red.log` records only missing AppCore methods, exit 101. They reuse
actual SQLCipher profiles, MLS contacts/outbox/receipts and the existing independent Anvil
funding vectors, including paidWei above u128::MAX. Synthetic signed successor headers
isolate the state transition contract; they are not evidence of live chain finality.

The separate, originally context-free `/root/node_test_critic` returned REVISE because a
post-reopen status call could mask missing denial-time persistence, and generic error checks
could mask invalid input instead of agent authorization. The revised tests read SQL directly
immediately after denied funding/accept requests, then make the first call after reopen use
an earlier time. Agent denial requires exact Unauthorized with the same grant first proving
successful permitted snapshot access. A relative size bound also rejects accumulated head
history. The parent waited for FINAL ACCEPT before production implementation.

## Implemented behavior

- One strict versioned `l2/checkpoint` CAS row, at most 192 KiB, contains the original selected
  profile/issuer JSON, exact last certificate, verification time and maximum observed time.
- An existing identity and matching application network/issuer binding are required to install.
  A manifest is immutable after installation. Equivalent encoding/authority order retries do
  not replace saved bytes or erase the head; there is no reset or agent broker method.
- Restore validates the profile, issuer binding and certificate at its historical verification
  time. Every funding use still checks current expiry. A previous recorded later time blocks
  acceptance/funding at an earlier time, including after closing/reopening the profile.
- Valid successors require current CAS revision. The same checkpoint ID with another valid
  signer subset retains the original certificate. Forks, skips and stale revisions fail.
- Certificate and time commit together. Higher time from denied acceptance/funding also commits,
  without changing the head. An SQL failure prevents a successful funding result from escaping.
- Funding uses the pinned issuer and current expected checkpoint ID with the existing real
  Ethereum verifier. DTOs retain exact U256 amounts and explicit trusted provenance. They do
  not represent available balance, custody or a spending permit.
- Damaged L2 state fails closed locally without preventing Core startup or existing chat.

The trust assumption is unchanged: a dishonest selected quorum can attest false/historical
roots. This is not a light client, hardware monotonic clock or protection against restoring
an entire older database. Future adapters must supply trusted time and preserve the explicit
manifest provenance. No external dependencies were installed or upgraded in this increment.

## Evidence

- `L06-core-checkpoint-green.log`: all six scenarios pass, including actual INSERT/UPDATE
  trigger failures, exact retry/reopen persistence, failure-time durability, bounded 16-head
  replacement, agent denial, corrupted state and unchanged real MLS delivery/receipts.
- `L06-core-checkpoint-mutations.json`: clean six-test baseline; five isolated compiling
  mutants all fail behavioral assertions: missing acceptance-denial clock write, missing
  funding-denial clock write, disabled clock fence, stale CAS acceptance, missing successor
  ordering. The first funding mutation had a type-inference compile error and was excluded;
  the retained `*-invalid.*` evidence records it. Its corrected, compiling version is killed.
  Scratch directories were removed; application source/target were never mutated.
- `L06-core-checkpoint-network.log`: all seven actual Linux network scenarios pass, run
  `ain-nat-1a702f64`, sourceHash
  `3f05b7369a9ae9c6c101a752534de54f3d7be860a3788047983d4612ebab6a11`.
  cleanupErrors is empty; exact-run Docker container/network inventories are empty.
- `L06-core-checkpoint-native.log`: final exit 0; formatting/Clippy, 267 Rust tests,
  14 Solidity tests/256 fuzz sequences, 30 frontend tests, TypeScript/Vite, all prior
  live EVM verifications (26 funded-proof and 12 trusted-checkpoint CLI checks), four actual
  packaged hidden WKWebView flows, release bundling and deep/strict ad-hoc signature passed.
  The normal release excludes the automation driver. Apple notarization is not configured.
- The actual restored-settings WKWebView screenshot was inspected beside the preceding run.
  Layout/fields/relay status remain intact with new fixture ports and PeerID. No foreground
  browser or focus-stealing application launch was used.

Current EVM reports have passed:true and cleanupErrors:[]; trusted-checkpoint sourceHash is
`89dd814cd9a071bfceb5b50792f72f7d53e5526646d56de448671428b886e777`, and funded-proof sourceHash is
`13374fd51426bcfcb1143243f99fc84a88e22af3d467c1d3ec7d8a97fe6dd585`. The selected trust mode
remains explicit, with chainFinalityVerified:false. Logs normalize trailing whitespace only.

## Traceability

This contributes durable state, replay/clock and restart failure coverage to L06.T01/F01 and
profile separation to L06.N01. No L06.E01/E21 admission claim follows from a Core API. Next is
owner-authenticated installation/status/acceptance/funding through the real daemon, followed
by bounded live sources and the independently specified spend/custody protocol. All required
E01–E26 product acceptance and remaining V1 modules stay open.
