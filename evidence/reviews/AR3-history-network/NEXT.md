# Ordinary sender and recipient manifest integration

1. Persist exact manifest publication/ACK state and derive candidate routes from
   genuine outgoing paid index receipts with confirmed holder locations. Every
   still-live previously declared reference must survive later messages, cold
   restart and eventual sender-job retirement. Choose the longest-lived anchor;
   shorter newer messages cannot truncate older retention. Acknowledgments must
   match the actual responding peer, operation and exact current manifest hash.
   Publish a pointer only after required anchor-index acknowledgments.
2. Ordinary recipient lookup must use its committed pointer's exact operation to
   request the paid anchor/directory, verify the actual Noise index and original
   funding/QC/lease, and commit it through Core's pointer-bound acceptance. Then
   independently route each pending reference through listed candidate indexes
   and existing verified holder lookup/fetch. Reuse atomic per-reference import.
   Do not turn missing manifests or expired/unavailable references into success.
3. Replace live-job index-roster intersection only with those ordinary paths in
   place. Prove genuinely funded disjoint books with no common index, sender
   absent, cold clients/providers, missing/corrupt bytes, actual SQL failures and
   expiry on one frozen application build. Existing raw-peer server acceptance
   is a prerequisite, not this automatic multi-book gate.
4. Complete real MLS control/epoch and first-Welcome delivery, safe successful
   sender retirement, attachments, historical-row lifecycle and autonomous
   index/data 10→7→10 repair. Preserve all AR1–AR5 work and 67 mandatory cards /
   22 E2E /three platforms. Full suite only at the end of the V1 plan. No push.
