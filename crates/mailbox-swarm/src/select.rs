//! Rendezvous (highest random weight) selection of a key's holders.

use crate::digest::digest;
use std::collections::BTreeSet;

/// One registry unit, identified by its stable on-chain commitment. Every
/// unit carries the same bond, so every unit has weight one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Member {
    pub commitment: [u8; 32],
}

/// Holders of one mailbox.
pub const SWARM_SIZE: usize = 10;
/// Signed holder receipts after which a message counts as stored.
pub const QUORUM: usize = 7;

/// The `size` distinct members with the highest rendezvous score for `key`,
/// best first. The result depends only on the set of members, not on their
/// order or duplicates.
pub fn rendezvous(key: &[u8; 32], members: &[Member], size: usize) -> Vec<Member> {
    let mut ranked: Vec<_> = members
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|m| (digest("AIN_RENDEZVOUS_V1", &[key, &m.commitment]), m))
        .collect();
    ranked.sort_by(|a, b| b.cmp(a));
    ranked.into_iter().take(size).map(|(_, m)| m).collect()
}

/// The point a key's notaries are chosen around: apart from any mailbox.
pub fn notary_point(key: &[u8; 32]) -> [u8; 32] {
    digest("AIN_NOTARY_V1", &[key])
}

/// The units that notarize `key`: first writer wins there.
pub fn notaries(key: &[u8; 32], members: &[Member]) -> Vec<Member> {
    rendezvous(&notary_point(key), members, SWARM_SIZE)
}
