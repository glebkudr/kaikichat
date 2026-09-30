# User amendment of September 5, 2026

Plan version 1.1 updates the version 1.0 documents. This is an additional message outside the 27 messages of the original archive; their Uxx/Axx numbering is preserved.

## Verbatim remarks

> - Let's have Tauri do the UI
> - a limited number of Google attestors -- okay. Trusted networks can be relayed through attestation. In the first version this can be the government organization itself/oauth on the site
> - by the way, I would also like to add Telegram authorization
>
> In essence, only the executor can attest the result. Perhaps, to start, we simply need a public rating mechanism here, i.e., for an ordered service the customer has the right to write a review. This is the level of a basic service order protocol (public reviews), while complex economic agents on top of that are already v2 and beyond.
>
> update the documents per these remarks

## Normalization for traceability

| ID | Decision | Requirement linkage |
|---|---|---|
| AMENDMENT-01 | Tauri instead of the proposed egui/eframe; the Rust daemon is preserved | R02, R35, R45 |
| AMENDMENT-02 | Generalized attestation of trusted networks; a limited set and an organizational issuer/site OAuth are acceptable | R01, R25–R27, R44, R46 |
| AMENDMENT-03 | Telegram authorization is included in V1 alongside Google | R25–R27, R47 |
| AMENDMENT-04 | The executor declares the result; the client gets a public review/rating for their order at the protocol level | R19, R21–R22, R30, R35, R42, R48–R51 |
| AMENDMENT-05 | Complex economic agents are V2+; the basic order and review are V1 | R20, R23, R52 |

A government organization is an acceptable example of an explicitly trusted issuer, not a claim that a specific government system is connected. No unnamed provider is considered already integrated. Allowing a single issuer is treated as a limited voluntary trust-profile, not as a change to the decentralized transport requirement.

The protocol refinements of this update — the two-sided receipt, the absence of veto after order acceptance, 1–5 stars, amendments/replies, and deterministic rating — are design decisions for implementing the wish, not verbatim quotes from the user.
