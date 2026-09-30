//! Rotating mailbox addresses known only to the conversation.

use crate::digest::digest;

/// Length of one mailbox period.
pub const PERIOD_SECONDS: u64 = 86_400;

/// The period containing wall time `at` (seconds).
pub fn period(at: u64) -> u64 {
    at / PERIOD_SECONDS
}

/// Periods a mailbox is kept after its own.
pub const RETENTION_PERIODS: u64 = 30;

/// When holders drop a mailbox of `period`.
pub fn expires_at(period: u64) -> u64 {
    period
        .saturating_add(1 + RETENTION_PERIODS)
        .saturating_mul(PERIOD_SECONDS)
}

/// Whether a mailbox of `period` is still kept at `at`.
pub fn live(period: u64, at: u64) -> bool {
    at < expires_at(period)
}

/// Whether a fresh entry of `period` is taken at `at`: from one period
/// before (clock skew) to one after (a sender keeps a pinned period while
/// its recipient still reads it).
pub fn writable(period: u64, at: u64) -> bool {
    let now = self::period(at);
    period.saturating_add(1) >= now && period <= now.saturating_add(1)
}

/// `H(domain, MLS exporter secret for one direction, period)`.
pub fn mailbox_id(domain: &[u8; 32], exporter: &[u8; 32], period: u64) -> [u8; 32] {
    digest("AIN_MAILBOX_V1", &[domain, exporter, &period.to_be_bytes()])
}

/// `H(domain, network id digest, period)`: the mailbox where anyone who knows
/// a network id asks it for a conversation (spec/contact-by-id-v1.md).
pub fn intro_mailbox_id(domain: &[u8; 32], identity: &[u8; 32], period: u64) -> [u8; 32] {
    digest("AIN_INTRO_V1", &[domain, identity, &period.to_be_bytes()])
}

/// `H(domain, G, period)`: where an open group's posts and rosters lie in
/// the clear, for anyone who knows its reference `G`
/// (Docs/V1_DISCOVERY_2026_09_27.md, part 2).
pub fn public_group_mailbox_id(domain: &[u8; 32], group: &[u8; 32], period: u64) -> [u8; 32] {
    digest(
        "AIN_PUBLIC_GROUP_V1",
        &[domain, group, &period.to_be_bytes()],
    )
}

/// `H(domain, G, period)`: where a group's door card lies and applications
/// to join it are left (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 5).
pub fn door_mailbox_id(domain: &[u8; 32], group: &[u8; 32], period: u64) -> [u8; 32] {
    digest("AIN_GROUP_DOOR_V1", &[domain, group, &period.to_be_bytes()])
}
