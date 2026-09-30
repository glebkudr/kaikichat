//! Big groups (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts 1–3): a
//! group's ratchet tree goes apart from its Welcome — inline in a small
//! group's invitation, else in parts into a mailbox of its own that only
//! newcomers read — and group documents too big for one envelope are put
//! together from their parts.
use super::groups::GroupInvite;
use super::*;
use agentic_crypto::mailbox::MailboxSecret;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, RETENTION_PERIODS, period};
use agentic_protocol::parts::{Assembly, MAX_PARTS, Part, VerifiedPart, verify_part};

/// Where a newcomer finds the tree of the epoch it joins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum TreeRef {
    /// Small enough to come with the invitation.
    Inline(Vec<u8>),
    /// A `GroupTree` document in parts, named by its whole.
    Parts { whole: [u8; 32], count: u16 },
}

/// Invitations waiting for their tree, by group id.
const AWAITING: &str = "groups/await/";
/// Parts of awaited trees: `groups/tree-part/{group}/{whole}/{index}`.
const TREE_PARTS: &str = "groups/tree-part/";
/// Parts read from group mailboxes:
/// `groups/part/{group}/{author}{whole}/{index}`.
const PARTS: &str = "groups/part/";
/// The documents being put together in a group, oldest first.
const ASSEMBLIES: &str = "groups/assembly/";
/// Documents being put together per group at most; the oldest gives way.
const MAX_ASSEMBLIES: usize = 8;
/// A newcomer waits for its tree as long as the tree's mailbox is kept.
const AWAIT_FOR: u64 = (RETENTION_PERIODS + 1) * PERIOD_SECONDS;

/// The mailbox secret of the tree of `epoch`: derived from the group
/// mailbox's, so only those who know that find it.
pub(super) fn tree_secret(mailbox: &[u8; 32], epoch: u64) -> MailboxSecret {
    let mut hash = Sha256::new();
    hash.update(b"AIN_GROUP_TREE_V1");
    hash.update(mailbox);
    hash.update(epoch.to_be_bytes());
    MailboxSecret::from_bytes(hash.finalize().into())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Awaiting {
    /// The inviter's root key: the tree's parts are its.
    inviter: String,
    message_id: String,
    issued_at: u64,
    name: String,
    owner: String,
    roster: String,
    welcome: String,
    mailbox: String,
    epoch: u64,
    whole: String,
    count: u16,
    since: u64,
    /// The certificate of membership in hex.
    #[serde(default)]
    membership: String,
}

/// A group's documents being put together: `{author}{whole}`, the number
/// of parts, when the first came.
type Assemblies = Vec<(String, u16, u64)>;

fn unhex32(text: &str) -> Result<[u8; 32], CoreError> {
    hex::decode(text)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(CoreError::InvalidState)
}

fn part_name(prefix: &str, index: u16) -> String {
    format!("{prefix}{index:02}")
}

impl AppCore {
    /// Join a group of an invitation the recipient's policy let in: at once
    /// when its tree came along, else once the tree is whole.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn admit_group(
        &mut self,
        inviter_root: [u8; 32],
        message_id: String,
        issued_at: u64,
        group: [u8; 32],
        name: String,
        invite: &GroupInvite,
        welcome: &[u8],
        now: u64,
        states: Vec<StateChange>,
    ) -> Result<IntroOutcome, CoreError> {
        let (whole, count) = match &invite.tree {
            TreeRef::Inline(tree) => {
                return Ok(IntroOutcome::Joined(self.join_group(
                    inviter_root,
                    message_id,
                    issued_at,
                    group,
                    name,
                    invite,
                    welcome,
                    tree,
                    now,
                    states,
                )?));
            }
            TreeRef::Parts { whole, count } => (*whole, *count),
        };
        if count == 0 || usize::from(count) > MAX_PARTS {
            return Err(CoreError::InvalidInput);
        }
        let id = hex::encode(group);
        let namespace = format!("{AWAITING}{id}");
        // A newer invitation to the same group replaces the one waiting.
        self.drop_tree_parts(&id)?;
        let previous = self.store.state(&namespace)?;
        let mut all = states;
        all.push(StateChange {
            namespace,
            expected_revision: previous.map_or(0, |state| state.revision),
            bytes: serde_json::to_vec(&Awaiting {
                inviter: hex::encode(inviter_root),
                message_id,
                issued_at,
                name,
                owner: hex::encode(invite.owner),
                roster: hex::encode(&invite.roster),
                welcome: hex::encode(welcome),
                mailbox: hex::encode(invite.mailbox),
                epoch: invite.epoch,
                whole: hex::encode(whole),
                count,
                since: now,
                membership: hex::encode(&invite.membership),
            })
            .map_err(invalid)?,
        });
        self.store.commit_states(all)?;
        Ok(IntroOutcome::AwaitingTree(id))
    }

    fn awaiting(&self, id: &str) -> Result<Option<(Awaiting, u64)>, CoreError> {
        self.store
            .state(&format!("{AWAITING}{id}"))?
            .map(|state| {
                serde_json::from_slice(&state.bytes)
                    .map(|awaiting| (awaiting, state.revision))
                    .map_err(|_| CoreError::InvalidState)
            })
            .transpose()
    }

    fn names_under(&self, prefix: &str) -> Result<Vec<String>, CoreError> {
        let mut names = vec![];
        let mut after = prefix.to_owned();
        let through = format!("{prefix}~");
        loop {
            let page = self.store.state_namespaces_between(&after, &through, 64)?;
            let Some(last) = page.last().cloned() else {
                return Ok(names);
            };
            names.extend(page);
            after = last;
        }
    }

    /// Drop the given states, whatever their revision now.
    fn drop_states(&mut self, names: &[String]) -> Result<(), CoreError> {
        let mut removals = vec![];
        for name in names {
            if let Some(state) = self.store.state(name)? {
                removals.push((name.clone(), state.revision));
            }
        }
        for batch in removals.chunks(64) {
            self.store
                .commit_state_maintenance(vec![], batch.to_vec())?;
        }
        Ok(())
    }

    fn drop_tree_parts(&mut self, id: &str) -> Result<(), CoreError> {
        let names = self.names_under(&format!("{TREE_PARTS}{id}/"))?;
        self.drop_states(&names)
    }

    /// An awaited tree that can no longer come, or whose group was joined
    /// meanwhile, is let go.
    fn settle_awaiting(&mut self, id: &str, now: u64) -> Result<Option<Awaiting>, CoreError> {
        let Some((awaiting, _)) = self.awaiting(id)? else {
            return Ok(None);
        };
        if now >= awaiting.since.saturating_add(AWAIT_FOR) || self.member_of(id)? {
            self.drop_tree_parts(id)?;
            self.drop_states(&[format!("{AWAITING}{id}")])?;
            return Ok(None);
        }
        Ok(Some(awaiting))
    }

    /// The tree mailboxes newcomers read for the period containing `at`:
    /// one per group whose invitation waits for its tree.
    pub fn group_tree_mailboxes(&self, at: u64) -> Result<Vec<(String, [u8; 32])>, CoreError> {
        let mut out = vec![];
        for name in self.names_under(AWAITING)? {
            let Some(id) = name.strip_prefix(AWAITING) else {
                continue;
            };
            let Some((awaiting, _)) = self.awaiting(id)? else {
                continue;
            };
            if at >= awaiting.since.saturating_add(AWAIT_FOR) || self.member_of(id)? {
                continue;
            }
            let secret = tree_secret(&unhex32(&awaiting.mailbox)?, awaiting.epoch);
            out.push((
                id.to_owned(),
                secret.swarm_mailbox(&self.domain, period(at)),
            ));
        }
        Ok(out)
    }

    /// An envelope read from the tree mailbox of a group this profile was
    /// invited to: the group once its tree is whole and joined.
    pub fn receive_group_tree_envelope(
        &mut self,
        id: &str,
        period: u64,
        envelope: &[u8],
        now: u64,
    ) -> Result<Option<Conversation>, CoreError> {
        let Some(awaiting) = self.settle_awaiting(id, now)? else {
            return Err(CoreError::UnknownConversation);
        };
        let mailbox = unhex32(&awaiting.mailbox)?;
        let wire = tree_secret(&mailbox, awaiting.epoch)
            .open_swarm_envelope(&self.domain, period, envelope)
            .map_err(|_| CoreError::Unauthorized)?;
        let part = verify_part(&wire, self.domain, now)?;
        let whole = unhex32(&awaiting.whole)?;
        let inviter = unhex32(&awaiting.inviter)?;
        // Not a part of the awaited tree: passed over.
        if part.author != inviter || part.part.whole != whole || part.part.count != awaiting.count {
            return Ok(None);
        }
        let prefix = format!("{TREE_PARTS}{id}/{}/", awaiting.whole);
        let Some(wire) = self.keep_part(&prefix, &part)? else {
            return Ok(None);
        };
        let verified = VerifiedDocument::decode_large(&wire, self.domain, now)?;
        let group = parse_id(id)?;
        let tree = match Packet::decode(verified.body(), verified.kind())? {
            Packet::GroupTree {
                group: g,
                epoch,
                tree,
            } if g == group && epoch == awaiting.epoch && *verified.author() == inviter => tree,
            _ => {
                self.drop_tree_parts(id)?;
                return Err(CoreError::InvalidInput);
            }
        };
        let conversation = self.join_group(
            inviter,
            awaiting.message_id.clone(),
            awaiting.issued_at,
            group,
            awaiting.name.clone(),
            &GroupInvite {
                owner: unhex32(&awaiting.owner)?,
                roster: hex::decode(&awaiting.roster).map_err(|_| CoreError::InvalidState)?,
                mailbox,
                epoch: awaiting.epoch,
                tree: TreeRef::Parts {
                    whole,
                    count: awaiting.count,
                },
                membership: hex::decode(&awaiting.membership)
                    .map_err(|_| CoreError::InvalidState)?,
            },
            &hex::decode(&awaiting.welcome).map_err(|_| CoreError::InvalidState)?,
            &tree,
            now,
            vec![],
        )?;
        self.drop_tree_parts(id)?;
        self.drop_states(&[format!("{AWAITING}{id}")])?;
        Ok(Some(conversation))
    }

    /// Keep a part under `prefix`: the whole wire once every part is there
    /// and they add up to it, the kept parts then dropped.
    fn keep_part(
        &mut self,
        prefix: &str,
        part: &VerifiedPart,
    ) -> Result<Option<Vec<u8>>, CoreError> {
        let name = part_name(prefix, part.part.index);
        if self.store.state(&name)?.is_none() {
            self.store.commit_states(vec![StateChange {
                namespace: name,
                expected_revision: 0,
                bytes: part.part.bytes.clone(),
            }])?;
        }
        let mut assembly = Assembly::new(part);
        let mut names = vec![];
        let mut whole = None;
        for index in 0..part.part.count {
            let name = part_name(prefix, index);
            let Some(state) = self.store.state(&name)? else {
                return Ok(None);
            };
            names.push(name);
            let kept = VerifiedPart {
                author: part.author,
                part: Part {
                    whole: part.part.whole,
                    index,
                    count: part.part.count,
                    bytes: state.bytes.clone(),
                },
            };
            match assembly.add(&kept) {
                Ok(done) => whole = whole.or(done),
                Err(_) => {
                    // Parts of one author that do not add up: none is kept.
                    self.drop_states(&names)?;
                    return Ok(None);
                }
            }
        }
        self.drop_states(&names)?;
        Ok(whole)
    }

    /// A part read from a group mailbox: the whole document once complete.
    /// A group puts together a few documents at a time; the oldest gives way.
    pub(super) fn group_part(
        &mut self,
        id: &str,
        wire: &[u8],
        now: u64,
    ) -> Result<Option<VerifiedDocument>, CoreError> {
        let part = verify_part(wire, self.domain, now)?;
        let key = format!(
            "{}{}",
            hex::encode(part.author),
            hex::encode(part.part.whole)
        );
        let list_name = format!("{ASSEMBLIES}{id}");
        let stored = self.store.state(&list_name)?;
        let revision = stored.as_ref().map_or(0, |state| state.revision);
        let mut list: Assemblies = stored
            .map(|state| serde_json::from_slice(&state.bytes))
            .transpose()
            .map_err(|_| CoreError::InvalidState)?
            .unwrap_or_default();
        if !list.iter().any(|(k, _, _)| *k == key) {
            while list.len() >= MAX_ASSEMBLIES {
                let (oldest, _, _) = list.remove(0);
                let names = self.names_under(&format!("{PARTS}{id}/{oldest}/"))?;
                self.drop_states(&names)?;
            }
            list.push((key.clone(), part.part.count, now));
            self.store.commit_states(vec![StateChange {
                namespace: list_name.clone(),
                expected_revision: revision,
                bytes: serde_json::to_vec(&list).map_err(invalid)?,
            }])?;
        }
        let Some(whole) = self.keep_part(&format!("{PARTS}{id}/{key}/"), &part)? else {
            return Ok(None);
        };
        if let Some(state) = self.store.state(&list_name)? {
            list.retain(|(k, _, _)| *k != key);
            self.store.commit_states(vec![StateChange {
                namespace: list_name,
                expected_revision: state.revision,
                bytes: serde_json::to_vec(&list).map_err(invalid)?,
            }])?;
        }
        Ok(Some(VerifiedDocument::decode_large(
            &whole,
            self.domain,
            now,
        )?))
    }
}
