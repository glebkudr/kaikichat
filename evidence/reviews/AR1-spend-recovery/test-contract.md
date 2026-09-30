# Recovery of original historical spend evidence

Baseline a9fbad3. This extends AR1; V1 remains 67 cards / 22 E2E / three platforms.

A selected validator offline before receipt distribution must recover its actual
archive delivery after other replicas finalize and retire all active inputs.
Recovery binds an independently verified original SpendRecord to the entire exact
entry (committee, height, parent, nullifier, journal), commits without replacing a
known spent fact, and preserves the first QC and verification time. It neither
creates new spending authority nor fabricates a receipt. Existing historical
authority and record verification are reused.

Proposed implementation: retain at most one deferred Delivery per registered
service; no consumer ACK until record, history and retirement are durable. Other
effect failures still revoke/restart. Reuse the existing bounded submissions
codec/record response with an explicit optional selected-replica lookup field;
ordinary requests retain their existing wire shape. Authenticate selected peer,
current scope/transport permit, exact requested target and historical QC locally.
At most one recovery request is outstanding within the existing four-request
budget; retries rotate through selected routes. Revocation cancels the held
delivery and transport permit; restart derives the target again from the archive.

Tests written before production:

- Three Rust tests use actual public funded fixture receipts, real P-256 QCs,
  independently installed historical authority and SQLCipher. Atomic SQL failure,
  cold lookup, alternate valid QC idempotency, conflicting valid remote spend,
  wrong target committee/height/parent/key/journal and damaged evidence all have
  durable state assertions. Historical import works after expiry without granting
  current authority. Fixture signers are explicit; these are not live quorum tests.
- The native funded gate uses self-generated keys in four ordinary daemons. One
  is offline before the first owner submission; the other three retire all
  inputs. The lagger must observe an actual failed/deferred archive delivery
  while a real SQL insertion trigger prevents commitment. After a crash with
  cursor zero and an isolated restart, just one source returns. The lagger must
  recover the exact original record without receipt/owner resubmission or a new
  quorum, durably ACK, cold-read alone, and then participate in the next spend.
  One daemon binary is pinned for the full run. Public trace retains original
  records and the blocked delivery/cursor observations.

R1 revision: the shared harness's two-case competing-ticket assumption is now an
explicit defaulted option; this exercise uses two distinct tickets and skips the
shared rejected-receipt probe so the offline lagger is never given that input.
The first failed native attempt stopped in this harness assertion, not recovery.
The revised gate also revokes the role with a known deferred delivery, removes
the SQL fault while sources remain online, and observes nine seconds (longer
than transport timeout) of no writes, no ACK and no new recovery sends. It then
crashes/re-enables and requires archive replay and exact recovery. Wrong target
tests assert absence under both original and requested keys; alternate/conflicting
QCs are independently checked as positive controls before invoking recovery.

R3 refinements: actual Marshal metadata represents an untouched consumer as an
absent height (`None`) or genesis zero; both mean no application entry ACK and
are accepted without weakening the later required heights 1 and 2. After solo
replay, an independent selected peer reopens a stopped real profile, serves its
genuine binding and an altered original QC. The lagger must reject it without
writing or ACK, then recover from the ordinary source. The existing test carrier
only gains the optional request field and log observation; no production signer
or record verification is used to construct the hostile response.

Execution refinement: the old test carrier's postage mode had no service
announcements. Unlike the existing pinned ordinary-client exercise, a cold
validator cannot infer a remote member key from a Noise connection. The actual
diagnostic run showed only connected, zero routes and zero recovery requests;
it did not exercise hostile evidence. An opt-in postage-recovery response mode
now serves genuine own announcements as well as proofs, while older modes stay
unchanged. The test requires a genuine announcement, the actual authenticated
route matching peer/key/base committee, receipt-free lookup and local rejection.
The executed runtime reused the locally authenticated roster to verify a compact
binding, so no redundant full-proof exchange was needed. Requiring that particular
wire exchange would reject the supported compact path. This changes test
infrastructure only; scope/transport/QC validation remains mandatory.

The existing lifecycle gate covers expiry, generic pre/post-record failure and
terminal candidate cleanup. Existing client record tests cover historical QC
authentication; existing transport tests cover revoked/expired/wrong-peer permits.
New tests do not claim epoch handover or independent operator deployment.
