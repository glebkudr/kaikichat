//! The directory (phase 2 of Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md): the
//! registry's active units, read from the chain every ten minutes, and each
//! unit's record signed by its transport key, pulled from peers. Membership
//! is the active units with a verified record; a failed registry read keeps
//! the last membership and is tried again a minute later.
use super::mailbox_client::{Holder, Purpose};
use super::mailbox_holder::{Request, Response, UnitRecordWire};
use super::*;
use agentic_mailbox_swarm::directory::{UnitRecord, unit_commitment};
use std::collections::{BTreeMap, BTreeSet};

/// How often the registry is read...
const UNITS_EVERY: Duration = Duration::from_secs(600);
/// ...and how soon again after a failed read.
const UNITS_RETRY: Duration = Duration::from_secs(60);
/// How often a random member is asked for its directory...
const PULL_EVERY: Duration = Duration::from_secs(600);
/// ...and every connected peer, while active units lack a record.
const PULL_MISSING_EVERY: Duration = Duration::from_secs(30);
/// Peers asked at once while units are missing.
const PULL_FAN_OUT: usize = 16;
/// Records one answer carries.
const MAX_RECORDS: usize = 1024;
/// How far ahead of this node's clock a record may be dated.
const AHEAD: u64 = 600;
/// An own record is issued again this often, even with the same addresses.
const REISSUE_AFTER: u64 = 6 * 3_600;

#[derive(Default)]
pub(super) struct Directory {
    /// Active units at the last successful registry read; `None` before one.
    units: Option<BTreeSet<[u8; 32]>>,
    reading: bool,
    units_due: Option<Instant>,
    /// The newest verified record of each unit.
    records: BTreeMap<[u8; 32], UnitRecord>,
    pulls: usize,
    pull_due: Option<Instant>,
    pulled: u64,
}

impl Directory {
    pub(super) fn info(&self) -> Value {
        json!({
            "units": self.units.as_ref().map(BTreeSet::len),
            "records": self.records.len(),
            "pullsInFlight": self.pulls,
            "pulled": self.pulled,
        })
    }

    pub(super) fn next_due(&self) -> Option<Instant> {
        let units = self.units_due.filter(|_| !self.reading);
        let pull = self.pull_due.filter(|_| self.pulls == 0);
        units.into_iter().chain(pull).min()
    }

    /// Active units without a record.
    fn missing(&self) -> bool {
        self.units
            .as_ref()
            .is_some_and(|units| units.iter().any(|u| !self.records.contains_key(u)))
    }
}

/// The transport's peer id of an Ed25519 public key.
pub(super) fn peer_of(transport_key: &[u8; 32]) -> Option<PeerId> {
    let key = identity::ed25519::PublicKey::try_from_bytes(transport_key).ok()?;
    Some(identity::PublicKey::from(key).to_peer_id())
}

impl Runtime {
    pub(super) fn own_transport_key(&self) -> [u8; 32] {
        self.transport_key
            .public()
            .try_into_ed25519()
            .map(|key| key.to_bytes())
            .unwrap_or_default()
    }

    /// What this node bonds as a unit: its transport key and receipt account.
    pub(super) fn own_commitment(&self) -> [u8; 32] {
        unit_commitment(
            &NETWORK_DOMAIN,
            &self.own_transport_key(),
            &self.mailbox_holder.account(),
        )
    }

    /// This node's record, signed by its transport key.
    pub(super) fn unit_record(&self, issued_at: u64) -> UnitRecord {
        let key = self.transport_key.clone();
        UnitRecord::sign(
            &NETWORK_DOMAIN,
            self.own_transport_key(),
            self.mailbox_holder.account(),
            self.advertised(),
            issued_at,
            |digest| {
                key.sign(digest)
                    .ok()
                    .and_then(|signature| signature.try_into().ok())
                    .unwrap_or([0; 64])
            },
        )
    }

    /// Whether the registry lists `unit` as active.
    pub(super) fn directory_lists(&self, unit: &[u8; 32]) -> bool {
        self.directory
            .units
            .as_ref()
            .is_some_and(|units| units.contains(unit))
    }

    /// This node's own record while the registry lists it.
    pub(super) fn own_unit_record(&self) -> Option<UnitRecord> {
        let own = self.own_commitment();
        if !self.directory_lists(&own) {
            return None;
        }
        self.directory.records.get(&own).cloned()
    }

    /// Directory diagnostics, with the commitment an operator bonds for
    /// this node.
    pub(super) fn directory_info(&self) -> Value {
        let mut info = self.directory.info();
        info["ownCommitment"] = json!(hex::encode(self.own_commitment()));
        info
    }

    /// Active units read from the registry.
    #[cfg_attr(not(test), allow(dead_code, reason = "diagnostics"))]
    pub(super) fn directory_units(&self) -> usize {
        self.directory.units.as_ref().map_or(0, BTreeSet::len)
    }

    /// The verified records of active units, this node's own included.
    pub(super) fn directory_records(&self) -> Vec<UnitRecordWire> {
        let Some(units) = &self.directory.units else {
            return Vec::new();
        };
        self.directory
            .records
            .iter()
            .filter(|(unit, _)| units.contains(*unit))
            .take(MAX_RECORDS)
            .map(|(_, record)| UnitRecordWire::from(record))
            .collect()
    }

    /// A registry read finished: `None` if it failed.
    pub(super) fn units_read(&mut self, units: Option<Vec<[u8; 32]>>, instant: Instant) {
        self.directory.reading = false;
        let Some(units) = units else {
            self.directory.units_due = Some(instant + UNITS_RETRY);
            return;
        };
        self.directory.units_due = Some(instant + UNITS_EVERY);
        let units: BTreeSet<[u8; 32]> = units.into_iter().collect();
        let own = self.own_commitment();
        if units.contains(&own) {
            self.mailbox_holder.set_unit(own);
        } else if self.mailbox_holder.unit() == Some(own) {
            self.mailbox_holder.clear_unit();
        }
        self.directory
            .records
            .retain(|unit, _| units.contains(unit));
        self.directory.units = Some(units);
        self.issue_own_record();
        self.list_members();
        if self.directory.missing() {
            self.directory.pull_due = Some(instant);
        }
    }

    /// Records another node sent: the newest verified one of each active
    /// unit is kept.
    pub(super) fn directory_page(&mut self, records: Vec<UnitRecordWire>) {
        let Some(units) = &self.directory.units else {
            return;
        };
        let Ok(now) = now() else { return };
        let mut changed = false;
        for wire in records.iter().take(MAX_RECORDS) {
            let Ok(record) = UnitRecord::try_from(wire) else {
                continue;
            };
            let Ok(unit) = record.verify(&NETWORK_DOMAIN) else {
                continue;
            };
            if !units.contains(&unit) || record.issued_at > now + AHEAD {
                continue;
            }
            if self
                .directory
                .records
                .get(&unit)
                .is_some_and(|kept| kept.issued_at >= record.issued_at)
            {
                continue;
            }
            self.directory.records.insert(unit, record);
            changed = true;
        }
        if changed {
            self.list_members();
        }
    }

    /// Sign this node's record again when it is a unit and its addresses
    /// changed or the record grew old.
    fn issue_own_record(&mut self) {
        let own = self.own_commitment();
        if !self
            .directory
            .units
            .as_ref()
            .is_some_and(|units| units.contains(&own))
        {
            return;
        }
        let Ok(now) = now() else { return };
        let fresh = self.directory.records.get(&own).is_some_and(|record| {
            record.addresses == self.advertised()
                && now.saturating_sub(record.issued_at) < REISSUE_AFTER
        });
        if !fresh {
            let record = self.unit_record(now);
            self.directory.records.insert(own, record);
            self.list_members();
        }
    }

    /// The client's directory: active units with a verified record.
    fn list_members(&mut self) {
        let Some(units) = &self.directory.units else {
            return;
        };
        let members: BTreeMap<[u8; 32], Holder> = self
            .directory
            .records
            .iter()
            .filter(|(unit, _)| units.contains(*unit))
            .filter_map(|(unit, record)| {
                Some((
                    *unit,
                    Holder {
                        peer: peer_of(&record.transport_key)?,
                        addresses: record
                            .addresses
                            .iter()
                            .filter_map(|a| a.parse().ok())
                            .collect(),
                        account: record.receipt,
                    },
                ))
            })
            .collect();
        self.mailbox_client.set_directory(members);
    }

    /// Read the registry and pull records when due.
    pub(super) fn maintain_directory(&mut self) {
        let instant = clock::instant();
        let directory = &mut self.directory;
        if !directory.reading && directory.units_due.is_none_or(|due| due <= instant) {
            directory.units_due = None;
            if self.read_units() {
                self.directory.reading = true;
            } else {
                // No chain: the directory stays as it was set.
                return;
            }
        }
        if self.directory.units.is_none() {
            return;
        }
        self.issue_own_record();
        if self.directory.pulls > 0 || self.directory.pull_due.is_some_and(|due| due > instant) {
            return;
        }
        let peers: Vec<(PeerId, Vec<Multiaddr>)> = if self.directory.missing() {
            self.swarm
                .connected_peers()
                .take(PULL_FAN_OUT)
                .map(|peer| (*peer, Vec::new()))
                .collect()
        } else {
            let own = self.mailbox_holder.unit();
            let members: Vec<&Holder> = self
                .mailbox_client
                .directory
                .iter()
                .filter(|(unit, _)| Some(**unit) != own)
                .map(|(_, holder)| holder)
                .collect();
            let mut pick = [0; 8];
            if members.is_empty() || random::fill(&mut pick).is_err() {
                Vec::new()
            } else {
                let holder = members[(u64::from_be_bytes(pick) % members.len() as u64) as usize];
                vec![(holder.peer, holder.addresses.clone())]
            }
        };
        self.directory.pull_due = Some(
            instant
                + if self.directory.missing() {
                    PULL_MISSING_EVERY
                } else {
                    PULL_EVERY
                },
        );
        for (peer, addresses) in peers {
            self.directory.pulls += 1;
            self.send_mailbox(
                Holder {
                    peer,
                    addresses,
                    account: [0; 20],
                },
                Request::Directory,
                Purpose::Directory { peer },
            );
        }
    }

    /// A peer answered a directory pull.
    pub(super) fn directory_answer(
        &mut self,
        _peer: PeerId,
        outcome: std::result::Result<Response, String>,
    ) {
        self.directory.pulls = self.directory.pulls.saturating_sub(1);
        if let Ok(Response::Directory { records }) = outcome {
            self.directory.pulled += 1;
            self.directory_page(records);
        }
    }
}
