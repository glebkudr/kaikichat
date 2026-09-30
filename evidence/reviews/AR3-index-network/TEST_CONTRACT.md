# Native index transport and recipient location pages

Tests precede production. The two library scenarios use existing genuine native
funding/finality, primary admission, opted-in copy receipts and SQLCipher helpers.
They cover sorted count/byte pages and exact boundaries, cold historical reads
without wallet state, target/signature/expiry/mailbox/epoch/sequence scope,
unconfigured Core trust, failed read-clock commits and cold successful retry.

The native gate funds a fresh random book on real local EVM contracts and creates
an actual MLS message. Real selected daemons finalize its QC. An independently
drawn book-index member outside the message's primary data roster stores only the
index; actual data primary and replacement daemons store the original ciphertext.
Independent raw Noise peers send the new protocol requests. Missing publication,
wrong owned position, corrupt descriptor/QC, substituted holder key and bad
recipient capability have valid companions. Real SQL triggers block first index
admission, location persistence and read clocks. Exact retries preserve receipt
bytes. After source loss and cold index/replica restart, bounded authorized pages
return exactly the original and replacement claims, then the daemon retrieves and
decrypts the original MLS message from the copy. The sender wallet is reopened
only after this gate to check that the same one ticket paid index/data/copy work.

The wire oracle independently checks QC signatures, native sender signature,
funding equality, index roster selection, operator/transport signatures,
descriptor and journal hashes, and exact original/copy receipts. It does not
reimplement the index evidence digest's Rust serde field order; complete cold
index verification is also exercised by the library-backed location read path.

No automatic index publisher, resolver, recipient index importer, complete
history chain, R10 or successful sender retirement is claimed. No full workspace
or release-platform suite belongs to this affected-cluster gate.

The first real round trip exposed a shared-fixture assumption: independently
configured daemons record different valid authority observation times. A third
library test obtains both snapshots through Core, preserves the exact same proof
fields and checks future-time refusal, unchanged SQL, retry and cold canonical
claim verification. It failed with `Conflict` before the correction; the
independent critic accepted it before the proof-equality change. Both incoming
and retained snapshots are authenticated, and the rebuilt claim is checked before
storage. Candidate 2 then completed the offline workflow but used an obsolete
checkpoint for its final wallet audit. That test-only correction reuses the
existing current-context query helper and was independently accepted too.
