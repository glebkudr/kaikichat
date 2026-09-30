# Required continuation toward network indexes

Scheduling update, 2026-09-10: the [architecture follow-up](../../../Docs/V1_ARCHITECTURE_FOLLOWUP_2026_09_10.md)
places this retained dependency list within AR3, after initial sender/spend lifecycle
work. It does not invalidate the D05 backend evidence or close network/history/R10.

This backend store alone does not make a network index or complete V1.

1. Authenticate data-holder receipts from a verified compact descriptor and the
   same native postage/QC. A recipient needs actual signed custody locations after
   the sender and original admission context expire. Reuse historical custody
   operator/member/primary/copy verification; do not manufacture a PublicEnvelope
   or substitute metadata as ciphertext. Preserve existing data receipt bytes.
2. Retain bounded independently authenticated holder locations alongside the index
   and sender-side index receipts durably. Reuse ProfileStore/CAS and shared evidence;
   exact retries/restart and SQL commit failures must not create false success.
   Index receipt promises compact metadata; each data-holder receipt independently
   promises ciphertext. A response or cached replica count is neither delivery nor
   global history completeness.
3. Add index put/read on the existing authenticated, bounded paid-custody swarm
   protocol and integrate the public book index placement into the shared operator
   resolver. Receiver checks actual selected operator/transport before admission;
   sender persists and verifies index receipts before reporting storage.
4. Extend ordinary public_sender work to the stable index roster; publish those
   index routes instead of intersecting per-message data-holder sets. Automatic
   recipient reads index pages, verifies descriptor/paid proof/holder receipts,
   resolves actual holder routes and receives the exact envelope through Core.
   Test multiple distinct per-ticket data rosters, sender absent, source loss,
   restarts, denied reads, corrupt responses and real SQL failures on real daemons.
5. Complete-range/gap evidence, book/registry/MLS epoch handover, autonomous data and
   index repair (R10, 10-to-7-to-10), expired terminal queue cleanup and remaining
   current V1 cards/E2E remain mandatory. Keep parked live 64-validator/R24 gate
   parked; it is not required to implement or test this index module.

Tests first; separate no-context backend-test-critic ACCEPT before production.
