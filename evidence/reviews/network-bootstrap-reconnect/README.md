# Bounded recovery after a verified bootstrap peer disconnects

Status: observed graceful-restart correction accepted on the recorded final source.
Full V1 remains incomplete.

The preceding graceful-drain candidate passed its focused tests and real packaged
proving. One operator EVM run still failed to reconnect within eight seconds; a
second unchanged run passed. Both results remain in ../network-graceful-shutdown/.
Inspection found that last-connection loss removed verified peer authority but
left the scheduler's 30-second successful-refresh deadline untouched.

Two new actual daemon tests isolate outgoing TCP/QUIC bootstrap clients. An
independent unavailable AutoNAT service leaves the client's public addresses empty;
the test asserts that the provider cannot redial an advertised client route. Both
tests fail at the eight-second reconnect bound before bootstrap production changes.
Two earlier test-authoring failures are retained and are not product RED: duplicate
UI-profile creation after successful reachable-peer lifecycle, then assuming that a
UI profile existed before the user created one. The corrected process baseline was
repeated after critic revisions and both transports failed at the expected bound.

The critic first returned REVISE because absence of a duplicate request alone did
not prove active-slot retention. The corrected scheduler test fills four slots and
holds a fifth ready peer; disconnect cannot admit that peer, and only finished()
releases exactly one slot. The actual process tests also compare authenticated
rootId after each restart, plus the existing helper's fixed PeerID/listener checks.
The separate no-context critic then returned FINAL ACCEPT before production changes.
Seven accepted files are frozen in accepted-tests.json. The unit compile baseline
is the absent new Schedule::disconnected method; the process baseline is behavioral.

On loss of the last verified live connection, an inactive candidate waiting for a
healthy refresh is accelerated to the existing 500-ms first retry. That loss
consumes the first attempt; repeated closes cannot reset failure backoff or release
active work. The next failed reconnect waits one second. Existing source bounds,
four slots, relay-only filtering and ordinary 30-second healthy refresh remain.
No peer authority, transport timeout, crypto image or dependency changes.

Five focused scheduler/bootstrap unit tests and all four real shutdown/reconnect
tests passed. The final-source manifest captures 330 inputs. Raw logs are under
output/network-graceful-shutdown/ for test-first iterations and
output/network-bootstrap-reconnect/ for final regression. Previous accepted shutdown
tests only extracted the same SIGTERM assertions into a shared helper.

The final candidate passed 485 ordinary Rust tests, seven models, nine oracles,
formatting/Clippy and 40 frontend tests with TypeScript/Vite. The unchanged operator
EVM gate passed twice on the same source, with all 16 roles and both chains each
time. The actual bundled prover's advancing CPU was observed during both runs.
The release daemon proof lifecycle passed with a genuine 408315-ms, 584913-byte
receipt and independent oracle acceptance. Public receipt and reports are retained.

The existing isolated Linux gate passed all seven outcomes and removed its 32
containers and ten networks. Five hidden WKWebView product flows also passed;
the isolated profile and owned processes were removed. Four current native views
were visually compared with their previous baseline: chat, scoped agent access,
network settings and restored trust. Layout and controls remained consistent;
timestamps and temporary test peer identities changed as expected. Current images
are retained in native/. The release app still passes deep/strict ad-hoc codesign,
contains the same tested daemon/worker bytes, and its default dependency graph
contains no native automation driver. It is not notarized.

All 330 source, seven accepted test and five native-runner inputs stayed unchanged.
Exact command results, limitations and raw-log hashes are in validation.json. The
full aggregate and unrelated EVM/proof suites were not repeated for this network
correction. Canonical postage spending and complete V1 remain unfinished.

Retained focused GREEN log copies omit trailing blank lines only; validation.json
keeps SHA-256 values for the unmodified original output logs.
