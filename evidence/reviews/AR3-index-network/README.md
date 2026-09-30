# Accepted native index server and recipient location pages

The real funded gate passes on the same source/binary as the affected checks:
**27 backend tests (19 paid-index, one public historical custody, one production
custody codec and six Core postage-client tests), 21 frontend chat/history tests,
production Clippy and fmt; 580 unchanged input fingerprints.** No full workspace
suite or new platform release is claimed.

The native scenario completed in 54.35 seconds with **three independently
authenticated QC signatures, 522 owner calls, zero proof-worker invocations and
no cleanup errors**. Actual MLS ciphertext, original paid native funding and a
selected replacement copy are used. The book-index host is outside the message's
primary data roster and stores no ciphertext. After the sender and primary stop
and the index/copy restart, recipient capabilities recover exact bounded index
and location pages. The existing daemon retrieves and decrypts the original MLS
message from the copy. Final wallet audit confirms one allocated ticket for all
index/data/copy work. This is owner/raw Noise peer driven, not automatic discovery.

Independent critics accepted the tests before production and the later observer
correction before changing snapshot comparison. Original index evidence remains
immutable. Differing valid local snapshot observation times are allowed only with
identical certificate/issuer/roster proofs, full authentication of both snapshots
and verification of the claim with the retained common proof before commit.

Preserved development results:

- Library API-absent baseline: 14 missing-method compile errors.
- Real native baseline: existing read handled; new index request streams fail.
- Candidate 1: real admission exposed independent snapshot observation times.
  New genuine-Core observer test reproduced `Conflict` before its fix.
- Candidate 2: offline index/copy/message workflow passed, but final test wallet
  audit used an obsolete checkpoint. The accepted test correction reuses the
  existing current-context query helper.
- Candidate 3: complete native workflow and 26 backend/21 frontend tests passed;
  the runner rejected an accidentally empty node binary test selection.
  Correcting the selection to the library runs its custody codec test. Other
  successful checks were reused only after matching all application/test input
  hashes, exact commands and original log hashes; the native source and binary
  were also matched again. No empty selection is counted as a passing test.

`checks.json`, `native.json`, `native-integrity.json` and `source-inputs.json` are
the accepted evidence. Candidate reports are diagnostic, not additive gates.
Raw logs and trace remain under ignored `output/`. Reproduce via:

```sh
python3 scripts/build-storage.py run python3 evidence/reviews/AR3-index-network/verify.py --native
```

The [contract](../../../spec/paid-index-network-v1.md) and [next steps](NEXT.md)
keep automatic publication/discovery, book/epoch completeness, sender retirement,
R10 and the full V1 goal open.
