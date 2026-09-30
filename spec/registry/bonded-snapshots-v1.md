# Bonded operator snapshots v1 — L02 prerequisite for N05/D03

One immutable NodeRegistry deployment records fixed-price bond units, freezes epoch roots and
binds each frozen snapshot to a specified future EVM block. No public deployment/genesis is
selected by this implementation. Local fixtures exercise actual contract bytecode; they are
not a production committee size, an independence claim or an authenticated finality source.

A bond is an opaque operator commitment, one exact unitBondWei per registry entry. Opening
an entry for transport requires a role key, salt and owner binding:
C=keccak256(abi.encode(keccak256("AgenticInternet/OperatorCommitment/v1"),domain,owner,nodeKey,salt)).
The future transport consumer must verify this opening and proof of possession of nodeKey.
The registry alone proves capital committed to an entry, not endpoint availability, physical
independence or an honest operator. Copying a commitment under another owner cannot open it.
An owner cannot reuse a commitment, even after exiting. Indices are append-only, never recycled.
One fixed paid unit always has one count; no nonlinear per-key bonus or unbacked top-up exists.
Multiple units may belong to one operator. Selection and diversity must disclose that fact.

Configuration: nonzero genesis and chainId; unitBondWei:uint128>0; epochSeconds:uint32 in
60..86400; snapshotLeaseSeconds:uint32 in epochSeconds..7days; maxObligationSeconds:uint32
in1..366days; futureBlockDelay:uint16 in1..128. These are schema/resource bounds. The constructor
records startedAt and deploymentChainId. The domain is immutable and includes chainId,
contract address, genesis and keccak256(abi.encode(the five numerical parameters)). All mutations
reject a changed chainId. A chain fork retaining its ID still needs external finality verification.
No admin, proxy, pause, whitelist, confiscation or treasury withdrawal exists in this contract.

A32-level Merkle-sum tree authenticates all active bonded entries and their total count. Empty
leaf hash=keccak256(abi.encode(EMPTY_TAG,domain)), count0. Active leaf hash=
keccak256(abi.encode(LEAF_TAG,domain,index:uint32,owner:address,commitment:bytes32)), count1.
Each parent hashes abi.encode(BRANCH_TAG,leftHash,leftCount:uint64,rightHash,rightCount:uint64)
and sums counts. Tags are keccak256 of AgenticInternet/RegistryEmpty/v1, RegistryLeaf/v1,
RegistryBranch/v1. Empty levels follow that same parent recurrence. Children remain ordered.
Up to2^32 lifetime entries; nextIndex/count are uint64. Mutations touch only one32-level path.
A proof consumer must verify both hashes/counts and the full range/ordinal, not a suggested subset.
Historical proof construction requires an event index or historical state; a current tree query
must not be represented as a proof for an old root.

bond(commitment) takes exactly one unit price and assigns the caller as withdrawal owner.
requestExit(index) is owner-only, removes the leaf from the mutable tree immediately and sets
withdrawAfter=now+snapshotLeaseSeconds+maxObligationSeconds. Existing sealed snapshots remain
unchanged and can only authorize new obligations until their signed/proven admissionUntil.
Consumers must enforce that bound and maximum obligation duration: the delay cannot cover
arbitrary later obligations. Exit cannot be undone. withdraw(index,recipient) is owner-only,
requires a nonzero recipient and now>=withdrawAfter, pays exactly that unit once and applies
state/accounting before the callback. Failed transfer reverts all changes; another owner remains
usable. Total deposited=held+withdrawn; forced extra ETH creates no bond or withdrawal entitlement.

currentEpoch=floor((now-startedAt)/epochSeconds)+1. Anyone may seal the current epoch once,
only with a nonempty active tree. sealEpoch stores immutable domain/root/count/sealedAt/sealedBlock,
beaconBlock=block.number+futureBlockDelay and admissionUntil=sealedAt+snapshotLeaseSeconds.
New bonds/exits cannot change a sealed snapshot, including before beaconBlock arrives. Missing
old epochs cannot be retrospectively filled. No seed is known/captured at seal time.

captureSeed(epoch) requires an existing snapshot and beaconBlock<block.number<=beaconBlock+256.
The exact nonzero blockhash(beaconBlock) produces
keccak256(abi.encode(SEED_TAG,domain,epoch:uint64,root,count:uint64,beaconBlock:uint64,blockHash)),
where SEED_TAG=keccak256("AgenticInternet/RegistrySeed/evm-blockhash-v1"). Exact retries return the
saved seed without changing it. Missed/withheld capture has no latest-block/time/caller fallback.
Saved seeds remain historical data after lease expiry; seed presence is never current admission.

This explicitly labelled EVM-blockhash profile is biasable/withholdable by the underlying block
producer/sequencer. An attacker with g candidate outcomes can amplify a desirable event with
baseline probability p to at most min(1,g*p) by a union bound; independence is not assumed.
No numerical g bound or production safety is established here. Assignment commitments must
precede the selected beacon and the consumer must authenticate the snapshot/finality; these
N05/P04 gates remain required. Epoch root freezing alone does not prevent all assignment grinding.

Primary references: Solidity global variables document the256-block lookup horizon and producer
influence (https://docs.soliditylang.org/en/latest/units-and-global-variables.html);
Solidity security considerations describe callback/reentrancy and checks-effects-interactions
(https://docs.soliditylang.org/en/latest/security-considerations.html). Checked2026-09-05.

Required tests: exact funding/root/count/reference events; failed inputs conserve funds/state;
post-seal deposits/exits leave prior snapshot unchanged; exact future block/capture horizon,
missed capture without fallback; owner/early/duplicate exits and withdrawals; callback failures
and reentrancy; independent-domain deployment; randomized multi-owner conservation/root updates;
actual local Anvil restart/reorg and proof reconstruction. Passing this increment does not close
L02/N05/D03, Rust authenticated registry proofs, actual keeper placement or full V1 acceptance.
