#![allow(clippy::unwrap_used, clippy::expect_used)]
//! A closed channel's key documents (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
//! part 10c): a subscriber's own key, signed by the subscriber, and a
//! reseed onto those keys, which only the owner signs.

use agentic_protocol::group::{
    Access, GroupKind, KeyUpdate, Roster, SubscriberKey, group_ref, verify_key_update,
    verify_subscriber_key,
};
use agentic_protocol::{DocumentDraft, DocumentKind, SignedDocument, WireError};
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;

const DOMAIN: [u8; 32] = [3; 32];
const NOW: u64 = 1_790_000_000;

fn signed(kind: DocumentKind, body: Vec<u8>, key: &SigningKey) -> Vec<u8> {
    SignedDocument::sign(
        DocumentDraft {
            domain: DOMAIN,
            kind,
            authority_epoch: 0,
            issued_at: NOW,
            expires_at: None,
            body,
            extensions: BTreeMap::new(),
        },
        key,
    )
    .unwrap()
    .to_wire()
}

struct Channel {
    owner: SigningKey,
    admin: SigningKey,
    reference: [u8; 32],
    roster: Vec<u8>,
}

fn channel() -> Channel {
    let owner = SigningKey::from_bytes(&[1; 32]);
    let admin = SigningKey::from_bytes(&[2; 32]);
    let reference = group_ref(&DOMAIN, &owner.verifying_key().to_bytes(), &[9; 32]);
    let roster = signed(
        DocumentKind::GroupRoster,
        Roster {
            group: reference,
            version: 3,
            admins: vec![admin.verifying_key().to_bytes()],
            access: Access::Private,
            kind: GroupKind::Channel,
        }
        .encode(),
        &owner,
    );
    Channel {
        owner,
        admin,
        reference,
        roster,
    }
}

fn update(c: &Channel, personal: Option<[u8; 32]>) -> KeyUpdate {
    KeyUpdate {
        group: c.reference,
        generation: 4,
        version: 2,
        removed: None,
        entries: vec![(1 << 32, vec![7; 48]), (0, vec![8; 48])],
        roster: c.roster.clone(),
        personal,
    }
}

#[test]
fn a_reseed_onto_subscribers_keys_names_its_key_and_only_the_owner_signs_it() {
    let c = channel();
    let hard = update(&c, Some([5; 32]));
    let decoded = KeyUpdate::decode(&hard.encode()).unwrap();
    assert_eq!(decoded, hard);
    // The earlier form is unchanged, and the two never read as each other.
    let soft = update(&c, None);
    assert_eq!(KeyUpdate::decode(&soft.encode()).unwrap(), soft);
    assert_ne!(soft.encode(), hard.encode());
    assert!(hard.encode().windows(7).any(|w| w == b"keys-v2"));
    assert!(soft.encode().windows(7).any(|w| w == b"keys-v1"));

    let by_owner = signed(DocumentKind::ChannelKeys, hard.encode(), &c.owner);
    let verified = verify_key_update(&by_owner, DOMAIN, NOW, &c.reference).unwrap();
    assert_eq!(verified.update.personal, Some([5; 32]));
    assert_eq!(verified.signer, c.owner.verifying_key().to_bytes());
    // An admin signs removals and reseeds under the old keys, never this.
    let by_admin = signed(DocumentKind::ChannelKeys, hard.encode(), &c.admin);
    assert!(matches!(
        verify_key_update(&by_admin, DOMAIN, NOW, &c.reference),
        Err(WireError::GroupRule)
    ));
    let soft_by_admin = signed(DocumentKind::ChannelKeys, soft.encode(), &c.admin);
    assert!(verify_key_update(&soft_by_admin, DOMAIN, NOW, &c.reference).is_ok());
    // A removal onto subscribers' keys is not a thing.
    let odd = KeyUpdate {
        removed: Some(3),
        ..hard.clone()
    };
    assert!(KeyUpdate::decode(&odd.encode()).is_err());
}

#[test]
fn a_subscribers_key_is_signed_by_the_subscriber_for_one_channel_and_leaf() {
    let c = channel();
    let subscriber = SigningKey::from_bytes(&[4; 32]);
    let key = SubscriberKey {
        group: c.reference,
        leaf: 0x0100_0007,
        key: [6; 32],
    };
    assert_eq!(SubscriberKey::decode(&key.encode()).unwrap(), key);
    let wire = signed(DocumentKind::ChannelSubscriber, key.encode(), &subscriber);
    let (verified, signer) = verify_subscriber_key(&wire, DOMAIN, NOW, &c.reference).unwrap();
    assert_eq!(verified, key);
    assert_eq!(signer, subscriber.verifying_key().to_bytes());
    // Another channel's, or another kind of document, is not taken.
    assert!(verify_subscriber_key(&wire, DOMAIN, NOW, &[0; 32]).is_err());
    let other = signed(DocumentKind::ChannelKeys, key.encode(), &subscriber);
    assert!(verify_subscriber_key(&other, DOMAIN, NOW, &c.reference).is_err());
    // Extra bytes are not the same document.
    let mut longer = key.encode();
    longer.push(0);
    assert!(SubscriberKey::decode(&longer).is_err());
}
