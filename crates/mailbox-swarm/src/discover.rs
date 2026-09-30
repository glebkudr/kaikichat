//! What the discovery service is paid for (spec/discovery-v1.md): the
//! operations its stamps sign, keccak256 digests like every stamp's.

use crate::digest::digest;

/// The one byte a handle kind is signed as.
fn kind_code(kind: &str) -> u8 {
    match kind {
        "google" => 1,
        "github" => 2,
        _ => 0,
    }
}

/// A lookup of one handle on UTC `day` (unix time / 86400): its kind and
/// the SHA-256 of its normal form. The day bounds how long one stamp is
/// answered for.
pub fn lookup_operation(domain: &[u8; 32], kind: &str, handle: &[u8; 32], day: u64) -> [u8; 32] {
    digest(
        "AIN_DISCOVER_LOOKUP_V1",
        &[domain, &[kind_code(kind)], handle, &day.to_be_bytes()],
    )
}

/// Stamp `index` (0…9) of the ten a card is published with.
pub fn card_operation(domain: &[u8; 32], card: &[u8; 32], index: u32) -> [u8; 32] {
    digest(
        "AIN_DISCOVER_CARD_V1",
        &[domain, card, &index.to_be_bytes()],
    )
}
