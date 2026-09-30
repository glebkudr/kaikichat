# Required independent backend test review before production changes

Repo /Users/glebk/Code/chat, HEAD d7d806654ebfdf584bf0d23ac1e391a40d084af2.
The user's full goal is a real V1 messenger with all mandatory backend/frontend/E2E
passing. No full-product completion is claimed. New backend tests first, separate
no-context critic FINAL ACCEPT, then production. Do not edit files.

The previously independently accepted exact 100-request gate failed on unchanged
production. Archived raw evidence, exact source/accepted manifests, runner/logs and
source copies are in output/postage-spend-concurrent/failed-initial/.
The first run is preserved; do not treat future reruns as erasing RED.

Actual RED: all 100 unique concurrent IPC requests became durable pending, exact
capacity retry and cold pending recovery passed, real 2+2 partition existed. After
60 seconds of public client ingress, selected candidates [1,1,2,2], selected routes
[1,0,1,1]. Each selected has 14/15 actual connected peers (13 ordinary clients).
All clients have 4 authenticated selected routes and sent 908 requests total.
No finalizer proof admission throttling, no result/effect commit errors, cleanup[].
The two fresh genuine proofs and source-chain-offline gate are in evidence.json.
Production discover_finalizer_routes creates every selected key x connected peer,
with global 2-second/per-peer 6-second pacing. The present fair per-peer cursor
needs up to 3 full rounds through 14 peers, exceeding this actual 60-second case.

Review newly written tests only:
crates/node/tests/finalizer_discovery_load.rs (actual existing scheduler included
by path). They propose order_preferred(candidates, &BTreeSet<PeerId>), which does
not yet exist. Existing three finalizer_discovery_tests.rs are untouched; exact
100-request gate, helpers, limits and deadlines are untouched.

Proposed production correction after ACCEPT: bounded preference for connected peers
explicitly configured by the owner in NetworkPreferences.bootstrap_peers (already
limited to four, already validated). These are scheduling hints ONLY, no authority.
Alternate preferred and general eligible work, preserve separate progress for both
classes and existing per-peer target progress. Fall through when preferred work is
cooling down/pending. The general class must never starve even if every explicit
bootstrap is ordinary or continually fails. No protocol change, receiver-limit
increase, test deadline extension, skipped proof or current Core/route check.
The runtime reads its existing preferences and still uses check_finalizer_peer,
actual Noise/transport identity and full Core selected P256 proof verification.

Tests use genuine generated PeerIDs and public P256 keys from the failed live run;
finite millisecond scheduling stimuli retain global2s/per-peer6s/held-pending rules.
1 preferred real operator among13 ordinary clients is found <=20s inside unchanged
live60s ingress deadline; others still receive work.
2 four ordinary preferred bootstraps cannot starve any of13 unhinted peers x3 keys within
180s (less than the actual300s head); one unhinted real operator is found.
3 held pending preferred peer cannot stall remaining3 peers within35s.
Candidate order changes and repeated client targets are included. These are bounded
scheduling tests, not fake cryptographic verification; the unchanged actual live
100-request gate and existing full TCP/QUIC finalizer gate remain necessary GREEN.

Check realistic behavior, meaningful assertions/failure, DRY/helper reuse, no weakened
existing acceptance, no unfair priority that drops unhinted work. Missing realistic
scenarios, blockers/nonblockers, verdict FINAL ACCEPT or REVISE. Production files
have not been edited since the actual failed live gate. Do not modify files.


## R2 correction after FINAL REVISE

The original one-hint fairness stimulus admitted strict preferred priority because
one peer consumes at most one of three2-second slots. Critic independently showed
strict priority passed all3 tests but four/three available hints could monopolize
all attempts. The revised second test uses4 ordinary preferred hints plus13
unhinted connected peers; all39 fallback peer/key pairs must receive an attempt
within the unchanged180-second bound, including the genuine unhinted operator.
No production file, old scheduling test, live gate limit or deadline changed.
