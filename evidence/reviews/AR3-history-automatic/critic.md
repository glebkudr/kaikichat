# Independent test acceptance before ordinary runtime integration

Reviewer `/root/index_holder_test_critic` was separately spawned without inherited
context and reused. It reviewed only changed native tests, test-only fixture and
related source/helper context. It did not edit files or execute checks. Production
for this slice was unchanged at prerequisite commit `45d2681` during the initial
acceptance. Subsequent test-only corrections are recorded separately below.

First REVISE: sender diagnostics alone did not establish ten actual remote
manifest writes; cold dedup waited only for lookup initiation. Tests now read
exact paid history bundles from all ten claimed anchor ACK peers and wait for an
observed verified directory/progress pass with no active/pending custody work.
Optional fixes select surviving indexes jointly and accept null pending history.

Second REVISE: the shared independent index checker implicitly read the setup
sentinel envelope during the new anchor audit. An optional explicit `envelope=`
argument now supplies the actual anchor without changing existing callers.

Final verdict: ACCEPT. No required missing scenario within this integration
boundary. Accepted SHA256:

| File | SHA256 |
|---|---|
| tests/evm/public_index_sender.py | 7dd7e4fe63267cd39b9b2cc50c26eb3371fd53e9ebd820ba32d3d91c2b150ba4 |
| tests/evm/public_index_recipient.py | 785eafba39a21f1ea8b07a4f6d21496add020d46e10dc84f0d65abdba97813d2 |
| tests/evm/paid_ciphertext_delivery.py | c75a519593d12efe077b50ef1605654cb6783b78471862c665cbacb1513a2c68 |
| tests/evm/paid_index_network.py | 7d045817e1c483c6fe0a83390b8582d780577d1dc0e6a5247f481a6b6669bb6a |
| crates/node/examples/postage_spend_store_failure.rs | 9f03a12b1cc2b5051d393f30b127584c52e8a9f8746707374ee638e0759076f6 |
| TEST_CONTRACT.md | c19f57c7a4137172b7c2ad5987cbcc79b5b6b6f7da32a360838e96fec9cc7fd1 |

Baseline recipient reached real stored ordinary work and failed because no signed
manifest was exposed. This was before the final optional-argument audit fix; the
failing assertion and its executed path did not change. Later recipient import
and all-ten audit assertions were not executed by that baseline.

## Native sender phase synchronization correction

Candidate-sender-1 reached the real manifest preparation SQL fault, including
cold retry. Immediately after trigger removal its next status still contained
the previous blocked/storage_error with null history. The test asserted a new
revision before the asynchronous worker retried. The raw failing run is retained.

The test now waits for blocked/storage_error **and** a prepared revision before
accepting the ACK-fault phase. Requiring blocked excludes a queued request whose
publishing status retains an older error. No stored state or ACK is permitted
while either SQL fault is active. Production was unchanged by this correction.

The same separate context-free critic returned ACCEPT: the new condition
distinguishes preparation, pending transport and actual ACK failure. No blocking
issue or additional required scenario for this correction. Final sender SHA256:
`21ff20ace47961a414cefc6a5ea8d18a174df8e023495c057243fb6da8cb2980`.
The other originally accepted test/helper hashes above remain unchanged.

## Additional failure diagnostics

Candidate-recipient-2 timed out in publication after both exact manifest ACK
sets reached ten. Both pointer views remained sequence 2, publishing, zero ACKs.
Status calls took up to 4.9 seconds. The test now retains the existing node_info
routing and mailboxes fields alongside its other network samples, with no new
RPC, assertion, timeout or sampling-frequency change.

The separate critic returned ACCEPT for this diagnostic-only delta. Removing
the two added keys reproduces the previous accepted test hash. Current sender
test SHA256 is `5364b810db9d0193a981df2d55e0fd24bab057c5814647f55548336676ee36f8`.
The production follow-up delays full sender revalidation while its pointer
publication is already pending; the critic verdict concerns tests only.
