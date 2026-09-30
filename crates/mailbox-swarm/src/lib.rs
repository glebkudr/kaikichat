//! Pure rules of the recipient-mailbox swarm (Docs/V1_STORAGE_REDESIGN_2026_09_24.md):
//! who holds a mailbox, how it is addressed, how a stamp pays for one message,
//! what a holder signs, and the self-contained proofs that justify blocking a
//! sender or a holder. No I/O, clocks or storage live here; callers pass time.
//!
//! Signed digests are keccak256 over fixed-width big-endian fields behind a
//! domain tag, so an EVM contract can recompute them and `ecrecover` the
//! secp256k1 signer to act on a proof.

pub mod access;
pub mod address;
mod digest;
pub mod directory;
pub mod discover;
pub mod payout;
pub mod proof;
pub mod receipt;
pub mod select;
pub mod stamp;

/// An EVM-style account: the last 20 bytes of keccak256 of the public key.
pub type Account = [u8; 20];

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
