# N04 AutoNAT callback negotiation and bounded cleanup

Baseline:49b9d14. This resolves the intermittent callback confirmation issue discovered while validating signed contact route refresh. Full V1 acceptance remains open.

## Observed failure

The current-source Linux gate sometimes timed out on public reachability or firewall recovery. A read-only status trace showed accepted service requests followed by client probe expiry, despite an otherwise working network. A separate diagnostic image added opt-in public network-event logging, ran the existing AutoNAT scenario unchanged, and removed only its own containers/networks. All temporary production logging was removed before implementing this correction.

The diagnostic trace is retained in `output/network-e2e/autonat-callback-negotiation-red`. At1788595393244 the service reported ConnectionEstablished and a successful probe response, then closed callback ConnectionId2 immediately. The client received the response but its same-time incoming transport failed with multistream `ProtocolError(IoError(BrokenPipe))`. A local transport establishment event does not imply that the remote endpoint has finished protocol negotiation. Some probes worked depending on this ordering; a passing run did not remove the race.

## Tests before implementation

Three pure lifecycle tests were added before production changes. They require no wall-clock sleeps or network mocks. The first forbids closure at local establishment and before the existing eight-second probe timeout, while requiring one exact callback closure at expiry and no closure of an ordinary connection. The second covers client-initiated close, dial failure and a late callback that completes after its original request expired. The third keeps two callbacks of one PeerID alive simultaneously, with different deadlines, and proves that each exact ConnectionId expires independently while a failed third dial does not remove either.

`node_test_critic` initially returned REVISE because the first two tests could be passed by incorrectly tracking only one connection per peer. The overlapping callback test was added; the critic then returned FINAL ACCEPT. `/tmp/ain-nat-callback-lease-red.log` demonstrates absent CallbackLeases. No timeout, assertion or existing network scenario was relaxed.

## Implementation

GuardedAutonat tracks only the exact connection IDs created for accepted service dial-back requests. Local establishment starts a finite callback lease using the existing AutoNAT probe timeout. The verified client can promptly close its own callback; ConnectionClosed and failed dials remove tracking. The regular100ms runtime maintenance schedules exact expired connections for closure, including abandoned and late callbacks. Other control, chat and relay connections to the same peer remain untouched. Swarm connection/admission limits still bound pending and established resources.

Client reachability still requires both a valid service response and a fresh authenticated matching inbound callback before its original probe deadline. No bare success response becomes public. Service-side cleanup now allows the remote negotiation to finish rather than racing it with immediate teardown.

## Verification

`/tmp/ain-nat-callback-lease-green.log`:all8 node unit tests passed, including the three new lifecycle cases and existing reachability ordering/security checks.

`/tmp/ain-nat-callback-lease-native.log`:complete native gate passed187 Rust/27 frontend tests, formatting/strict Clippy, TypeScript/production frontend, all four actual packaged hidden WKWebView flows, normal release bundle, deep/strict codesign verification and no WebDriver in the ordinary dependency graph. The current restored-settings screenshot was visually checked alongside the component reference; saved configuration, contact/history and confirmed diagnostics remain intact.

`/tmp/ain-nat-callback-lease-network.log` and `output/network-e2e/result.json`:all6 Linux outcomes passed with current source hash `5e341909f2dcb3a6e1f821081291e553765b4fb71a90fd1a9d0d0b08ea43f10c`. Initial public detection has successful1/failed0, intentional firewall denial advances failed to1, restoration advances successful to2 without another failure, and the combined relay/AutoNAT provider starts at successful1/failed0 while retaining its reservation. Service shutdown and closed NAT still produce the required unavailable/private results. Cleanup errors are empty and no run-owned containers remain. No diagnostic logging is present in shipped source or the rebuilt application.
