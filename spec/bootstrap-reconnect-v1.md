# Reconnect a verified bootstrap peer after transport retirement

The first bounded-shutdown candidate passed real packaged proving and all ordinary
tests, but unchanged operator-network acceptance still failed after restart: the
consumer had no connection within the original eight-second bound. The same source
passed a second run. Both reports remain under network-graceful-shutdown evidence.
This is not a resolved intermittent fault.

The scheduler currently waits 30 seconds after a successful self-record exchange.
Losing the last live connection removes verified authority but leaves that healthy
refresh deadline unchanged. Message outbox retries can hide this missing discovery
reconnection behavior; a bootstrap-only client has no such trigger.

Add two real daemon tests for TCP and QUIC, using an explicit bootstrap hint.
The client has no verified public address: a separate unavailable AutoNAT provider
and the actual supported 900-second probe interval keep its advertised addresses
empty. Assert that condition. The restarted bootstrap provider cannot redial a
cached client address and hide the missing client-side retry. No network mock or
firewall change is involved; this models an ordinary outgoing client behind NAT.
Require initial reciprocal authentication, successful SIGTERM within five seconds,
removal of stale connection/verified state within two seconds, and reciprocal fresh
authentication within eight seconds of same-endpoint/persisted-identity restart.
Repeat twice without importing contacts, sending messages, changing network settings
or explicitly dialing. Bootstrap creates no conversations. Only after this passes,
import an actual invitation and check MLS plaintext, one copy and delivery receipt.
Reuse the existing clean-stop and bounded-wait helpers without relaxing their tests.

On loss of the last connection to a verified peer, schedule its first reconnect at
the existing 500-ms retry delay instead of the 30-second successful-refresh delay.
Only a retained, inactive healthy candidate may be accelerated. Duplicate closes
cannot advance or postpone the deadline, release active work, reset failure backoff
or insert unknown hints. Four active candidates and a fifth ready candidate prove
that a close cannot release a slot; only finished releases exactly one. A failed
reconnect waits the existing next one-second
delay; later failures retain bounded exponential backoff. Normal healthy refresh,
four shared slots, relay-only filtering and source bounds stay unchanged.

The deterministic scheduler test covers those resource limits at explicit Instants;
the real process tests must reproduce the externally visible failure before the
bootstrap implementation changes. The separate no-context critic must accept the
test package first. Run all backend/frontend and the original EVM scenario with
unchanged assertions, then actual packaged prover lifecycle acceptance on the final
daemon. No reliable close or instant recovery is promised for a crashed or lossy
remote network, and this module does not complete the V1 product goal.
