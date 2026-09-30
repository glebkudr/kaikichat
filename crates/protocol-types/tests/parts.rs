#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Documents too big for one mailbox envelope travel in parts
//! (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 2): the tree of a group
//! of 2000 (488 KB), a removal commit in it (165 KB). Each part is a signed
//! document paid like any message; the reader puts them together and checks
//! the whole, itself a signed document.
use agentic_protocol::parts::{
    Assembly, AssemblyError, MAX_PART_BYTES, MAX_PARTS, MAX_WHOLE_BYTES, Part, reference, split,
    verify_part,
};
use agentic_protocol::{
    DocumentDraft, DocumentKind, MAX_DOCUMENT_BYTES, SignedDocument, VerifiedDocument, WireError,
};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const DOMAIN: [u8; 32] = [7; 32];
const NOW: u64 = 1_790_000_000;
/// The measured ratchet tree of a group of 2000.
const TREE_BYTES: usize = 488_372;

fn key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}

/// Bytes that do not compress or repeat, like a ratchet tree.
fn noise(len: usize, seed: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 32);
    let mut block: [u8; 32] = Sha256::digest([seed]).into();
    while out.len() < len {
        out.extend_from_slice(&block);
        block = Sha256::digest(block).into();
    }
    out.truncate(len);
    out
}

fn draft(body: Vec<u8>) -> DocumentDraft {
    DocumentDraft {
        domain: DOMAIN,
        kind: DocumentKind::GroupControl,
        authority_epoch: 0,
        issued_at: NOW,
        expires_at: None,
        body,
        extensions: BTreeMap::new(),
    }
}

fn big_document(author: &SigningKey, body: Vec<u8>) -> SignedDocument {
    SignedDocument::sign_large(draft(body), author).unwrap()
}

#[test]
fn a_tree_of_a_group_of_2000_travels_in_eight_parts_and_comes_back_whole_once_in_any_order() {
    let author = key(1);
    let body = noise(TREE_BYTES, 1);
    let whole = big_document(&author, body.clone());
    // Too big for one envelope.
    assert_eq!(
        VerifiedDocument::decode(whole.as_wire(), DOMAIN, NOW).unwrap_err(),
        WireError::TooLarge
    );

    let parts = split(&whole, DOMAIN, &author, NOW).unwrap();
    // Each part is one stamp: eight, as the price list says.
    assert_eq!(parts.len(), 8);
    assert!(
        parts
            .iter()
            .all(|p| p.as_wire().len() <= MAX_DOCUMENT_BYTES)
    );
    // What a newcomer is told to look for is what the parts name.
    let (hash, count) = reference(&whole);
    assert_eq!(hash, <[u8; 32]>::from(Sha256::digest(whole.as_wire())));
    assert_eq!(count, 8);

    // Holders return them in any order, some twice, also after the whole
    // is complete.
    let mut arrived: Vec<_> = parts
        .iter()
        .rev()
        .map(|p| verify_part(p.as_wire(), DOMAIN, NOW).unwrap())
        .collect();
    assert!(
        arrived
            .iter()
            .all(|p| p.author == *author.verifying_key().as_bytes()
                && p.part.whole == hash
                && p.part.count == count)
    );
    arrived.insert(3, arrived[5].clone());
    let completing = arrived[8].clone();
    arrived.push(completing);
    arrived.push(arrived[0].clone());
    let mut assembly = Assembly::new(&arrived[0]);
    let mut done = vec![];
    for part in &arrived {
        if let Some(wire) = assembly.add(part).unwrap() {
            done.push(wire);
        }
    }
    assert_eq!(done.len(), 1, "the whole comes out once");
    assert_eq!(done[0], whole.as_wire());
    let verified = VerifiedDocument::decode_large(&done[0], DOMAIN, NOW).unwrap();
    assert_eq!(verified.author(), author.verifying_key().as_bytes());
    assert_eq!(verified.kind(), DocumentKind::GroupControl);
    assert_eq!(verified.body(), body.as_slice());
}

#[test]
fn parts_of_someone_else_that_name_the_same_whole_do_not_mix_in() {
    let author = key(1);
    let whole = big_document(&author, noise(165_000, 2));
    let parts: Vec<_> = split(&whole, DOMAIN, &author, NOW)
        .unwrap()
        .iter()
        .map(|p| verify_part(p.as_wire(), DOMAIN, NOW).unwrap())
        .collect();
    let first = &parts[0].part;

    // Another writer of the same mailbox pays for a part naming the same
    // whole, at an index the author also sends, with garbage in it.
    let intruder = Part {
        whole: first.whole,
        index: 1,
        count: first.count,
        bytes: noise(first.bytes.len(), 9),
    }
    .sign(DOMAIN, &key(2), NOW)
    .unwrap();
    let intruder = verify_part(intruder.as_wire(), DOMAIN, NOW).unwrap();

    let mut assembly = Assembly::new(&parts[0]);
    assert_eq!(assembly.add(&intruder), Err(AssemblyError::Foreign));
    let mut out = None;
    for part in &parts {
        out = out.or(assembly.add(part).unwrap());
    }
    assert_eq!(out.unwrap(), whole.as_wire());
}

/// A writer of the mailbox cannot pass garbage off as someone else's part.
#[test]
fn a_part_changed_on_the_way_is_not_taken() {
    let author = key(1);
    let whole = big_document(&author, noise(165_000, 3));
    let parts = split(&whole, DOMAIN, &author, NOW).unwrap();
    let honest = verify_part(parts[1].as_wire(), DOMAIN, NOW).unwrap();
    let mut wire = parts[1].to_wire();
    let at = wire
        .windows(32)
        .position(|w| w == &honest.part.bytes[..32])
        .unwrap();
    wire[at] ^= 1;
    assert_eq!(
        verify_part(&wire, DOMAIN, NOW).unwrap_err(),
        WireError::InvalidSignature
    );
}

#[test]
fn parts_that_do_not_add_up_to_the_whole_they_name_give_nothing() {
    let author = key(1);
    let named = Sha256::digest(b"what the author claims").into();
    let parts: Vec<_> = (0..3u16)
        .map(|index| {
            let signed = Part {
                whole: named,
                index,
                count: 3,
                bytes: noise(40_000, index as u8),
            }
            .sign(DOMAIN, &author, NOW)
            .unwrap();
            verify_part(signed.as_wire(), DOMAIN, NOW).unwrap()
        })
        .collect();
    let mut assembly = Assembly::new(&parts[0]);
    assert_eq!(assembly.add(&parts[0]), Ok(None));
    assert_eq!(assembly.add(&parts[1]), Ok(None));
    assert_eq!(assembly.add(&parts[2]), Err(AssemblyError::Mismatch));
}

#[test]
fn a_part_whose_place_cannot_be_is_refused_and_a_whole_beyond_the_limit_is_not_made() {
    let author = key(1);
    let whole = Sha256::digest(b"any").into();
    for (index, count) in [(3, 3), (0, 0), (0, MAX_PARTS as u16 + 1)] {
        let signed = Part {
            whole,
            index,
            count,
            bytes: vec![1; 100],
        }
        .sign(DOMAIN, &author, NOW)
        .unwrap();
        assert!(
            verify_part(signed.as_wire(), DOMAIN, NOW).is_err(),
            "part {index} of {count} was taken"
        );
    }
    assert_eq!(
        SignedDocument::sign_large(draft(vec![0; MAX_WHOLE_BYTES]), &author).unwrap_err(),
        WireError::TooLarge
    );
    // The last place of the biggest whole, full, still fits one envelope.
    let last = Part {
        whole,
        index: MAX_PARTS as u16 - 1,
        count: MAX_PARTS as u16,
        bytes: vec![1; MAX_PART_BYTES],
    }
    .sign(DOMAIN, &author, NOW)
    .unwrap();
    assert!(last.as_wire().len() <= MAX_DOCUMENT_BYTES);
    verify_part(last.as_wire(), DOMAIN, NOW).unwrap();
}
