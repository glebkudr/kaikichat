# Incoming operator retention in the ordinary Runtime

The shared daemon constructor now sets independent admission quotas for incoming
data, compact paid indexes and outgoing evidence: 4096 objects /64 MiB each.
The index allowance includes portable proof, holder receipts and history; data
counts ciphertext and outgoing counts ciphertext plus its index evidence. The
row engine's separate evidence bound still applies. These are finite admission
caps, not a promise that every mix of 4096 objects fits in the byte allowance.
A full index cannot consume the data or outgoing allowance. This changes local
storage policy, not signed placement, operator consent or payment authority.

The existing `Runtime::pump` maintenance hook now attempts one bounded batch for
each store at most once per second, including while custody jobs are empty.
Each store uses its own atomic transaction and processes at most two queued new
objects; the bounded legacy compatibility archive keeps its existing behavior.
A failed namespace leaves its durable cursor, body and quota unchanged, and the
other namespaces still run. Failed attempts use the same retry interval. The
shared row commit helper preserves the existing outgoing rule that maintenance
of a fresh empty store does not create a head. It now applies to data and index
too, avoiding idle writes to stores the operator has never used.

## Verified contract

The independent backend-test-critic accepted a new real Runtime test before
production. It reuses the current signed public-funded fixture and independent
P-256 finality oracle. Real data and index admission targets the Runtime's actual
transport identity. Its local outgoing receipt is forwarding evidence for the
same authentic commitment, not an additional independent replica. The fixture
belongs to an explicit fixture network; the shared constructor selects that
network for the test, while public `Service::open` remains fixed to the normal
daemon network. Runtime Core is not populated with current payment authority.

The test requires:

- No SQL writes during maintenance of a fresh empty profile, followed by 134 real
  data obligations, 134 index obligations and their outgoing receipts under the
  constructor's unchanged policy. Cold reopening returns every exact original
  and all stored bodies, revisions and occupied object counts agree.
- An incoming SQL UPDATE failure leaves its complete namespace and head intact.
  Index and outgoing each reclaim two expired bodies. Removing the fault does
  not bypass backoff. The next attempt reclaims two incoming and the last expired
  index/outgoing body. A repeated wakeup does nothing; cold reopening performs no
  cleanup and the next attempt reclaims the third incoming body.
- With the historical fixture expired at wall time, actual `Runtime::pump` runs
  with an outgoing SQL failure and no active jobs. Data/index still reclaim two
  bodies each; outgoing state is exact. A later scheduled retry services all
  three. Surviving original bodies and revisions never change, and rolling the
  read clock backwards fails in every store without changing SQL.

The first compile exposed an incorrect import and a StoredReceipt/portable-bundle
fixture mismatch. The corrected RED then reached actual `put_index` and failed
with `Limit` under the old daemon policy. The critic caught the old outgoing head
name in the SQL assertion before production; it was corrected and accepted.

[Final validation](operator-retention-checks.json) passes: 96 distinct backend
scenarios (two Runtime, 52 paid-custody and 42 paid-index), 31 frontend tests,
postage-spend/node all-target Clippy, formatting and whitespace. The 56 focused
source hashes were checked after the runs; this is not a full application input
manifest and does not renew native acceptance. Regressions overlap prior reports
and are not additional independent product coverage.

## Required continuation

This establishes shared constructor policy and ordinary pump maintenance with
signed fixture evidence. It is not a paid networking/native acceptance run in
the daemon network. Inspection observations still use a bounded whole document;
network workers still need explicit continuations and signed history pages/roots.
The complete native gate must retain more than 128 simultaneous paid originals,
restart/remove the sender, lose real data/index nodes and recover the declared
full range through ordinary recipient work. Earlier native reports remain bound
to their recorded sources. MLS epochs/Welcome, autonomous repair, full E11 and all
67 cards /22 E2E /three platforms remain required.
