# AR3 Core history persistence acceptance

Accepted local prerequisite: exact outgoing finite history publication and durable
incoming anti-rollback checkpoints, including anchor changes. This is not native
multi-book retrieval, complete history, successful retirement or a V1 release.

Seven genuine SQLCipher/MLS tests received independent REVISE → ACCEPT before
production. The [final compile baseline](baseline-3.json) against `743e122`
contains 45 missing-API/type errors only. Earlier baseline files remain historical.
The [test contract](TEST_CONTRACT.md) and [critic record](critic.md) preserve the
review sequence and accepted hashes.

The [affected gate](checks.json) passes **91 backend /21 frontend tests**, production
Clippy and formatting, with **597 unchanged source inputs**. Backend groups are
24 crypto, 29 Core custody, six Core mailbox, 31 paid index and one node custody.
No full workspace suite or new native gate was run. Commands use the managed
build-storage wrapper; raw logs remain in `output/ar3-history-core/` and hashes
are in the report. [Source fingerprints](source-inputs.json) cover tracked and
nonignored source inputs under crates/apps/scripts/tests and Cargo manifests.

Checks cover exact signed descriptors with disjoint candidate routes, an older
longer-lived anchor, cold retry including an expired short reference, actual
first/update SQL failures in both directions, immutable non-history state and
fetch progress, current direction/real MLS epoch fences, foreign conversation,
signed rollback/equivocation/earlier issuance/live omission, expired pruning,
checkpoint retention after expiry and cold signature corruption. Existing
paid-store tests continue to cover signed descriptor-hash substitution through
the now-shared verified-reference rule.

The runtime last accepted at `31ab80d` still intersects live-job index rosters.
The [Core contract](../../../spec/custody-history-core-v1.md) describes current
epoch and retained-original limitations. [Next work](NEXT.md) includes locator
binding, reference-bound atomic import, transport and actual multi-book/epoch
acceptance. All 67 cards /22 E2E /three platforms remain required; V1 stays open.
