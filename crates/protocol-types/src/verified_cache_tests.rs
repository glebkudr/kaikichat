//! A document's Ed25519 check depends only on its exact canonical wire, which
//! the SHA-256 document id covers (unsigned bytes with the author key, and the
//! signature). Re-decoding an identical wire must not repeat that check, while
//! every domain, time and canonical-encoding check still runs on each decode.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use std::sync::Mutex;

/// Decodes in this binary share one process-wide cache; the eviction test
/// fills it, so every test here runs alone.
static SERIAL: Mutex<()> = Mutex::new(());

fn checks() -> u64 {
    SIGNATURE_CHECKS.with(std::cell::Cell::get)
}

fn draft(domain: [u8; 32], body: &[u8]) -> DocumentDraft {
    DocumentDraft {
        domain,
        kind: DocumentKind::Message,
        authority_epoch: 0,
        issued_at: 1_000,
        expires_at: Some(2_000),
        body: body.to_vec(),
        extensions: BTreeMap::new(),
    }
}

fn document(seed: u8, body: &[u8]) -> Vec<u8> {
    SignedDocument::sign(draft([7; 32], body), &SigningKey::from_bytes(&[seed; 32]))
        .unwrap()
        .to_wire()
}

/// A canonical wire carrying `signature` over different unsigned bytes.
fn relabeled(draft: &DocumentDraft, author: [u8; 32], signature: [u8; 64]) -> Vec<u8> {
    encode_outer(&encode_unsigned(draft, &author).unwrap(), &signature).unwrap()
}

#[test]
fn an_identical_wire_is_not_signature_checked_twice() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let wire = document(1, b"first");
    let before = checks();
    let first = VerifiedDocument::decode(&wire, [7; 32], 1_500).unwrap();
    assert_eq!(checks(), before + 1);
    let again = VerifiedDocument::decode(&wire, [7; 32], 1_600).unwrap();
    assert_eq!(checks(), before + 1, "the same wire was already verified");
    assert_eq!(
        (again.id(), again.author(), again.body()),
        (first.id(), first.author(), first.body())
    );
}

#[test]
fn a_remembered_wire_still_fails_domain_and_time_checks() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let wire = document(2, b"second");
    VerifiedDocument::decode(&wire, [7; 32], 1_500).unwrap();
    let before = checks();
    assert_eq!(
        VerifiedDocument::decode(&wire, [8; 32], 1_500).unwrap_err(),
        WireError::WrongDomain
    );
    assert_eq!(
        VerifiedDocument::decode(&wire, [7; 32], 2_000).unwrap_err(),
        WireError::Expired
    );
    assert_eq!(
        VerifiedDocument::decode(&wire, [7; 32], 900).unwrap_err(),
        WireError::NotYetValid
    );
    assert_eq!(checks(), before);
}

#[test]
fn a_bad_signature_is_checked_on_every_decode_and_never_remembered() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let good = document(3, b"third");
    VerifiedDocument::decode(&good, [7; 32], 1_500).unwrap();
    // The last byte belongs to the signature; the altered wire stays canonical.
    let mut forged = good.clone();
    *forged.last_mut().unwrap() ^= 1;
    let before = checks();
    for _ in 0..2 {
        assert_eq!(
            VerifiedDocument::decode(&forged, [7; 32], 1_500).unwrap_err(),
            WireError::InvalidSignature
        );
    }
    assert_eq!(checks(), before + 2);
    // The genuine wire remains accepted without a new check.
    VerifiedDocument::decode(&good, [7; 32], 1_500).unwrap();
    assert_eq!(checks(), before + 2);
}

#[test]
fn a_remembered_signature_does_not_cover_other_unsigned_bytes() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let key = SigningKey::from_bytes(&[6; 32]);
    let good = SignedDocument::sign(draft([7; 32], b"sixth"), &key)
        .unwrap()
        .to_wire();
    VerifiedDocument::decode(&good, [7; 32], 1_500).unwrap();
    let signature: [u8; 64] = good[good.len() - 64..].try_into().unwrap();
    let author = key.verifying_key().to_bytes();
    let other_author = SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes();
    let before = checks();
    // Same signature and author, another network's domain.
    let other_domain = relabeled(&draft([8; 32], b"sixth"), author, signature);
    assert_eq!(
        VerifiedDocument::decode(&other_domain, [8; 32], 1_500).unwrap_err(),
        WireError::InvalidSignature
    );
    // Same signature and body, another valid author key.
    let other_key = relabeled(&draft([7; 32], b"sixth"), other_author, signature);
    assert_eq!(
        VerifiedDocument::decode(&other_key, [7; 32], 1_500).unwrap_err(),
        WireError::InvalidSignature
    );
    assert_eq!(checks(), before + 2);
}

#[test]
fn a_remembered_wire_is_shared_by_every_thread_of_the_process() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let wire = document(10, b"shared");
    VerifiedDocument::decode(&wire, [7; 32], 1_500).unwrap();
    let elsewhere = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let before = checks();
                VerifiedDocument::decode(&wire, [7; 32], 1_500).unwrap();
                checks() - before
            })
            .join()
            .unwrap()
    });
    assert_eq!(elsewhere, 0, "another runtime thread reuses the same check");
}

#[test]
fn exactly_the_bound_of_remembered_wires_survives_and_the_oldest_is_checked_again() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let oldest = document(4, b"oldest");
    VerifiedDocument::decode(&oldest, [7; 32], 1_500).unwrap();
    let newer: Vec<_> = (0..VERIFIED_DOCUMENTS)
        .map(|n| document(5, &n.to_be_bytes()))
        .collect();
    for wire in &newer {
        VerifiedDocument::decode(wire, [7; 32], 1_500).unwrap();
    }
    let before = checks();
    // The earliest of the last VERIFIED_DOCUMENTS wires is still remembered.
    VerifiedDocument::decode(&newer[0], [7; 32], 1_500).unwrap();
    assert_eq!(checks(), before, "the bound keeps every one of its wires");
    VerifiedDocument::decode(&oldest, [7; 32], 1_500).unwrap();
    assert_eq!(checks(), before + 1, "the bound evicted the oldest wire");
}
