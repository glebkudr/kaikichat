//! The operator pool's draw (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md): a
//! paid stamp's slot wins when its score under the seed of its book's
//! purchase day is below the pool's threshold. `OperatorPool.score`
//! computes the same.

use crate::digest::digest;

/// A slot's draw under a day's seed, a big-endian number.
pub fn win_score(seed: &[u8; 32], ticket: &[u8; 32]) -> [u8; 32] {
    digest("AIN_WIN_V1", &[seed, ticket])
}

/// Whether the slot wins: strictly below the threshold.
pub fn wins(seed: &[u8; 32], ticket: &[u8; 32], threshold: &[u8; 32]) -> bool {
    win_score(seed, ticket) < *threshold
}
