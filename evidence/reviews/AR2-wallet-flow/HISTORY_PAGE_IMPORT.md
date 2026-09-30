# Core history page imports — in progress, required recovery gate fails

Core now accepts an incoming signed v2 root only under its exact current mailbox
pointer and MLS scope. Every read carries an authenticated bounded path to its
leaf, and descriptor/fetch completion repeats the current-root fence. Imported
operations and sequence claims use separate rows, committed atomically with the
actual message, MLS and dedup state. The ordinary mailbox publication API can
bind the exact current outgoing v2 root; the ordinary network workers still
exchange v1 flat manifests.

Eight new scenarios pass: root/pointer/SQL fences, malformed or substituted paths,
operation/sequence/message rollback, cold distinct imports, exact live v1
wrapping, durable semantic corruption, real epoch changes, signed non-extensions
and reference expiry. These eight tests contain multiple cases; the case labels
are not additional independent tests. Independent backend review accepted the
tests before production, including revised full message-table corruption oracles.

The ninth required test **fails**. All 130 genuine encrypted originals are live
in one epoch. The recipient commits arrival offsets `1..129` successfully, then
offset `0` fails with `Crypto(Mls)`. This reproduces review R19: the existing
128-generation sender window has evicted its key. Neither a valid signed history
graph nor retained ciphertext establishes that the original can still decrypt.
The three-past-epoch limit is not involved in this reproduction.

[Checks](history-page-import-checks.json) record **69 passing backend cases,
one failing backend case and 31 passing frontend tests**, Core all-target Clippy
and formatting. All 793 captured inputs stayed unchanged throughout the run.
The known failure ran separately; the regression command explicitly skipped it
only to collect the other results. The aggregate is failed. A prior command with
the wrong filter ran zero tests and is not acceptance evidence. No native run or
GUI acceptance was renewed by this work.

Keep this failure as a required recovery gate. The next work is an explicit
bounded MLS admission/cursor policy with durable pending progress, fresh bounded
refetch, and the same guard on direct delivery. The incoming root must not be
mistaken for a completeness cursor. The staged
[contiguous receive prerequisite](../../../spec/mls-contiguous-receive-v1.md)
can prevent new forward skips but cannot protect gaps that predate activation or
restore deleted keys. Its eventual integration must preserve the original
`1..129,0` arrival scenario and final recovery of every exact original, while
making deferred arrivals observable. Do not merely reorder the test or enlarge
the key window. Epoch/control-log/rejoin outcomes, ordinary graph workers, paid
over-128 native recovery and the full 67-card/22-E2E/three-platform V1 remain open.

The [contiguous MLS prerequisite](MLS_CONTIGUOUS.md) now passes 21 MLS scenarios.
The final run records 90 backend PASS /1 FAIL /31 frontend PASS, Clippy/fmt and
794 unchanged inputs. The original Core recovery failure remains; durable
admission/cursor, direct-path fencing and the pre-existing-gap transition are next.
