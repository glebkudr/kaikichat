# Graceful transport shutdown

Status: bounded-drain candidate passed ordinary regression and packaged proving,
but did not resolve operator-network acceptance. Bootstrap reconnection is under
test-first review. No full V1 readiness is claimed.

The unchanged operator-network EVM scenario failed after actual provider restart.
The consumer advertised a QUIC connection absent at the restarted provider. Its
operator request remained pending beyond the original eight-second deadline and
became failed another 2.036 seconds later. The failure and reciprocal states are
retained in `../P02-daemon-jobs/operator-restart/`. A separate real-process Python
probe saw the QUIC connection remain visible two seconds after clean process exit,
while the TCP positive control retired in 3 ms. A subsequent idle probe passed.

The single-peer and revised three-peer Rust baselines passed on unmodified code,
including under an actual supervised prover with advancing CPU. They are positive
lifecycle coverage, not relabeled RED or a deterministic reproduction. The actual
pre-implementation EVM RED is the basis for this correction. The separate no-context
backend critic explicitly accepted that evidence and revised test package before
production changes. Five accepted inputs are frozen in `accepted-tests.json`.

Two focused tests use the real daemon, one TCP-only topology and one server with a
TCP client and two QUIC clients. Successful SIGTERM must finish within five seconds
without SIGKILL fallback. All peers share a two-second connection-retirement bound;
three same-endpoint/identity restarts preserve the exact historical message-ID sets,
new plaintext, one received copy and actual MLS delivery receipts. Failure cleanup
still uses the existing Node destructor. The original EVM deadline is unchanged.

The candidate aborts owner IPC, stops finalizer/prover work, removes listeners and
explicitly disconnects peers. It drives the swarm for a bounded 500-ms best-effort
transport drain without dispatching application messages or restarting listeners;
connections completing during shutdown are closed too. QUIC/request timeouts,
authority, resource limits and durable message state are unchanged. This does not
promise reliable close delivery on a lossy network or after a crashed machine.

Quinn 0.11.11 documents that immediate exit can leave peers waiting for idle expiry:
https://docs.rs/quinn/0.11.11/quinn/struct.Endpoint.html#method.wait_idle
The libp2p transport hides direct Endpoint access; its listener close and connection
poll_close paths call the existing Quinn close operations. The bounded runtime drain
allows those asynchronous drivers to run before Tokio teardown; it is not claimed
to be an Endpoint::wait_idle acknowledgment or an end-to-end delivery guarantee.

The frozen manifest captures 329 source inputs. Raw command logs and the finite
actual-prover load diagnostic are under `output/network-graceful-shutdown/`.
The candidate passed 482 ordinary Rust tests, seven models, nine oracles, formatting,
Clippy, 40 frontend tests, TypeScript, Vite, release bundling and deep/strict ad-hoc
codesign. Actual packaged daemon proof acceptance passed with a 416191-ms,
585106-byte genuine receipt and no cleanup errors. Both manifests stayed unchanged.

The first unchanged operator EVM run during proving failed after six verified roles
at wait_connected after provider restart. A second run on the exact same source
passed all 16 roles and both chains. These are retained separately under
first-candidate/ and second-candidate-run/; first-candidate-validation.json records
the command results. A passing retry does not erase the failure. Inspection found
that loss of the last verified connection leaves the bootstrap scheduler's healthy
30-second refresh deadline untouched. New outgoing-client reconnect tests and a
bounded-scheduler test are reviewed separately under ../network-bootstrap-reconnect/.
No ZK relation, proof image or dependency changed.
