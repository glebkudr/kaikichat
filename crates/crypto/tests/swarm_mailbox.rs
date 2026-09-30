#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The swarm mailbox of a direction is the redesign's mailbox_id over the
//! exported MLS secret, not the pointer-record address of the same secret.
use agentic_crypto::mailbox::MailboxSecret;
use agentic_mailbox_swarm::address::mailbox_id;

#[test]
fn a_swarm_mailbox_is_the_mailbox_id_of_the_exported_secret() {
    let secret = [0x5c; 32];
    let domain = [0x11; 32];
    let swarm = MailboxSecret::from_bytes(secret).swarm_mailbox(&domain, 20_000);
    assert_eq!(swarm, mailbox_id(&domain, &secret, 20_000));
    assert_ne!(
        swarm,
        MailboxSecret::from_bytes(secret).swarm_mailbox(&domain, 20_001)
    );
}

// Holders see only the sealed envelope: never the signed wire (author key,
// kind, MLS frame) nor its exact length.
use agentic_crypto::mailbox::{MAX_SWARM_ENVELOPE, MailboxError, SWARM_ENVELOPE_BUCKET};

const DOMAIN: [u8; 32] = [0x11; 32];
const SECRET: [u8; 32] = [0x5c; 32];
const PERIOD: u64 = 20_000;

fn sealed_len(wire: usize) -> usize {
    12 + (4 + wire).div_ceil(SWARM_ENVELOPE_BUCKET) * SWARM_ENVELOPE_BUCKET + 16
}

fn seal(secret: [u8; 32], period: u64, wire: &[u8]) -> Vec<u8> {
    MailboxSecret::from_bytes(secret)
        .seal_swarm_envelope(&DOMAIN, period, wire)
        .unwrap()
}

fn open(secret: [u8; 32], period: u64, envelope: &[u8]) -> Result<Vec<u8>, MailboxError> {
    MailboxSecret::from_bytes(secret).open_swarm_envelope(&DOMAIN, period, envelope)
}

#[test]
fn a_sealed_envelope_opens_only_with_its_secret_and_period() {
    let wire = b"signed application document".to_vec();
    let envelope = seal(SECRET, PERIOD, &wire);
    assert_eq!(open(SECRET, PERIOD, &envelope).unwrap(), wire);
    // Deterministic, so a retried send pays for the same operation again.
    assert_eq!(seal(SECRET, PERIOD, &wire), envelope);
    // Yet never one nonce for two plaintexts under one key.
    let (a, b) = (
        seal(SECRET, PERIOD, &[1; 100]),
        seal(SECRET, PERIOD, &[2; 100]),
    );
    assert_eq!(a.len(), b.len());
    assert_ne!(a[..12], b[..12]);
    let (c, d) = (
        seal(SECRET, PERIOD, &[1; 100]),
        seal(SECRET, PERIOD, &[1; 101]),
    );
    assert_ne!(c[..12], d[..12]);
    let next = seal(SECRET, PERIOD + 1, &wire);
    assert_ne!(next, envelope);
    assert_eq!(open(SECRET, PERIOD + 1, &next).unwrap(), wire);
    assert!(matches!(
        open(SECRET, PERIOD + 1, &envelope),
        Err(MailboxError::Authentication)
    ));
    assert!(matches!(
        open([0x5d; 32], PERIOD, &envelope),
        Err(MailboxError::Authentication)
    ));
    assert!(matches!(
        MailboxSecret::from_bytes(SECRET).open_swarm_envelope(&[0x12; 32], PERIOD, &envelope),
        Err(MailboxError::Authentication)
    ));
    assert_ne!(seal([0x5d; 32], PERIOD, &wire), envelope);
    for at in [0, 11, 12, envelope.len() / 2, envelope.len() - 1] {
        let mut tampered = envelope.clone();
        tampered[at] ^= 1;
        assert!(
            matches!(
                open(SECRET, PERIOD, &tampered),
                Err(MailboxError::Authentication)
            ),
            "byte {at}"
        );
    }
    for cut in [0, 12, 27, 28, envelope.len() - 1] {
        assert!(open(SECRET, PERIOD, &envelope[..cut]).is_err(), "cut {cut}");
    }
    let mut longer = envelope.clone();
    longer.push(0);
    assert!(open(SECRET, PERIOD, &longer).is_err());
}

#[test]
fn a_sealed_envelope_hides_the_wire_and_its_exact_length() {
    let marker = [0xa7u8; 32];
    let mut wire = b"author:".to_vec();
    wire.extend_from_slice(&marker);
    let envelope = seal(SECRET, PERIOD, &wire);
    assert!(!envelope.windows(marker.len()).any(|w| w == marker));
    assert!(!envelope.windows(7).any(|w| w == b"author:"));
    assert_eq!(envelope.len(), sealed_len(wire.len()));
    // Lengths reveal only the bucket.
    let bucket = SWARM_ENVELOPE_BUCKET;
    for (a, b) in [(1, bucket - 4), (bucket - 3, 2 * bucket - 4)] {
        let (a, b) = (
            seal(SECRET, PERIOD, &vec![1; a]),
            seal(SECRET, PERIOD, &vec![2; b]),
        );
        assert_eq!(a.len(), b.len());
    }
    assert_eq!(seal(SECRET, PERIOD, &[1]).len(), 12 + bucket + 16);
    assert_eq!(
        seal(SECRET, PERIOD, &vec![1; bucket - 3]).len(),
        12 + 2 * bucket + 16
    );
    assert_eq!(
        open(SECRET, PERIOD, &seal(SECRET, PERIOD, &[])).unwrap(),
        b""
    );
}

#[test]
fn every_signed_document_fits_one_holder_envelope() {
    let largest = vec![0x42; agentic_protocol::MAX_DOCUMENT_BYTES];
    let envelope = seal(SECRET, PERIOD, &largest);
    assert_eq!(envelope.len(), MAX_SWARM_ENVELOPE);
    assert_eq!(
        MAX_SWARM_ENVELOPE,
        sealed_len(agentic_protocol::MAX_DOCUMENT_BYTES)
    );
    assert_eq!(open(SECRET, PERIOD, &envelope).unwrap(), largest);
    assert!(matches!(
        MailboxSecret::from_bytes(SECRET).seal_swarm_envelope(
            &DOMAIN,
            PERIOD,
            &vec![0x42; agentic_protocol::MAX_DOCUMENT_BYTES + 1]
        ),
        Err(MailboxError::Limit)
    ));
    let mut oversized = envelope;
    oversized.extend_from_slice(&[0; SWARM_ENVELOPE_BUCKET]);
    assert!(open(SECRET, PERIOD, &oversized).is_err());
}
