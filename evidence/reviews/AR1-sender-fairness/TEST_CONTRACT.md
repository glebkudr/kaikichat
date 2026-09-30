# Bounded sender scheduling — test contract (independent review accepted)

Architecture finding AR-R02: one job every 500 ms makes a full 128-job queue
wait at least 64 seconds between visits. Keep the admission bound and the
existing allocation, grant, finality, receipt and pointer stores. Do not claim
complete retirement, a new job ledger, or complete event-driven scheduling.

The funded native test creates 127 ordinary messages through real Core send
transactions while the daemon is stopped, then lowers owner sponsorship to zero
through the ordinary Core configure API. A live runtime grant retains one ticket.
The helper never injects SQL rows or manufactures stamps, QCs or custody evidence.
The next actual signed runtime call admits the 128th message. Original messages,
queue order and cumulative reservations are checked.

With all finalizers offline, the ready tail must prepare its actual MLS envelope
and funded native stamp within 12 seconds. With its pending ticket retained, the
sender crashes. Real finalizers return, and normal P2P must provide independently
verified finality within 20 seconds measured after reconnect setup. Both elapsed bounds are
explicitly asserted after the shared polling helper returns. Original preparation and allocation must not
change. All 127 paused jobs must remain unauthorized and unspent, all 128 jobs
and sponsorship counts must remain exact. Owner IPC is sampled during the run
for every observed method and must stay responsive. A separate live phase pauses
the ready runtime sponsor, observes its existing job become unauthorized, then
restores the sponsor and requires that same prepared job to resume without a
new send or restart and without changing allocation, queue or reservations.
Existing full sender E2E will separately guard custody,
publication, offline receive and no repeated puts after restart.

Intended bounded implementation: reconcile durable membership, retain a disposable
ready queue with per-job retry deadlines, skip sleeping work, and bound each
maintenance pass by advancement count and elapsed time. Every actual advancement
still reauthorizes through Core. This is not a change to MAX_ACTIVE and does not
interpret recipient ACK or spent tickets as refunded capacity.

Independent critic ACCEPT is retained in test-review-2.json; the diagnostic
revision was accepted in test-review-3.json. Production was unchanged at review.
This describes the pre-implementation contract. Candidate-specific results and
remaining gates are recorded in README.md and the matching source maps.

The first baseline failed during cold admission setup at its 40-second helper
timeout, before any scheduler assertion; this is not a production RED. Real
SQL/MLS admission gets a separate 180-second setup bound. Scheduling deadlines
remain 12/20 seconds and the clock and queue limits are unchanged.

The next two baseline runs completed genuine admission. One lost the initial
node_info reply on full-queue startup; the diagnostic run reached signed runtime
admission but timed out before receiving its response. These are actual IPC
responsiveness failures under the authentic backlog, not a measured tail-delay
result. The diagnostic run records 66.78 seconds for cold admission and preserves
phase, exception and daemon logs.


The later resolver cache preservation contract keeps full incoming verification
and requires the current Core authority plus the original authenticated
connection and binding lifetime on every read. The actual TCP/QUIC gate stops a
selected provider, observes disconnection only through node_info, restarts the
same identity, waits for reconnection, and only then reads the original cached
resolution. The original binding must still be unexpired and the head unchanged;
that one offer must disappear, and a new resolution must recover the same peer.
Existing binding/head expiry, copied proof, network replacement, relay-only,
request capacity and cold-cache tests remain required. Reviews 11–12 record the
separate critic's request and acceptance before the cache production change.
