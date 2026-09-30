# AR3 compact holder-location verifier

The index needs authenticated data-holder claims without retaining ciphertext
or accessing the sender's private postage wallet. Original finite custody
receipts/QCs, operator selection and copied-obligation consent must remain the
source of authority. This slice adds verification only, not storage or networking.

Tests were written before production. The independent backend-test critic ran
without inherited context. First verdict REVISE required missing native-proof,
membership and historical-copy-consent scenarios. Revised tests received ACCEPT
before implementation. Both baselines were compilation RED because the proposed
API did not exist; no pre-implementation runtime failure is claimed.

Four tests use genuine native paid fixtures, actual P-256 finality certificates,
SQLCipher Core/CustodyStore admission and real operator/transport signatures:

1. Original receipt verification after admission and binding expiry, on a cold
   configured reader with no sender wallet; exact receipt fields and fetched
   envelope binding checked independently.
2. Refusal of another signed descriptor, another record/native spend, another
   genuine operator, wrong transport, altered QC and a genuinely signed false
   envelope-size claim. The unchanged original remains accepted.
3. Actual opted-in replacement copy admitted after original admission expiry,
   then cold verification after copy consent expiry but before retention expiry.
   Missing/damaged original evidence, valid other membership and a genuinely
   signed wrong-origin consent with a matching new receipt digest are refused.
4. Installed trust, live retention and monotonic reader time remain mandatory.

Shared fixture key restoration, signed-document mutation and consent mutation
were extracted from existing paid-custody tests without replacing real Core
publication, opt-in or custody admission. Two existing cases passed separately
before production: original-primary copy retry and semantic consent mutations.

Targeted regression covers paid index/custody integration, the native public
historical ciphertext path, matching Clippy, formatting and frontend chat history.
Full workspace and native end-to-end network verification are not this gate.
