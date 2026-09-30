//! A channel's archive (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts
//! 9, 10 and 10b): its team packs its posts into parts of up to 64 KB and
//! lays them in the public mailbox of the day, then again every 25 days,
//! for as long as the channel keeps its history. A part counts as laid
//! that day once the node stored it at a quorum, or read a copy of it back
//! from the mailbox, the team's own or another's.
use super::*;
use agentic_mailbox_swarm::address::{period, public_group_mailbox_id};
use agentic_protocol::group::{Access, ArchivePart, Retention};
use std::collections::BTreeMap;

/// An entry of a public mailbox: an archive part.
pub const PUBLIC_ARCHIVE: u8 = 3;

const PACKS: &str = "channel/pack/";
const PARTS: &str = "channel/part/";
const DAY: u64 = 24 * 60 * 60;
/// A mailbox keeps a post, or a copy of a part, this long.
const LIVE: u64 = 30 * DAY;
/// A part closes once its oldest post is this old.
const CLOSE_AFTER: u64 = 25 * DAY;
/// Days after its latest copy a part is laid again.
const LAY_EVERY: u64 = 25;
/// What a part's entry takes besides its posts, at most.
const PART_OVERHEAD: usize = 64;
/// What each post adds to a part besides its bytes, at most.
const POST_OVERHEAD: usize = 5;

/// What keeping a channel's history costs its team.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStorage {
    /// Days, `None` for ever.
    pub retention: Option<u32>,
    /// Parts the team keeps laying down: what readers find besides the
    /// posts of the last 30 days.
    pub parts: u64,
    pub bytes: u64,
    /// Stamps a month to keep them: each part is laid every 25 days.
    pub stamps_per_month: u64,
    /// Parts closed within the last 30 days: how a channel kept for ever
    /// grows.
    pub added_last_month: u64,
}

/// What a member of a channel's team keeps to pack and lay its archive.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    /// Posts in no part yet.
    open: Vec<OpenPost>,
    /// Posts in parts, with when they were stored: forgotten once no
    /// mailbox holds them.
    archived: BTreeMap<String, u64>,
    /// Parts kept, by hash; their entries apart (`PARTS`).
    parts: BTreeMap<String, PartRecord>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenPost {
    id: String,
    wire: String,
    stored_at: u64,
    issued_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PartRecord {
    /// The day of the latest copy read back from the mailbox; `None` until
    /// one is.
    seen: Option<u64>,
    /// When its newest post was written.
    newest: u64,
    bytes: u64,
    closed_at: u64,
}

impl PartRecord {
    /// Whether the team keeps it under `retention` at `now`: while its
    /// newest post is younger than the retention; with no archive, while
    /// its posts are still alive, should the retention grow.
    fn kept(&self, retention: Retention, now: u64) -> bool {
        if !archived(retention) {
            return self.newest + LIVE > now;
        }
        retention
            .days()
            .is_none_or(|days| self.newest + u64::from(days) * DAY > now)
    }
}

/// A channel keeps an archive when it keeps posts longer than a mailbox.
fn archived(retention: Retention) -> bool {
    retention
        .days()
        .is_none_or(|days| u64::from(days) * DAY > LIVE)
}

impl AppCore {
    fn pack(&self, id: &str) -> Result<(Pack, u64), CoreError> {
        match self.store.state(&format!("{PACKS}{id}"))? {
            Some(state) => Ok((
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                state.revision,
            )),
            None => Ok((Pack::default(), 0)),
        }
    }

    fn pack_change(id: &str, pack: &Pack, revision: u64) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{PACKS}{id}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(pack).map_err(invalid)?,
        })
    }

    /// A post of channel `id` its team member wrote or took from the
    /// mailbox, to pack: the change to commit with it, if any.
    pub(super) fn channel_post_change(
        &self,
        id: &str,
        wire: &[u8],
        stored_at: u64,
        issued_at: u64,
    ) -> Result<Option<StateChange>, CoreError> {
        let (mut pack, revision) = self.pack(id)?;
        let post = hex::encode(Sha256::digest(wire));
        if pack.archived.contains_key(&post) || pack.open.iter().any(|open| open.id == post) {
            return Ok(None);
        }
        pack.open.push(OpenPost {
            id: post,
            wire: hex::encode(wire),
            stored_at,
            issued_at,
        });
        Ok(Some(Self::pack_change(id, &pack, revision)?))
    }

    /// Note a copy of a part this team member read in the mailbox, stored
    /// at `stored_at`: laid that day. Whether the part was known.
    pub(super) fn channel_part_known(
        &mut self,
        id: &str,
        hash: &str,
        stored_at: u64,
    ) -> Result<bool, CoreError> {
        let (mut pack, revision) = self.pack(id)?;
        let Some(record) = pack.parts.get_mut(hash) else {
            return Ok(false);
        };
        let day = period(stored_at);
        if record.seen.is_none_or(|seen| seen < day) {
            record.seen = Some(day);
            self.store
                .commit_states(vec![Self::pack_change(id, &pack, revision)?])?;
        }
        Ok(true)
    }

    /// A part of channel `id` (its entry's hash in hex) this node stored
    /// at a quorum in the mailbox of day `day`: laid that day.
    pub fn channel_part_stored(&mut self, id: &str, hash: &str, day: u64) -> Result<(), CoreError> {
        let (mut pack, revision) = self.pack(id)?;
        if let Some(record) = pack.parts.get_mut(hash)
            && record.seen.is_none_or(|seen| seen < day)
        {
            record.seen = Some(day);
            self.store
                .commit_states(vec![Self::pack_change(id, &pack, revision)?])?;
        }
        Ok(())
    }

    /// A part new to this team member, read in the mailbox with `posts`
    /// (ids and when written) that hold: kept and laid that day, its posts
    /// no longer to pack. The changes to commit.
    pub(super) fn channel_part_changes(
        &self,
        id: &str,
        entry: &[u8],
        posts: &[(String, u64)],
        stored_at: u64,
    ) -> Result<Vec<StateChange>, CoreError> {
        let (mut pack, revision) = self.pack(id)?;
        let hash = hex::encode(Sha256::digest(entry));
        for (post, _) in posts {
            pack.archived.insert(post.clone(), stored_at);
        }
        pack.open
            .retain(|open| !posts.iter().any(|(post, _)| *post == open.id));
        pack.parts.insert(
            hash.clone(),
            PartRecord {
                seen: Some(period(stored_at)),
                newest: posts.iter().map(|(_, at)| *at).max().unwrap_or(stored_at),
                bytes: entry.len() as u64,
                closed_at: stored_at,
            },
        );
        Ok(vec![
            Self::pack_change(id, &pack, revision)?,
            self.part_row(id, &hash, entry)?,
        ])
    }

    fn part_row(&self, id: &str, hash: &str, entry: &[u8]) -> Result<StateChange, CoreError> {
        let namespace = format!("{PARTS}{id}/{hash}");
        let expected_revision = self
            .store
            .state(&namespace)?
            .map_or(0, |state| state.revision);
        Ok(StateChange {
            namespace,
            expected_revision,
            bytes: entry.to_vec(),
        })
    }

    /// The parts of channel `id` to lay in the public mailbox now: closed
    /// ones not yet read back, and those whose latest copy is 25 days old,
    /// while the channel keeps them. The same part under the same stamp
    /// until a copy of it is read back. None for a plain reader, or a
    /// channel kept no longer than a mailbox keeps posts.
    pub fn channel_archive(&mut self, id: &str, now: u64) -> Result<Vec<SwarmDelivery>, CoreError> {
        let view = self.public_view(id)?;
        // A closed channel keeps no history, whatever its data says: its
        // posts never go into the clear.
        if !view.channel || !view.team || view.roster.access != Access::Public {
            return Ok(vec![]);
        }
        let (mut pack, revision) = self.pack(id)?;
        pack.archived.retain(|_, stored_at| *stored_at + LIVE > now);
        // Close the parts that are full, and the last once its oldest post
        // is 25 days old.
        let mut open = std::mem::take(&mut pack.open);
        // Oldest first; as they came when stored at once.
        open.sort_by_key(|post| post.stored_at);
        let mut rows = vec![];
        let mut current: Vec<OpenPost> = vec![];
        let mut size = PART_OVERHEAD;
        for post in open {
            let bytes = post.wire.len() / 2 + POST_OVERHEAD;
            if !current.is_empty() && size + bytes > agentic_protocol::MAX_DOCUMENT_BYTES {
                rows.push(self.close_part(id, view.group, &mut pack, &current, now)?);
                current.clear();
                size = PART_OVERHEAD;
            }
            size += bytes;
            current.push(post);
        }
        if current
            .first()
            .is_some_and(|oldest| oldest.stored_at + CLOSE_AFTER <= now)
        {
            rows.push(self.close_part(id, view.group, &mut pack, &current, now)?);
        } else {
            pack.open = current;
        }
        // Let go what is no longer kept; lay what is due.
        let mut deleted = vec![];
        let mut due = vec![];
        let day = period(now);
        for (hash, record) in pack.parts.clone() {
            let namespace = format!("{PARTS}{id}/{hash}");
            if !record.kept(view.retention, now) {
                pack.parts.remove(&hash);
                if let Some(row) = self.store.state(&namespace)? {
                    deleted.push((namespace, row.revision));
                }
                continue;
            }
            if archived(view.retention) && record.seen.is_none_or(|seen| day >= seen + LAY_EVERY) {
                due.push(namespace);
            }
        }
        let mut changes = vec![Self::pack_change(id, &pack, revision)?];
        changes.extend(rows);
        let entries: BTreeMap<String, Vec<u8>> = changes
            .iter()
            .filter(|change| change.namespace.starts_with(PARTS))
            .map(|change| (change.namespace.clone(), change.bytes.clone()))
            .collect();
        self.store.commit_state_maintenance(changes, deleted)?;
        let mailbox = public_group_mailbox_id(&self.domain, &view.group, day);
        let mut deliveries = vec![];
        for namespace in due {
            let entry = match entries.get(&namespace) {
                Some(entry) => entry.clone(),
                None => match self.store.state(&namespace)? {
                    Some(row) => row.bytes.clone(),
                    None => continue,
                },
            };
            let (stamp, changes) = self.stamp_changes(&mailbox, day, &entry, now)?;
            if !changes.is_empty() {
                self.store.commit_states(changes)?;
            }
            deliveries.push(SwarmDelivery {
                message_id: String::new(),
                conversation_id: id.into(),
                period: day,
                mailbox,
                envelope: entry,
                stamp,
            });
        }
        Ok(deliveries)
    }

    /// Close a part of `posts`: kept, its posts no longer to pack. The row
    /// of its entry.
    fn close_part(
        &self,
        id: &str,
        group: [u8; 32],
        pack: &mut Pack,
        posts: &[OpenPost],
        now: u64,
    ) -> Result<StateChange, CoreError> {
        let part = ArchivePart {
            group,
            posts: posts
                .iter()
                .map(|post| hex::decode(&post.wire).map_err(|_| CoreError::InvalidState))
                .collect::<Result<_, _>>()?,
        };
        let entry = [&[PUBLIC_ARCHIVE][..], &part.encode()].concat();
        let hash = hex::encode(Sha256::digest(&entry));
        for post in posts {
            pack.archived.insert(post.id.clone(), post.stored_at);
        }
        pack.parts.insert(
            hash.clone(),
            PartRecord {
                seen: None,
                newest: posts.iter().map(|post| post.issued_at).max().unwrap_or(now),
                bytes: entry.len() as u64,
                closed_at: now,
            },
        );
        self.part_row(id, &hash, &entry)
    }

    /// What keeping channel `id`'s history costs now.
    pub fn channel_storage(&self, id: &str, now: u64) -> Result<ChannelStorage, CoreError> {
        let view = self.public_view(id)?;
        if !view.channel {
            return Err(CoreError::InvalidInput);
        }
        let (pack, _) = self.pack(id)?;
        let kept: Vec<&PartRecord> = if archived(view.retention) {
            pack.parts
                .values()
                .filter(|record| record.kept(view.retention, now))
                .collect()
        } else {
            vec![]
        };
        let parts = kept.len() as u64;
        Ok(ChannelStorage {
            retention: view.retention.days(),
            parts,
            bytes: kept.iter().map(|record| record.bytes).sum(),
            stamps_per_month: (parts * 30).div_ceil(LAY_EVERY),
            added_last_month: kept
                .iter()
                .filter(|record| record.closed_at + LIVE > now)
                .count() as u64,
        })
    }
}
