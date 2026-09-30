#![allow(clippy::unwrap_used, clippy::expect_used)]

use agentic_protocol::{DocumentDraft, DocumentKind, SignedDocument, VerifiedDocument, WireError};
use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use serde_json::Value;
use std::collections::BTreeMap;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/wire/signed-document-v1.json"
    ))
    .unwrap()
}
fn fixture_bytes(field: &str) -> Vec<u8> {
    hex::decode(fixture()[field].as_str().unwrap()).unwrap()
}
fn variant(name: &str) -> Vec<u8> {
    hex::decode(fixture()["variants"][name]["wire_hex"].as_str().unwrap()).unwrap()
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&fixture_bytes("seed_hex").try_into().unwrap())
}
fn draft() -> DocumentDraft {
    DocumentDraft {
        domain: [0; 32],
        kind: DocumentKind::Message,
        authority_epoch: 0,
        issued_at: 100,
        expires_at: Some(200),
        body: b"hello".to_vec(),
        extensions: BTreeMap::new(),
    }
}

#[test]
fn verifies_an_independently_signed_peer_document() {
    let doc = VerifiedDocument::decode(&fixture_bytes("wire_hex"), [0; 32], 150).unwrap();
    assert_eq!(doc.author().as_slice(), fixture_bytes("author_hex"));
    assert_eq!(doc.body(), b"hello");
    assert_eq!(doc.kind(), DocumentKind::Message);
    assert_eq!(doc.authority_epoch(), 0);
    assert_eq!(doc.issued_at(), 100);
    assert_eq!(doc.expires_at(), Some(200));
    assert_eq!(hex::encode(doc.id()), fixture()["id_hex"].as_str().unwrap());
}

#[test]
fn signing_matches_the_independent_wire_and_identity_vector() {
    let wire = SignedDocument::sign(draft(), &key()).unwrap().to_wire();
    assert_eq!(wire, fixture_bytes("wire_hex"));
}

#[test]
fn a_valid_signature_from_another_network_is_not_admitted() {
    let error = VerifiedDocument::decode(&fixture_bytes("wire_hex"), [1; 32], 150).unwrap_err();
    assert!(matches!(error, WireError::WrongDomain));
}

#[test]
fn altered_payload_is_not_authenticated() {
    let mut wire = fixture_bytes("wire_hex");
    let offset = wire.windows(5).position(|bytes| bytes == b"hello").unwrap();
    wire[offset] = b'j';
    assert!(matches!(
        VerifiedDocument::decode(&wire, [0; 32], 150),
        Err(WireError::InvalidSignature)
    ));
}

#[test]
fn replacing_the_signature_cannot_impersonate_an_author() {
    let mut wire = fixture_bytes("wire_hex");
    let last = wire.len() - 1;
    wire[last] ^= 1;
    assert!(matches!(
        VerifiedDocument::decode(&wire, [0; 32], 150),
        Err(WireError::InvalidSignature)
    ));
}

#[test]
fn signed_authority_metadata_cannot_be_relabelled() {
    // Offsets are fixed by the independently authored CBOR fixture, not the Rust encoder.
    for (field, unsigned_offset, replacement) in [
        ("kind", 36, 4),
        ("authority_epoch", 71, 1),
        ("author", 39, 0),
    ] {
        let mut wire = fixture_bytes("wire_hex");
        wire[3 + unsigned_offset] = replacement;
        assert!(
            matches!(
                VerifiedDocument::decode(&wire, [0; 32], 150),
                Err(WireError::InvalidSignature)
            ),
            "metadata {field}"
        );
    }
}

#[test]
fn changed_network_bytes_are_still_bound_to_the_signature() {
    let mut wire = fixture_bytes("wire_hex");
    wire[6] = 1;
    let mut changed_domain = [0; 32];
    changed_domain[0] = 1;
    assert!(matches!(
        VerifiedDocument::decode(&wire, changed_domain, 150),
        Err(WireError::InvalidSignature)
    ));
}

#[test]
fn weak_key_universal_forgery_is_rejected_by_strict_verification() {
    let vector = &fixture()["variants"]["weak_key_forgery"];
    let mut identity_point = [0; 32];
    identity_point[0] = 1;
    let public = VerifyingKey::from_bytes(&identity_point).unwrap();
    let signature_bytes: [u8; 64] = hex::decode(vector["signature_hex"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let signature = Signature::from_bytes(&signature_bytes);
    let mut message = b"AgenticInternet/signed-document/v1\0".to_vec();
    message.extend(hex::decode(vector["unsigned_hex"].as_str().unwrap()).unwrap());
    assert!(
        public.verify(&message, &signature).is_ok(),
        "fixture must distinguish lax verification"
    );
    assert!(matches!(
        VerifiedDocument::decode(&variant("weak_key_forgery"), [0; 32], 150),
        Err(WireError::InvalidSignature)
    ));
}

#[test]
fn live_admission_expires_at_the_declared_boundary() {
    let wire = fixture_bytes("wire_hex");
    assert!(VerifiedDocument::decode(&wire, [0; 32], 199).is_ok());
    assert!(matches!(
        VerifiedDocument::decode(&wire, [0; 32], 200),
        Err(WireError::Expired)
    ));
}

#[test]
fn future_dated_document_is_rejected_with_a_bounded_clock_tolerance() {
    let wire = fixture_bytes("wire_hex");
    assert!(VerifiedDocument::decode(&wire, [0; 32], 70).is_ok());
    assert!(matches!(
        VerifiedDocument::decode(&wire, [0; 32], 69),
        Err(WireError::NotYetValid)
    ));
}

#[test]
fn unknown_optional_extensions_are_preserved_and_signed() {
    let mut input = draft();
    input.extensions.insert(42, b"relay-hint".to_vec());
    let wire = SignedDocument::sign(input, &key()).unwrap().to_wire();
    let verified = VerifiedDocument::decode(&wire, [0; 32], 150).unwrap();
    assert_eq!(verified.extensions().get(&42).unwrap(), b"relay-hint");
    let mut tampered = wire;
    let offset = tampered
        .windows(10)
        .position(|part| part == b"relay-hint")
        .unwrap();
    tampered[offset] ^= 1;
    assert!(matches!(
        VerifiedDocument::decode(&tampered, [0; 32], 150),
        Err(WireError::InvalidSignature)
    ));
}

#[test]
fn unknown_critical_extension_is_not_silently_ignored() {
    assert!(matches!(
        VerifiedDocument::decode(&variant("critical_extension"), [0; 32], 150),
        Err(WireError::UnsupportedCriticalExtension(32768))
    ));
}

#[test]
fn duplicate_extension_keys_cannot_create_two_interpretations() {
    assert!(matches!(
        VerifiedDocument::decode(&variant("duplicate_extension"), [0; 32], 150),
        Err(WireError::NonCanonical)
    ));
}

#[test]
fn even_a_correct_signature_does_not_allow_nonminimal_cbor() {
    assert!(matches!(
        VerifiedDocument::decode(&variant("nonminimal_version"), [0; 32], 150),
        Err(WireError::NonCanonical)
    ));
}

#[test]
fn alternate_containers_cannot_create_multiple_ids_for_one_signed_document() {
    for name in [
        "nonminimal_outer_array",
        "nonminimal_outer_unsigned_length",
        "nonminimal_outer_signature_length",
        "indefinite_outer",
        "indefinite_inner",
        "out_of_order_extensions",
    ] {
        assert!(
            matches!(
                VerifiedDocument::decode(&variant(name), [0; 32], 150),
                Err(WireError::NonCanonical)
            ),
            "accepted {name}"
        );
    }
}

#[test]
fn unknown_protocol_and_message_kind_are_explicit_errors() {
    assert!(matches!(
        VerifiedDocument::decode(&variant("future_version"), [0; 32], 150),
        Err(WireError::UnsupportedVersion(2))
    ));
    assert!(matches!(
        VerifiedDocument::decode(&variant("unknown_kind"), [0; 32], 150),
        Err(WireError::UnsupportedKind(99))
    ));
}

#[test]
fn truncated_or_concatenated_network_records_are_never_accepted() {
    let wire = fixture_bytes("wire_hex");
    for len in [0, 1, 2, 32, wire.len() / 2, wire.len() - 1] {
        assert!(
            VerifiedDocument::decode(&wire[..len], [0; 32], 150).is_err(),
            "accepted truncated length {len}"
        );
    }
    let mut concatenated = wire.clone();
    concatenated.extend_from_slice(&wire);
    assert!(VerifiedDocument::decode(&concatenated, [0; 32], 150).is_err());
}

#[test]
fn resource_limits_apply_to_both_sending_and_receiving() {
    let mut input = draft();
    input.body = vec![7; 49_152];
    let wire = SignedDocument::sign(input.clone(), &key())
        .unwrap()
        .to_wire();
    assert!(VerifiedDocument::decode(&wire, [0; 32], 150).is_ok());
    input.body.push(7);
    assert!(matches!(
        SignedDocument::sign(input, &key()),
        Err(WireError::TooLarge)
    ));
    assert!(matches!(
        VerifiedDocument::decode(&vec![0; 65_537], [0; 32], 150),
        Err(WireError::TooLarge)
    ));
    assert!(matches!(
        VerifiedDocument::decode(&variant("oversized_body"), [0; 32], 150),
        Err(WireError::TooLarge)
    ));
}

#[test]
fn extension_limits_are_enforced_on_untrusted_peers_and_local_senders() {
    let allowed = VerifiedDocument::decode(&variant("extensions_at_limit"), [0; 32], 150).unwrap();
    assert_eq!(allowed.extensions().len(), 16);
    for id in 0u16..16 {
        assert_eq!(allowed.extensions()[&id], vec![id as u8; 1024]);
    }
    let mut input = draft();
    input.extensions = allowed.extensions().clone();
    assert_eq!(
        SignedDocument::sign(input.clone(), &key())
            .unwrap()
            .to_wire(),
        variant("extensions_at_limit")
    );
    input.extensions.insert(16, vec![1]);
    assert!(matches!(
        SignedDocument::sign(input, &key()),
        Err(WireError::TooLarge)
    ));
    let mut input = draft();
    input.extensions.insert(1, vec![2; 1025]);
    assert!(matches!(
        SignedDocument::sign(input, &key()),
        Err(WireError::TooLarge)
    ));
    for name in ["too_many_extensions", "oversized_extension"] {
        assert!(
            matches!(
                VerifiedDocument::decode(&variant(name), [0; 32], 150),
                Err(WireError::TooLarge)
            ),
            "accepted {name}"
        );
    }
}

#[test]
fn total_wire_budget_and_critical_extensions_also_apply_before_sending() {
    let mut input = draft();
    input.body = vec![1; 49_152];
    for id in 0..16 {
        input.extensions.insert(id, vec![2; 1024]);
    }
    assert!(matches!(
        SignedDocument::sign(input, &key()),
        Err(WireError::TooLarge)
    ));
    let mut input = draft();
    input.extensions.insert(32768, vec![1]);
    assert!(matches!(
        SignedDocument::sign(input, &key()),
        Err(WireError::UnsupportedCriticalExtension(32768))
    ));
}

#[test]
fn realistic_unix_timestamps_and_nonzero_authority_epoch_are_interoperable() {
    let doc = VerifiedDocument::decode(&variant("modern_epoch"), [0; 32], 1_788_480_030).unwrap();
    assert_eq!(doc.authority_epoch(), 7);
    assert_eq!(doc.issued_at(), 1_788_480_000);
    assert_eq!(doc.expires_at(), Some(1_788_483_600));
}

#[test]
fn huge_declared_length_in_a_tiny_packet_is_rejected() {
    assert!(VerifiedDocument::decode(&[0x82, 0x5a, 0xff, 0xff, 0xff, 0xff], [0; 32], 150).is_err());
}

#[test]
fn no_expiry_identity_is_still_authenticated_and_subject_to_domain_rules() {
    let mut input = draft();
    input.kind = DocumentKind::Identity;
    input.expires_at = None;
    let wire = SignedDocument::sign(input, &key()).unwrap().to_wire();
    let result = VerifiedDocument::decode(&wire, [0; 32], u64::MAX).unwrap();
    assert_eq!(result.kind(), DocumentKind::Identity);
    assert_eq!(result.expires_at(), None);
    assert!(matches!(
        VerifiedDocument::decode(&wire, [1; 32], u64::MAX),
        Err(WireError::WrongDomain)
    ));
}

#[test]
fn sender_cannot_create_an_already_invalid_lifetime() {
    let mut input = draft();
    input.expires_at = Some(input.issued_at);
    assert!(matches!(
        SignedDocument::sign(input, &key()),
        Err(WireError::InvalidLifetime)
    ));
    assert!(matches!(
        VerifiedDocument::decode(&variant("invalid_lifetime"), [0; 32], 150),
        Err(WireError::InvalidLifetime)
    ));
}
