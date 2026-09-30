//! Open-read groups (Docs/V1_DISCOVERY_2026_09_27.md, part 2;
//! Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 10a): a public group's
//! posts go in the clear, signed by their authors, into its public mailbox,
//! each with its author's certificate of membership; its owner and admins
//! publish there the roster in force and who may no longer write. A
//! profile follows groups it only reads, from the day before.
use super::*;
use agentic_mailbox_swarm::address::{period, public_group_mailbox_id};
use agentic_protocol::group::{
    Access, ArchivePart, BANNED, GroupKind, MAX_PUBLIC_REMOVED, Retention, Roster, VerifiedPost,
    verify_public_post, verify_public_roster,
};
use std::collections::BTreeMap;

/// An entry of a public mailbox: a post, or a roster.
pub const PUBLIC_POST: u8 = 1;
pub const PUBLIC_ROSTER: u8 = 2;

const FOLLOWS: &str = "public/follow/";
const READERS: &str = "public/reader/";
/// Posts of members a member does not know yet, kept per conversation.
const MAX_WAITING: usize = 64;
/// A follower and a newcomer read this far back: the group's last day.
const HISTORY: u64 = 24 * 60 * 60;
/// A channel's follower and newcomer read what the mailboxes keep.
const CHANNEL_HISTORY: u64 = 30 * HISTORY;
/// Archive parts a follower remembers having read, the latest.
const MAX_KNOWN_PARTS: usize = 512;
/// Removals a follower keeps at most, the latest.
const MAX_KEPT_REMOVED: usize = 2 * MAX_PUBLIC_REMOVED;
/// Groups a profile follows at most.
const MAX_FOLLOWS: usize = 256;

/// A group this profile reads without being a member.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowInfo {
    /// `G` in hex: the conversation its posts are kept in.
    pub id: String,
    pub name: String,
    /// The owner's network id.
    pub owner: String,
    /// Posts stored before this moment are not taken.
    pub since: u64,
    /// The group was closed: nothing more is read.
    pub closed: bool,
    /// `group`, or `channel`, as its roster says once read.
    pub kind: String,
    /// Days its history is kept for readers, `None` for ever.
    pub retention: Option<u32>,
    /// A closed channel read by its keys, from when they were given: it
    /// has no history to read back.
    pub sealed: bool,
}

/// What an entry of a public mailbox did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicEntry {
    /// A post was taken.
    Post,
    /// A roster was taken.
    Roster,
    /// An archive part of a channel not read before was taken.
    Archive,
    /// A closed channel's new keys were taken: its key moved on.
    Rekeyed,
    /// A closed channel's subscriber's own key was taken by its owner.
    SubscriberKey,
    /// The group was closed.
    Closed,
    /// Passed over: not the group's, not one of its writers', a duplicate,
    /// or waiting for its roster.
    Ignored,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredFollow {
    version: u8,
    name: String,
    owner: String,
    since: u64,
    closed: bool,
    /// Its roster made it a channel.
    #[serde(default)]
    channel: bool,
    /// Days its history is kept, 0 for ever.
    #[serde(default = "a_day")]
    retention: u32,
    /// A closed channel's keys: it is read sealed.
    #[serde(default)]
    keys: Option<super::closed_channels::FollowKeys>,
}

fn a_day() -> u32 {
    1
}

/// What a reader of a public mailbox keeps.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reader {
    /// Rosters by epoch, as older readers kept them.
    #[serde(default, skip_serializing, rename = "rosters")]
    _rosters: Option<serde::de::IgnoredAny>,
    /// The newest public roster's epoch taken.
    #[serde(default)]
    epoch: Option<u64>,
    /// The newest owner-signed roster known: its version and its admins'
    /// root keys in hex.
    #[serde(default)]
    roster: Option<(u64, Vec<String>)>,
    /// Network id digests in hex that may no longer write under a
    /// certificate of that epoch or before (`BANNED`: under none).
    #[serde(default)]
    removed: BTreeMap<String, u64>,
    /// A member's posts by authors it does not know as members yet: the
    /// post and when it was stored.
    waiting: Vec<(String, u64)>,
    /// A follower's archive parts read, by hash.
    #[serde(default)]
    parts: Vec<String>,
}

/// Who a public mailbox is read for.
struct Context {
    group: [u8; 32],
    owner: String,
    /// Posts stored before this are not taken: the day before one followed
    /// or joined.
    since: u64,
    closed: bool,
    /// Only its owner and admins write in it.
    channel: bool,
    /// A closed channel's: how its entries are sealed.
    sealing: Option<super::closed_channels::Sealing>,
    /// A member's own view of the group: it checks posts against it, not
    /// against published rosters.
    member: Option<MemberView>,
}

/// What a post read in a public mailbox is to its reader.
enum Verdict {
    /// Its certificate holds, and it is new: its id.
    Take(VerifiedPost, String),
    /// Its certificate holds; it is taken already.
    Known(VerifiedPost, String),
    /// By someone a member does not know yet, under a later certificate.
    Wait,
    Reject,
}

struct MemberView {
    epoch: u64,
    members: Vec<String>,
    roster: Roster,
    removed: BTreeMap<String, u64>,
}

impl AppCore {
    fn follow_state(&self, id: &str) -> Result<Option<(StoredFollow, u64)>, CoreError> {
        self.store
            .state(&format!("{FOLLOWS}{id}"))?
            .map(|state| {
                Ok((
                    serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                    state.revision,
                ))
            })
            .transpose()
    }

    pub(super) fn is_follow(&self, id: &str) -> Result<bool, CoreError> {
        Ok(parse_id(id).is_ok() && self.follow_state(id)?.is_some())
    }

    /// Ids of the groups this profile follows.
    /// The name a follow is listed under.
    pub(super) fn follow_name(&self, id: &str) -> Result<Option<String>, CoreError> {
        Ok(self.follow_state(id)?.map(|(stored, _)| stored.name))
    }

    pub(super) fn follow_ids(&self) -> Result<Vec<String>, CoreError> {
        let mut ids = vec![];
        let mut after = FOLLOWS.to_owned();
        let through = format!("{FOLLOWS}~");
        loop {
            let page = self.store.state_namespaces_between(&after, &through, 64)?;
            let Some(last) = page.last().cloned() else {
                return Ok(ids);
            };
            ids.extend(
                page.iter()
                    .filter_map(|namespace| namespace.strip_prefix(FOLLOWS).map(str::to_owned)),
            );
            after = last;
        }
    }

    /// Follow the open group `group` (`G`) of the owner `owner`, to read its
    /// posts from now on.
    pub fn follow_group(
        &mut self,
        group: [u8; 32],
        owner: &str,
        name: &str,
        now: u64,
    ) -> Result<FollowInfo, CoreError> {
        valid_name(name)?;
        super::intro::identity_digest(owner)?;
        let id = hex::encode(group);
        if let Some((stored, _)) = self.follow_state(&id)? {
            return Ok(info_of(&id, &stored));
        }
        if self.follow_ids()?.len() >= MAX_FOLLOWS {
            return Err(CoreError::InvalidInput);
        }
        let stored = StoredFollow {
            version: 1,
            name: name.trim().into(),
            owner: owner.into(),
            since: now,
            closed: false,
            channel: false,
            retention: a_day(),
            keys: None,
        };
        self.store.commit_states(vec![StateChange {
            namespace: format!("{FOLLOWS}{id}"),
            expected_revision: 0,
            bytes: serde_json::to_vec(&stored).map_err(invalid)?,
        }])?;
        Ok(info_of(&id, &stored))
    }

    /// Stop following; the posts already taken stay.
    pub fn unfollow_group(&mut self, id: &str) -> Result<(), CoreError> {
        parse_id(id)?;
        let (_, revision) = self
            .follow_state(id)?
            .ok_or(CoreError::UnknownConversation)?;
        let mut deleted = vec![(format!("{FOLLOWS}{id}"), revision)];
        if let Some(state) = self.store.state(&format!("{READERS}{id}"))? {
            deleted.push((format!("{READERS}{id}"), state.revision));
        }
        self.store.commit_state_maintenance(vec![], deleted)?;
        Ok(())
    }

    pub fn follows(&self) -> Result<Vec<FollowInfo>, CoreError> {
        self.follow_ids()?
            .into_iter()
            .filter_map(|id| match self.follow_state(&id) {
                Ok(Some((stored, _))) => Some(Ok(info_of(&id, &stored))),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    /// The groups this profile follows, as read-only conversations.
    pub(super) fn follow_conversations(&self) -> Result<Vec<Conversation>, CoreError> {
        let mut conversations = vec![];
        for id in self.follow_ids()? {
            let Some((stored, _)) = self.follow_state(&id)? else {
                continue;
            };
            let mut messages = vec![];
            // History comes after what is new: shown as written.
            let mut kept = self.store.messages(&id, 0, 1000)?;
            kept.sort_by_key(|message| message.record.created_at);
            for message in kept {
                if let Some(message) = self.present_message(&message)? {
                    messages.push(message);
                }
            }
            conversations.push(Conversation {
                id,
                title: stored.name,
                unread: 0,
                messages,
            });
        }
        Ok(conversations)
    }

    fn public_context(&self, conversation: &str) -> Result<Context, CoreError> {
        if let Some((stored, _)) = self.follow_state(conversation)? {
            // A closed channel is read from the keys on: no history.
            let back = if stored.keys.is_some() {
                0
            } else if stored.channel {
                CHANNEL_HISTORY
            } else {
                HISTORY
            };
            return Ok(Context {
                group: parse_id(conversation)?,
                owner: stored.owner,
                since: stored.since.saturating_sub(back),
                closed: stored.closed,
                channel: stored.channel || stored.keys.is_some(),
                sealing: self.channel_sealing(conversation)?,
                member: None,
            });
        }
        let sealing = self.channel_sealing(conversation)?;
        let view = self.public_view(conversation)?;
        let back = if view.channel {
            CHANNEL_HISTORY
        } else {
            HISTORY
        };
        Ok(Context {
            group: view.group,
            owner: network_id(&view.owner),
            since: view.joined_at.saturating_sub(back),
            closed: !view.public,
            channel: view.channel,
            sealing,
            member: Some(MemberView {
                epoch: view.epoch,
                members: view.members,
                roster: view.roster,
                removed: view.removed,
            }),
        })
    }

    /// The public mailbox of an open group this profile is in, or follows,
    /// for the period of `at`; `None` while it is not read.
    pub fn public_mailbox(
        &self,
        conversation: &str,
        at: u64,
    ) -> Result<Option<[u8; 32]>, CoreError> {
        let context = self.public_context(conversation)?;
        if context.closed {
            return Ok(None);
        }
        Ok(Some(match &context.sealing {
            Some(sealing) => sealing
                .mailbox(&self.domain, at)
                .ok_or(CoreError::InvalidState)?,
            None => public_group_mailbox_id(&self.domain, &context.group, period(at)),
        }))
    }

    /// The mailboxes of `conversation` to read for `at`'s period: the
    /// public one; for a closed channel, its key's and, for key updates
    /// and late posts, the key's before.
    pub fn public_mailboxes(
        &self,
        conversation: &str,
        at: u64,
    ) -> Result<Vec<[u8; 32]>, CoreError> {
        let context = self.public_context(conversation)?;
        if context.closed {
            return Ok(vec![]);
        }
        Ok(match &context.sealing {
            Some(sealing) => sealing.mailboxes(&self.domain, at),
            None => vec![public_group_mailbox_id(
                &self.domain,
                &context.group,
                period(at),
            )],
        })
    }

    /// Whether a signer the caller found in `roster` signs for followed
    /// group `conversation`: `roster` is no older than the newest known. An
    /// admin removed or demoted — every such change is a new roster — signs
    /// for nobody.
    pub(super) fn signer_in_force(
        &self,
        conversation: &str,
        roster: &Roster,
        signer: &[u8; 32],
    ) -> Result<bool, CoreError> {
        let (reader, _) = self.reader(conversation)?;
        let _ = signer;
        Ok(reader
            .roster
            .as_ref()
            .is_none_or(|(version, _)| roster.version >= *version))
    }

    /// Know `roster` of followed group `conversation` as the newest, if it
    /// is newer than the one known.
    pub(super) fn note_roster(
        &mut self,
        conversation: &str,
        roster: &Roster,
    ) -> Result<(), CoreError> {
        let (mut reader, revision) = self.reader(conversation)?;
        if reader
            .roster
            .as_ref()
            .is_none_or(|(version, _)| roster.version > *version)
        {
            reader.roster = Some((
                roster.version,
                roster.admins.iter().map(hex::encode).collect(),
            ));
            self.store.commit_states(vec![Self::reader_change(
                conversation,
                &reader,
                revision,
            )?])?;
        }
        Ok(())
    }

    /// A closed channel's keys this profile follows it with.
    pub(super) fn follow_keys(
        &self,
        conversation: &str,
    ) -> Result<Option<super::closed_channels::FollowKeys>, CoreError> {
        Ok(self
            .follow_state(conversation)?
            .and_then(|(stored, _)| stored.keys))
    }

    pub(super) fn set_follow_keys(
        &mut self,
        conversation: &str,
        keys: &super::closed_channels::FollowKeys,
    ) -> Result<(), CoreError> {
        let (mut stored, revision) = self
            .follow_state(conversation)?
            .ok_or(CoreError::UnknownConversation)?;
        stored.keys = Some(keys.clone());
        self.store.commit_states(vec![StateChange {
            namespace: format!("{FOLLOWS}{conversation}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(&stored).map_err(invalid)?,
        }])?;
        Ok(())
    }

    /// Follow closed channel `group` of `owner` with `keys`, reading what
    /// was stored from `since` on; its owner's roster known.
    pub(super) fn follow_with_keys(
        &mut self,
        group: [u8; 32],
        name: &str,
        owner: &str,
        keys: super::closed_channels::FollowKeys,
        roster: &Roster,
        since: u64,
    ) -> Result<(), CoreError> {
        let id = hex::encode(group);
        let previous = self.follow_state(&id)?;
        if previous.is_none() && self.follow_ids()?.len() >= MAX_FOLLOWS {
            return Err(CoreError::InvalidInput);
        }
        let stored = StoredFollow {
            version: 1,
            name: name.trim().into(),
            owner: owner.into(),
            since,
            closed: false,
            channel: true,
            retention: Retention::DEFAULT.code(),
            keys: Some(keys),
        };
        // The roster known never goes back to an older one.
        let (mut reader, revision) = self.reader(&id)?;
        if reader
            .roster
            .as_ref()
            .is_none_or(|(version, _)| roster.version > *version)
        {
            reader.roster = Some((
                roster.version,
                roster.admins.iter().map(hex::encode).collect(),
            ));
        }
        self.store.commit_states(vec![
            StateChange {
                namespace: format!("{FOLLOWS}{id}"),
                expected_revision: previous.map_or(0, |(_, r)| r),
                bytes: serde_json::to_vec(&stored).map_err(invalid)?,
            },
            Self::reader_change(&id, &reader, revision)?,
        ])?;
        Ok(())
    }

    fn reader(&self, conversation: &str) -> Result<(Reader, u64), CoreError> {
        match self.store.state(&format!("{READERS}{conversation}"))? {
            Some(state) => Ok((
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                state.revision,
            )),
            None => Ok((Reader::default(), 0)),
        }
    }

    fn reader_change(
        conversation: &str,
        reader: &Reader,
        revision: u64,
    ) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace: format!("{READERS}{conversation}"),
            expected_revision: revision,
            bytes: serde_json::to_vec(reader).map_err(invalid)?,
        })
    }

    /// Take an entry read from `conversation`'s public mailbox, stored there
    /// at `stored_at`.
    pub fn receive_public_entry(
        &mut self,
        conversation: &str,
        entry: &[u8],
        stored_at: u64,
        now: u64,
    ) -> Result<PublicEntry, CoreError> {
        let context = self.public_context(conversation)?;
        if context.closed {
            return Ok(PublicEntry::Ignored);
        }
        let Some((tag, wire)) = entry.split_first() else {
            return Ok(PublicEntry::Ignored);
        };
        // A closed channel's entries come sealed: its posts, its roster,
        // its new keys.
        if let Some(sealing) = &context.sealing {
            if *tag != super::key_tree::SEALED_ENTRY {
                return Ok(PublicEntry::Ignored);
            }
            return match sealing.open(wire) {
                Some(inner) if matches!(inner.first(), Some(&PUBLIC_POST | &PUBLIC_ROSTER)) => {
                    self.take_entry(conversation, &context, &inner, stored_at, now)
                }
                Some(inner) if inner.first() == Some(&super::key_tree::KEY_UPDATE) => {
                    self.take_key_update(conversation, &context.owner, entry, &inner[1..], now)
                }
                Some(inner) if inner.first() == Some(&super::key_tree::SUBSCRIBER_KEY) => {
                    self.take_subscriber_key(conversation, context.member.is_some(), &inner, now)
                }
                // The team's own update under a generation it has left.
                None if context.member.is_some() => {
                    self.channel_key_doc_stored(conversation, &hex::encode(Sha256::digest(entry)))?;
                    Ok(PublicEntry::Ignored)
                }
                _ => Ok(PublicEntry::Ignored),
            };
        }
        self.take_entry(conversation, &context, entry, stored_at, now)
    }

    fn take_entry(
        &mut self,
        conversation: &str,
        context: &Context,
        entry: &[u8],
        stored_at: u64,
        now: u64,
    ) -> Result<PublicEntry, CoreError> {
        let Some((tag, wire)) = entry.split_first() else {
            return Ok(PublicEntry::Ignored);
        };
        match *tag {
            // A member goes by its own view of the group.
            PUBLIC_ROSTER if context.member.is_none() => {
                self.take_roster(conversation, context, wire, now)
            }
            // A channel's archive parts, whatever their age: its team lays
            // them for as long as it keeps them.
            super::channels::PUBLIC_ARCHIVE if context.channel => {
                self.take_part(conversation, context, entry, wire, stored_at, now)
            }
            // The day is counted by when the holders stored a post, not by
            // the date its author put on it.
            PUBLIC_POST if stored_at >= context.since => {
                let (mut reader, revision) = self.reader(conversation)?;
                let taken =
                    self.take_post(conversation, context, &mut reader, wire, stored_at, now)?;
                self.store.commit_states(vec![Self::reader_change(
                    conversation,
                    &reader,
                    revision,
                )?])?;
                Ok(taken)
            }
            _ => Ok(PublicEntry::Ignored),
        }
    }

    /// Take the posts that waited for their authors' epoch: a member's
    /// after it moved on.
    pub(super) fn retry_public_posts(
        &mut self,
        conversation: &str,
        now: u64,
    ) -> Result<(), CoreError> {
        let (mut reader, revision) = self.reader(conversation)?;
        if reader.waiting.is_empty() {
            return Ok(());
        }
        let context = self.public_context(conversation)?;
        if context.closed {
            return Ok(());
        }
        let waiting = std::mem::take(&mut reader.waiting);
        for (post, stored_at) in waiting {
            let post = hex::decode(post).map_err(|_| CoreError::InvalidState)?;
            self.take_post(conversation, &context, &mut reader, &post, stored_at, now)?;
        }
        self.store
            .commit_states(vec![Self::reader_change(conversation, &reader, revision)?])?;
        Ok(())
    }

    fn take_roster(
        &mut self,
        conversation: &str,
        context: &Context,
        wire: &[u8],
        now: u64,
    ) -> Result<PublicEntry, CoreError> {
        let Ok(roster) = verify_public_roster(wire, self.domain, now, &context.group) else {
            return Ok(PublicEntry::Ignored);
        };
        if network_id(&roster.owner) != context.owner {
            return Ok(PublicEntry::Ignored);
        }
        let (mut reader, revision) = self.reader(conversation)?;
        if reader.epoch.is_some_and(|newest| roster.epoch < newest) {
            return Ok(PublicEntry::Ignored);
        }
        // Closed to reading, private or by request: the follow ends — but
        // a closed channel's subscriber reads it by its keys.
        if roster.access != Access::Public && context.sealing.is_none() {
            if let Some((mut stored, follow_revision)) = self.follow_state(conversation)? {
                stored.closed = true;
                let mut changes = vec![StateChange {
                    namespace: format!("{FOLLOWS}{conversation}"),
                    expected_revision: follow_revision,
                    bytes: serde_json::to_vec(&stored).map_err(invalid)?,
                }];
                if revision > 0 {
                    changes.push(Self::reader_change(
                        conversation,
                        &Reader::default(),
                        revision,
                    )?);
                }
                self.store.commit_states(changes)?;
            }
            return Ok(PublicEntry::Closed);
        }
        reader.epoch = Some(roster.epoch);
        let mut changes = vec![];
        if let Some((mut stored, follow_revision)) = self.follow_state(conversation)? {
            let channel = roster.roster.kind == GroupKind::Channel;
            let retention = roster.retention.code();
            if (stored.channel, stored.retention) != (channel, retention) {
                stored.channel = channel;
                stored.retention = retention;
                changes.push(StateChange {
                    namespace: format!("{FOLLOWS}{conversation}"),
                    expected_revision: follow_revision,
                    bytes: serde_json::to_vec(&stored).map_err(invalid)?,
                });
            }
        }
        if reader
            .roster
            .as_ref()
            .is_none_or(|(version, _)| roster.roster.version >= *version)
        {
            reader.roster = Some((
                roster.roster.version,
                roster.roster.admins.iter().map(hex::encode).collect(),
            ));
        }
        // Bans as the newest roster says; removals are facts, kept.
        reader.removed.retain(|_, last| *last != BANNED);
        for (member, last) in roster.removed {
            let kept = reader.removed.entry(hex::encode(member)).or_default();
            *kept = (*kept).max(last);
        }
        while reader.removed.len() > MAX_KEPT_REMOVED {
            let Some(oldest) = reader
                .removed
                .iter()
                .min_by_key(|(_, last)| **last)
                .map(|(member, _)| member.clone())
            else {
                break;
            };
            reader.removed.remove(&oldest);
        }
        changes.push(Self::reader_change(conversation, &reader, revision)?);
        self.store.commit_states(changes)?;
        Ok(PublicEntry::Roster)
    }

    /// Take a post whose certificate holds; hold one by someone a member
    /// does not know yet.
    fn take_post(
        &mut self,
        conversation: &str,
        context: &Context,
        reader: &mut Reader,
        wire: &[u8],
        stored_at: u64,
        now: u64,
    ) -> Result<PublicEntry, CoreError> {
        match self.judge_post(context, reader, wire, now)? {
            Verdict::Take(post, id) => {
                // An open channel's team packs what it takes.
                let pack =
                    if context.channel && context.member.is_some() && context.sealing.is_none() {
                        self.channel_post_change(conversation, wire, stored_at, post.issued_at)?
                    } else {
                        None
                    };
                self.store_post(conversation, id, post, pack.into_iter().collect())?;
                Ok(PublicEntry::Post)
            }
            Verdict::Wait => {
                if reader.waiting.len() < MAX_WAITING {
                    reader.waiting.push((hex::encode(wire), stored_at));
                }
                Ok(PublicEntry::Ignored)
            }
            Verdict::Known(..) | Verdict::Reject => Ok(PublicEntry::Ignored),
        }
    }

    /// Whether a post's certificate holds: it stands on the owner this
    /// profile knows, its issuer is the owner or an admin of the newest
    /// roster known, its author was not removed since nor banned — and,
    /// in a channel, is the owner or an admin. A member also knows the
    /// author as a member, else waits for a post of an epoch it has not
    /// reached.
    fn judge_post(
        &self,
        context: &Context,
        reader: &Reader,
        wire: &[u8],
        now: u64,
    ) -> Result<Verdict, CoreError> {
        let Ok(post) = verify_public_post(wire, self.domain, now, &context.group) else {
            return Ok(Verdict::Reject);
        };
        if network_id(&post.owner) != context.owner {
            return Ok(Verdict::Reject);
        }
        let (known, removed) = match &context.member {
            Some(member) => (
                (member.roster.version, member.roster.admins.clone()),
                &member.removed,
            ),
            None => (
                match &reader.roster {
                    Some((version, admins)) => (
                        *version,
                        admins
                            .iter()
                            .filter_map(|admin| parse_id(admin).ok())
                            .collect(),
                    ),
                    None => (0, vec![]),
                },
                &reader.removed,
            ),
        };
        let admins = match &post.roster {
            Some(inside) if inside.version > known.0 => &inside.admins,
            _ => &known.1,
        };
        if post.issuer != post.owner && !admins.contains(&post.issuer) {
            return Ok(Verdict::Reject);
        }
        if context.channel && post.author != post.owner && !admins.contains(&post.author) {
            return Ok(Verdict::Reject);
        }
        let author = network_id(&post.author);
        let digest = author.strip_prefix("ain1").unwrap_or_default();
        if removed.get(digest).is_some_and(|last| *last >= post.since) {
            return Ok(Verdict::Reject);
        }
        if let Some(member) = &context.member
            && !member.members.contains(&author)
        {
            return Ok(if post.since > member.epoch {
                Verdict::Wait
            } else {
                Verdict::Reject
            });
        }
        let id = hex::encode(Sha256::digest(wire));
        Ok(if self.store.message(&id)?.is_some() {
            Verdict::Known(post, id)
        } else {
            Verdict::Take(post, id)
        })
    }

    fn store_post(
        &mut self,
        conversation: &str,
        id: String,
        post: VerifiedPost,
        states: Vec<StateChange>,
    ) -> Result<(), CoreError> {
        let message = record(
            id,
            conversation,
            &network_id(&post.author),
            post.issued_at,
            false,
            Event::Text { text: post.text },
        )?;
        self.store
            .commit_incoming(agentic_store::IncomingCommit { message, states })?;
        Ok(())
    }

    /// Take an archive part of a channel, stored at `stored_at`: its posts
    /// whose certificates hold. A part read before is passed over; for the
    /// channel's team, a copy of it read is its laying that day.
    fn take_part(
        &mut self,
        conversation: &str,
        context: &Context,
        entry: &[u8],
        wire: &[u8],
        stored_at: u64,
        now: u64,
    ) -> Result<PublicEntry, CoreError> {
        let Ok(part) = ArchivePart::decode(wire) else {
            return Ok(PublicEntry::Ignored);
        };
        if part.group != context.group {
            return Ok(PublicEntry::Ignored);
        }
        let hash = hex::encode(Sha256::digest(entry));
        let (mut reader, revision) = self.reader(conversation)?;
        let known = match context.member {
            Some(_) => self.channel_part_known(conversation, &hash, stored_at)?,
            None => reader.parts.contains(&hash),
        };
        if known {
            return Ok(PublicEntry::Ignored);
        }
        let mut held = vec![];
        for post in &part.posts {
            match self.judge_post(context, &reader, post, now)? {
                Verdict::Take(post, id) => {
                    held.push((id.clone(), post.issued_at));
                    self.store_post(conversation, id, post, vec![])?;
                }
                Verdict::Known(post, id) => held.push((id, post.issued_at)),
                Verdict::Wait | Verdict::Reject => {}
            }
        }
        if held.is_empty() {
            return Ok(PublicEntry::Ignored);
        }
        let changes = match context.member {
            Some(_) => self.channel_part_changes(conversation, entry, &held, stored_at)?,
            None => {
                reader.parts.push(hash);
                if reader.parts.len() > MAX_KNOWN_PARTS {
                    reader.parts.remove(0);
                }
                vec![Self::reader_change(conversation, &reader, revision)?]
            }
        };
        self.store.commit_states(changes)?;
        Ok(PublicEntry::Archive)
    }

    /// The roster an owner or admin of an open group publishes now, stamped
    /// for the group's public mailbox; after the group closed, the closing
    /// roster for that day and the next. `None` for a plain member or a
    /// private group.
    pub fn public_roster(
        &mut self,
        id: &str,
        now: u64,
    ) -> Result<Option<SwarmDelivery>, CoreError> {
        let Some((group, wire)) = self.public_roster_document(id, now)? else {
            return Ok(None);
        };
        let _ = group;
        let pinned = period(now);
        let (mailbox, envelope) =
            self.public_entry_target(id, [&[PUBLIC_ROSTER][..], &wire].concat(), pinned)?;
        let (stamp, changes) = self.stamp_changes(&mailbox, pinned, &envelope, now)?;
        if !changes.is_empty() {
            self.store.commit_states(changes)?;
        }
        Ok(Some(SwarmDelivery {
            message_id: String::new(),
            conversation_id: id.into(),
            period: pinned,
            mailbox,
            envelope,
            stamp,
        }))
    }
}

fn info_of(id: &str, stored: &StoredFollow) -> FollowInfo {
    FollowInfo {
        id: id.into(),
        name: stored.name.clone(),
        owner: stored.owner.clone(),
        since: stored.since,
        closed: stored.closed,
        kind: if stored.channel { "channel" } else { "group" }.into(),
        retention: Retention::from_code(stored.retention).days(),
        sealed: stored.keys.is_some(),
    }
}
