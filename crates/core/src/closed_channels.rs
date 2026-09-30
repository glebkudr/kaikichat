//! Closed channels (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts 8
//! and 10c): the team seals the channel's entries under the root key of a
//! key tree it derives from its seed; each subscriber gets its leaf's path
//! keys alone, like an invitation; a removal or a reseed is published by
//! the commit's author as signed key updates, sealed under the key before,
//! in that key's mailbox. Nothing of a closed channel ever goes out in the
//! clear: without its key tree it sends nothing.
use super::intro::identity_digest;
use super::key_tree::{self, PathKeys, ROOT};
use super::*;
use agentic_mailbox_swarm::address::period;
use agentic_protocol::group::{
    Access, BANNED, GroupKind, KeyTree, KeyUpdate, Roster, SubscriberKey, group_ref,
    verify_key_update, verify_roster, verify_subscriber_key,
};

/// The team's map of subscribers: network id → the leaves given.
const SUBSCRIBERS: &str = "channel/subscribers/";
/// Leaves of this profile's branch given so far: never given again.
const NEXT_LEAF: &str = "channel/next/";
/// Notes to the team of keys given, not sent yet.
const ANNOUNCE: &str = "channel/announce/";
/// What the commit's author still has to publish: the tree before and the
/// leaves removed, kept with the commit.
const KEY_PLAN: &str = "channel/keyplan/";
const KEY_DOCS: &str = "channel/keydocs/";
/// The owner's record of its subscribers' own keys: network id → leaf →
/// key, as each subscriber signed it.
const PERSONAL: &str = "channel/personal/";
/// Keys of their own for leaves the team's notes did not give their signer
/// yet: a few per leaf.
const PERSONAL_WAITING: &str = "channel/personal-waiting/";
/// Such keys kept per leaf.
const WAITING_KEYS: usize = 4;
/// Leaves past the highest a branch gave that such keys are kept for.
const EXPECTED_AHEAD: u32 = 1024;
/// New keys onto subscribers' own are due at the owner: a team member is
/// out.
const HARD_DUE: &str = "channel/hard/";
/// Subscribers whose keys the owner gives again: no key of their own was
/// known at the reseed onto subscribers' keys.
const REISSUE: &str = "channel/reissue/";
/// Channel keys before the current a subscriber keeps.
const KEPT_KEYS: usize = 16;
/// A key replaced is read in this long after, for late posts and updates.
const KEEP_READING: u64 = 2 * 24 * 60 * 60;
/// Key updates a subscriber holds until those before them come, and their
/// bytes at most.
const MAX_WAITING_UPDATES: usize = 64;
const MAX_WAITING_BYTES: usize = 1 << 20;
/// A reseed's sealed keys per update: one document each.
const RESEED_ENTRIES: usize = 700;
/// Leaves of a branch: the 24 low bits.
const BRANCH_SHIFT: u32 = 24;

/// A subscriber's own keys as the owner keeps them: leaf → key in hex and
/// when the subscriber signed it.
type PersonalKeys = BTreeMap<u32, (String, u64)>;

/// A channel key before the current: what it opens, until when it is read.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Before {
    generation: u32,
    version: u32,
    key: String,
    /// When it was replaced.
    replaced: u64,
}

/// A subscriber's keys of a closed channel, kept with its follow.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FollowKeys {
    pub generation: u32,
    pub leaf: u32,
    /// The root's version: the removals applied.
    pub version: u32,
    /// The leaf's path: node, its version and key in hex.
    pub path: BTreeMap<u64, (u32, String)>,
    #[serde(default)]
    pub before: Vec<Before>,
    /// A reseed's keys found so far.
    #[serde(default)]
    pub staged: BTreeMap<u64, (u32, String)>,
    /// Key updates, in hex, waiting for those before them.
    #[serde(default)]
    pub waiting: Vec<String>,
    /// This subscriber's own X25519 key for the channel, the secret in hex:
    /// the owner seals new keys to it when a team member leaves.
    #[serde(default)]
    pub personal: Option<String>,
    /// The generation its own key was last published under and read back
    /// or stored; none since keys were given.
    #[serde(default)]
    pub announced: Option<u32>,
    /// Its signed document of that key, and the generation it goes under.
    #[serde(default)]
    pub hello: Option<(u32, String)>,
    /// The roster's version the current generation was taken under.
    #[serde(default)]
    pub roster: u64,
    /// A reseed onto its own key coming together: the generation it starts
    /// from, the keys found, and its path's entries not opened yet.
    #[serde(default)]
    pub hard: Option<u32>,
    #[serde(default)]
    pub hard_staged: BTreeMap<u64, (u32, String)>,
    #[serde(default)]
    pub pending: BTreeMap<u64, Vec<String>>,
    /// The hash of the envelope its key document last went out in.
    #[serde(default)]
    pub hello_sealed: Option<String>,
}

fn from_hex(keys: &BTreeMap<u64, (u32, String)>) -> Option<PathKeys> {
    keys.iter()
        .map(|(of, (v, key))| Some((*of, (*v, key_of(key)?))))
        .collect()
}

fn to_hex(keys: &PathKeys) -> BTreeMap<u64, (u32, String)> {
    keys.iter()
        .map(|(of, (v, key))| (*of, (*v, hex::encode(key))))
        .collect()
}

fn unhex(text: &str) -> Result<Vec<u8>, CoreError> {
    hex::decode(text).map_err(|_| CoreError::InvalidState)
}

fn unhex32(text: &str) -> Result<[u8; 32], CoreError> {
    unhex(text)?.try_into().map_err(|_| CoreError::InvalidState)
}

fn key_of(text: &str) -> Option<[u8; 32]> {
    hex::decode(text).ok()?.try_into().ok()
}

impl FollowKeys {
    fn root(&self) -> Option<[u8; 32]> {
        key_of(&self.path.get(&ROOT)?.1)
    }

    /// Its own key's public half.
    pub(super) fn personal_public(&self) -> Option<[u8; 32]> {
        let secret = key_of(self.personal.as_deref()?)?;
        Some(x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(secret)).to_bytes())
    }

    /// A new generation: a reseed under the keys before that was coming
    /// together goes. One onto its own key stays: it counts from any
    /// generation, and what of it was read is not read again.
    fn clear_staging(&mut self) {
        self.staged.clear();
    }

    /// The key now becomes one before, replaced at `now`.
    fn retire(&mut self, now: u64) -> Option<()> {
        let key = hex::encode(self.root()?);
        self.before.push(Before {
            generation: self.generation,
            version: self.version,
            key,
            replaced: now,
        });
        while self.before.len() > KEPT_KEYS {
            self.before.remove(0);
        }
        Some(())
    }
}

/// How a closed channel's entries are sealed and opened.
pub(super) enum Sealing {
    /// Its team's, from the seed.
    Team { group: [u8; 32], tree: KeyTree },
    /// A subscriber's, from its keys.
    Subscriber { group: [u8; 32], keys: FollowKeys },
}

impl Sealing {
    /// The channel key now: its generation, version and key.
    pub(super) fn current(&self) -> Option<(u32, u32, [u8; 32])> {
        match self {
            Self::Team { group, tree } => {
                let version = u32::try_from(tree.removed.len()).ok()?;
                Some((
                    tree.generation,
                    version,
                    key_tree::node_key(&tree.seed, group, tree.generation, ROOT, version),
                ))
            }
            Self::Subscriber { keys, .. } => Some((keys.generation, keys.version, keys.root()?)),
        }
    }

    /// The channel key of `generation` at `version`, if known.
    pub(super) fn root_for(&self, generation: u32, version: u32) -> Option<[u8; 32]> {
        match self {
            Self::Team { group, tree } => (generation == tree.generation
                && version as usize <= tree.removed.len())
            .then(|| key_tree::node_key(&tree.seed, group, generation, ROOT, version)),
            Self::Subscriber { keys, .. } => {
                if (generation, version) == (keys.generation, keys.version) {
                    return keys.root();
                }
                keys.before
                    .iter()
                    .find(|b| (b.generation, b.version) == (generation, version))
                    .and_then(|b| key_of(&b.key))
            }
        }
    }

    fn group(&self) -> [u8; 32] {
        match self {
            Self::Team { group, .. } | Self::Subscriber { group, .. } => *group,
        }
    }

    /// Where the channel's entries under its key now go for `at`'s period.
    pub(super) fn mailbox(&self, domain: &[u8; 32], at: u64) -> Option<[u8; 32]> {
        let (_, _, root) = self.current()?;
        Some(channel_mailbox(domain, &self.group(), &root, period(at)))
    }

    /// The mailboxes to read for `at`'s period: the key's now, and of the
    /// keys replaced within two days, where the updates and late posts of
    /// those keys lie.
    pub(super) fn mailboxes(&self, domain: &[u8; 32], at: u64) -> Vec<[u8; 32]> {
        let group = self.group();
        let mut roots = vec![];
        if let Some((generation, version, root)) = self.current() {
            roots.push(root);
            match self {
                Self::Team { .. } => {
                    roots.extend(
                        version
                            .checked_sub(1)
                            .and_then(|v| self.root_for(generation, v)),
                    );
                }
                Self::Subscriber { keys, .. } => roots.extend(
                    keys.before
                        .iter()
                        .filter(|b| b.replaced.saturating_add(KEEP_READING) > at)
                        .filter_map(|b| key_of(&b.key)),
                ),
            }
        }
        roots
            .iter()
            .map(|root| channel_mailbox(domain, &group, root, period(at)))
            .collect()
    }

    /// Open a sealed entry: what is inside.
    pub(super) fn open(&self, wire: &[u8]) -> Option<Vec<u8>> {
        let (group, generation, version, nonce, sealed) = key_tree::sealed_under(wire)?;
        if group != self.group() {
            return None;
        }
        let root = self.root_for(generation, version)?;
        key_tree::open_entry(&root, &group, generation, version, &nonce, &sealed)
    }
}

/// The mailbox of the channel key `root` for `period`.
fn channel_mailbox(domain: &[u8; 32], group: &[u8; 32], root: &[u8; 32], period: u64) -> [u8; 32] {
    agentic_mailbox_swarm::address::mailbox_id(
        domain,
        &key_tree::derived(root, group, b"mailbox"),
        period,
    )
}

/// A key update to publish, in the mailbox of the key it starts from.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyDoc {
    hash: String,
    envelope: String,
    /// The mailbox secret of the key before.
    secret: String,
}

/// What the commit's author still has to publish.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KeyPlan {
    before: super::groups::StoredTree,
    leaves: Vec<u32>,
    /// A reseed onto subscribers' own keys.
    #[serde(default)]
    hard: bool,
    /// Team members out, by network id: what they held as subscribers is
    /// not given on.
    #[serde(default)]
    out: Vec<String>,
}

/// What an update did to a subscriber's keys; what it took came under a
/// roster to know from now on.
enum Update {
    Applied(Roster),
    /// A reseed's keys found, not all yet.
    Staged(Roster),
    Wait,
    No,
}

impl AppCore {
    /// How group or follow `conversation`'s entries are sealed, when it is
    /// a closed channel.
    pub(super) fn channel_sealing(&self, conversation: &str) -> Result<Option<Sealing>, CoreError> {
        if let Some(keys) = self.follow_keys(conversation)? {
            return Ok(Some(Sealing::Subscriber {
                group: parse_id(conversation)?,
                keys,
            }));
        }
        if self.is_follow(conversation)? {
            return Ok(None);
        }
        let view = self.public_view(conversation)?;
        Ok(view
            .tree
            .filter(|_| view.team || view.public)
            .map(|tree| Sealing::Team {
                group: view.group,
                tree,
            }))
    }

    /// Where the team puts an entry of group `id`'s public mailbox for
    /// `pinned`: sealed under a closed channel's key in its mailbox, else
    /// in the clear in the public one. A closed channel without its key
    /// tree — made before it had one, or left — sends nothing.
    pub(super) fn public_entry_target(
        &self,
        id: &str,
        entry: Vec<u8>,
        pinned: u64,
    ) -> Result<([u8; 32], Vec<u8>), CoreError> {
        let view = self.public_view(id)?;
        let closed = view.channel && view.roster.access != Access::Public;
        match view.tree {
            Some(tree) if closed => {
                let group = view.group;
                let version =
                    u32::try_from(tree.removed.len()).map_err(|_| CoreError::InvalidState)?;
                let root = key_tree::node_key(&tree.seed, &group, tree.generation, ROOT, version);
                Ok((
                    channel_mailbox(&self.domain, &group, &root, pinned),
                    key_tree::seal_entry(&root, &group, tree.generation, version, &entry)?,
                ))
            }
            _ if closed => Err(CoreError::InvalidState),
            _ => Ok((
                agentic_mailbox_swarm::address::public_group_mailbox_id(
                    &self.domain,
                    &view.group,
                    pinned,
                ),
                entry,
            )),
        }
    }

    /// The subscribers of closed channel `id` its team knows: network id →
    /// the leaves given.
    pub(super) fn channel_subscribers(
        &self,
        id: &str,
    ) -> Result<BTreeMap<String, Vec<u32>>, CoreError> {
        Ok(self.subscribers(id)?.0)
    }

    fn subscribers(&self, id: &str) -> Result<(BTreeMap<String, Vec<u32>>, u64), CoreError> {
        self.json_state(&format!("{SUBSCRIBERS}{id}"))
    }

    fn json_state<T: serde::de::DeserializeOwned + Default>(
        &self,
        namespace: &str,
    ) -> Result<(T, u64), CoreError> {
        match self.store.state(namespace)? {
            Some(state) => Ok((
                serde_json::from_slice(&state.bytes).map_err(|_| CoreError::InvalidState)?,
                state.revision,
            )),
            None => Ok((T::default(), 0)),
        }
    }

    fn json_change<T: Serialize>(
        namespace: String,
        value: &T,
        revision: u64,
    ) -> Result<StateChange, CoreError> {
        Ok(StateChange {
            namespace,
            expected_revision: revision,
            bytes: serde_json::to_vec(value).map_err(invalid)?,
        })
    }

    /// Forget the leaves a commit removed.
    pub(super) fn forget_subscribers(&mut self, id: &str, leaves: &[u32]) -> Result<(), CoreError> {
        if leaves.is_empty() {
            return Ok(());
        }
        let (mut subscribers, revision) = self.subscribers(id)?;
        let before: usize = subscribers.values().map(Vec::len).sum();
        for given in subscribers.values_mut() {
            given.retain(|leaf| !leaves.contains(leaf));
        }
        let gone: Vec<String> = subscribers
            .iter()
            .filter(|(_, given)| given.is_empty())
            .map(|(subscriber, _)| subscriber.clone())
            .collect();
        subscribers.retain(|_, given| !given.is_empty());
        if subscribers.values().map(Vec::len).sum::<usize>() != before {
            self.store.commit_states(vec![Self::json_change(
                format!("{SUBSCRIBERS}{id}"),
                &subscribers,
                revision,
            )?])?;
        }
        // The keys of their own of those who hold no leaf any more go too,
        // and those kept aside for the leaves gone.
        let removals: Vec<(String, u64)> = gone
            .iter()
            .map(|subscriber| format!("{PERSONAL}{id}/{subscriber}"))
            .chain(
                leaves
                    .iter()
                    .map(|leaf| format!("{PERSONAL_WAITING}{id}/{leaf}")),
            )
            .filter_map(|namespace| {
                let kept = self.store.state(&namespace).ok()??;
                Some((namespace, kept.revision))
            })
            .collect();
        let (mut again, again_revision) =
            self.json_state::<Vec<String>>(&format!("{REISSUE}{id}"))?;
        let listed = again.len();
        again.retain(|subscriber| !gone.contains(subscriber));
        let states = if again.len() == listed {
            vec![]
        } else {
            vec![Self::json_change(
                format!("{REISSUE}{id}"),
                &again,
                again_revision,
            )?]
        };
        if !removals.is_empty() || !states.is_empty() {
            self.store.commit_state_maintenance(states, removals)?;
        }
        Ok(())
    }

    /// Give `subscriber` the keys of closed channel `id`: a new leaf in
    /// this profile's branch, or the one it still holds, and that leaf's
    /// path keys, sealed to its card like an invitation; the team is told
    /// where the keys hang. The same operation id is the same keys.
    pub fn channel_subscribe(
        &mut self,
        id: &str,
        subscriber: &Invitee,
        operation_id: &str,
        now: u64,
    ) -> Result<(), CoreError> {
        let view = self.public_view(id)?;
        let tree = view.tree.clone().ok_or(CoreError::InvalidInput)?;
        if !view.team {
            return Err(CoreError::Unauthorized);
        }
        let member = identity_digest(&subscriber.network_id)?;
        if view.removed.get(&hex::encode(member)) == Some(&BANNED) {
            return Err(CoreError::Banned);
        }
        let request_hash: [u8; 32] = Sha256::digest(
            [
                b"channel-keys".as_slice(),
                id.as_bytes(),
                operation_id.as_bytes(),
            ]
            .concat(),
        )
        .into();
        let operation = format!("channel-keys:{}", hex::encode(request_hash));
        if self.store.operation_message(&operation)?.is_some() {
            return self.announce_subscribers(id, now);
        }
        let own = self.store.identity()?;
        let own_digest: [u8; 32] = Sha256::digest(own.public_key).into();
        let branch = tree
            .branches
            .iter()
            .find(|(m, _)| *m == own_digest)
            .map(|(_, b)| u32::from(*b))
            .ok_or(CoreError::InvalidState)?;
        let (mut subscribers, revision) = self.subscribers(id)?;
        let (mut next, next_revision) = self.json_state::<u32>(&format!("{NEXT_LEAF}{id}"))?;
        let held = subscribers
            .get(&subscriber.network_id)
            .and_then(|given| given.iter().find(|leaf| !tree.removed.contains(leaf)))
            .copied();
        let leaf = match held {
            Some(leaf) => leaf,
            None => {
                // Past every leaf this branch ever gave, even those removed
                // and forgotten.
                let seen = subscribers
                    .values()
                    .flatten()
                    .chain(&tree.removed)
                    .filter(|leaf| *leaf >> BRANCH_SHIFT == branch)
                    .map(|leaf| (leaf & ((1 << BRANCH_SHIFT) - 1)) + 1)
                    .max()
                    .unwrap_or(0);
                let number = next.max(seen);
                if number >= 1 << BRANCH_SHIFT {
                    return Err(CoreError::InvalidInput);
                }
                next = number + 1;
                (branch << BRANCH_SHIFT) | number
            }
        };
        let group = view.group;
        let keys: Vec<(u32, [u8; 32])> = key_tree::path(leaf)
            .into_iter()
            .map(|of| {
                let v = key_tree::version(of, &tree.removed);
                (
                    v,
                    key_tree::node_key(&tree.seed, &group, tree.generation, of, v),
                )
            })
            .collect();
        let (verified, card) = self.verified_card(&subscriber.network_id, &subscriber.card, now)?;
        let Packet::IntroCard {
            addresses,
            seal_key,
            ..
        } = card
        else {
            return Err(CoreError::InvalidInput);
        };
        let wire = self.sign(
            Packet::ChannelKeys {
                group: parse_id(id)?,
                card: verified.id(),
                name: view.name.clone(),
                owner: view.owner,
                roster: view.roster_wire.clone(),
                generation: tree.generation,
                leaf,
                version: u32::try_from(tree.removed.len()).map_err(|_| CoreError::InvalidState)?,
                keys,
            },
            now,
            None,
        )?;
        let message_id = hex::encode(Sha256::digest(&wire));
        let sent = self.intro_sent_state(
            &message_id,
            verified.id(),
            seal_key,
            &subscriber.network_id,
            addresses,
        )?;
        let given = subscribers
            .entry(subscriber.network_id.clone())
            .or_default();
        if !given.contains(&leaf) {
            given.push(leaf);
        }
        let (mut announce, announce_revision) =
            self.json_state::<Vec<(String, u32)>>(&format!("{ANNOUNCE}{id}"))?;
        announce.push((subscriber.network_id.clone(), leaf));
        let mut states = vec![
            sent,
            Self::json_change(format!("{SUBSCRIBERS}{id}"), &subscribers, revision)?,
            Self::json_change(format!("{NEXT_LEAF}{id}"), &next, next_revision)?,
            Self::json_change(format!("{ANNOUNCE}{id}"), &announce, announce_revision)?,
        ];
        // Given again after a reseed onto subscribers' keys: done.
        let (mut again, again_revision) =
            self.json_state::<Vec<String>>(&format!("{REISSUE}{id}"))?;
        if again.contains(&subscriber.network_id) {
            again.retain(|other| *other != subscriber.network_id);
            states.push(Self::json_change(
                format!("{REISSUE}{id}"),
                &again,
                again_revision,
            )?);
        }
        self.store.commit_outgoing_with_retry_states_and_records(
            agentic_store::OutgoingCommit {
                operation_id: operation,
                request_hash,
                message: record(message_id, id, &own.network_id, now, true, Event::Invite)?,
                destination: subscriber.network_id.clone(),
                wire,
                states,
            },
            vec![],
            vec![],
        )?;
        self.announce_subscribers(id, now)
    }

    /// Tell the rest of the team where the keys this profile gave hang, so
    /// that any of them removes the subscriber: once no commit of this
    /// profile waits; kept until sent.
    pub(super) fn announce_subscribers(&mut self, id: &str, now: u64) -> Result<(), CoreError> {
        let namespace = format!("{ANNOUNCE}{id}");
        let (announce, revision) = self.json_state::<Vec<(String, u32)>>(&namespace)?;
        if announce.is_empty() || self.group_busy(id)? {
            return Ok(());
        }
        for (subscriber, leaf) in &announce {
            let mut e = minicbor::Encoder::new(Vec::new());
            e.array(3)
                .and_then(|e| e.str("subscribed"))
                .and_then(|e| e.str(subscriber))
                .and_then(|e| e.u32(*leaf))
                .map_err(invalid)?;
            self.send_group_notice(id, &e.into_writer(), now)?;
        }
        self.store.commit_states(vec![Self::json_change(
            namespace,
            &Vec::<(String, u32)>::new(),
            revision,
        )?])?;
        Ok(())
    }

    /// A notice of closed channel `id`'s team member `author` that it gave
    /// keys: where they hang; an application of the same id at this
    /// profile's door is answered. `None` when the notice is another.
    pub(super) fn channel_notice(
        &self,
        id: &str,
        author: &[u8; 32],
        body: &[u8],
    ) -> Result<Option<Vec<StateChange>>, CoreError> {
        let mut d = minicbor::Decoder::new(body);
        let subscribed = (|| {
            if d.array().ok()? != Some(3) || d.str().ok()? != "subscribed" {
                return None;
            }
            Some((d.str().ok()?.to_owned(), d.u32().ok()?))
        })();
        let Some((subscriber, leaf)) = subscribed else {
            return Ok(None);
        };
        let view = self.public_view(id)?;
        let from_team = *author == view.owner || view.roster.admins.contains(author);
        if view.tree.is_none() || !from_team || identity_digest(&subscriber).is_err() {
            return Ok(Some(vec![]));
        }
        // A member gives leaves of its own branch alone, none another holds.
        let digest: [u8; 32] = Sha256::digest(author).into();
        let branch = view.tree.as_ref().and_then(|tree| {
            tree.branches
                .iter()
                .find(|(member, _)| *member == digest)
                .map(|(_, branch)| u32::from(*branch))
        });
        let (mut subscribers, revision) = self.subscribers(id)?;
        if branch != Some(leaf >> BRANCH_SHIFT)
            || subscribers
                .iter()
                .any(|(other, given)| *other != subscriber && given.contains(&leaf))
        {
            return Ok(Some(vec![]));
        }
        let given = subscribers.entry(subscriber.clone()).or_default();
        if !given.contains(&leaf) {
            given.push(leaf);
        }
        let mut changes = vec![Self::json_change(
            format!("{SUBSCRIBERS}{id}"),
            &subscribers,
            revision,
        )?];
        changes.extend(self.door_answered(id, &subscriber)?);
        Ok(Some(changes))
    }

    /// Closed channel `id`'s subscribers this profile knows that are
    /// banned but still hold a leaf: a removal is due for them.
    pub fn channel_removals_due(&self, id: &str) -> Result<Vec<String>, CoreError> {
        let view = self.public_view(id)?;
        let Some(tree) = view.tree.filter(|_| view.team) else {
            return Ok(vec![]);
        };
        Ok(self
            .channel_subscribers(id)?
            .into_iter()
            .filter(|(subscriber, given)| {
                identity_digest(subscriber)
                    .is_ok_and(|digest| view.removed.get(&hex::encode(digest)) == Some(&BANNED))
                    && given.iter().any(|leaf| !tree.removed.contains(leaf))
            })
            .map(|(subscriber, _)| subscriber)
            .collect())
    }

    /// What the commit's author will publish once its commit is in: kept
    /// with the commit.
    pub(super) fn key_plan_change(
        &self,
        id: &str,
        before: &super::groups::StoredTree,
        leaves: &[u32],
        hard: bool,
    ) -> Result<StateChange, CoreError> {
        let namespace = format!("{KEY_PLAN}{id}");
        let revision = self
            .store
            .state(&namespace)?
            .map_or(0, |state| state.revision);
        let out = if hard {
            self.hard_reseed_due(id)?
        } else {
            vec![]
        };
        Self::json_change(
            namespace,
            &Some(KeyPlan {
                before: before.clone(),
                leaves: leaves.to_vec(),
                hard,
                out,
            }),
            revision,
        )
    }

    /// Turn the commit's plan into key updates: one for each leaf removed
    /// from the tree before, then, if it reseeded, the next generation's
    /// keys of every subscriber still in; each sealed under the key before
    /// the commit, for its mailbox.
    pub(super) fn channel_key_changes(&mut self, id: &str, now: u64) -> Result<(), CoreError> {
        let namespace = format!("{KEY_PLAN}{id}");
        let (plan, plan_revision) = self.json_state::<Option<KeyPlan>>(&namespace)?;
        let Some(plan) = plan else {
            return Ok(());
        };
        let before = plan.before.tree()?;
        let leaves = plan.leaves;
        let hard = plan.hard;
        let out = plan.out;
        let view = self.public_view(id)?;
        let group = view.group;
        let roster = view.roster_wire.clone();
        let start = u32::try_from(before.removed.len()).map_err(|_| CoreError::InvalidState)?;
        let root = key_tree::node_key(&before.seed, &group, before.generation, ROOT, start);
        let secret = hex::encode(key_tree::derived(&root, &group, b"mailbox"));
        let mut removed = before.removed.clone();
        let mut updates = vec![];
        for leaf in &leaves {
            let version = u32::try_from(removed.len()).map_err(|_| CoreError::InvalidState)?;
            removed.push(*leaf);
            let sealed = key_tree::rekey(&before.seed, &group, before.generation, &removed, *leaf)?;
            updates.push(KeyUpdate {
                group,
                generation: before.generation,
                version,
                removed: Some(*leaf),
                entries: sealed.into_iter().map(|s| (0, s)).collect(),
                roster: roster.clone(),
                personal: None,
            });
        }
        let mut again = vec![];
        let mut gone_out: Vec<u32> = vec![];
        if let Some(after) = view
            .tree
            .as_ref()
            .filter(|t| t.generation != before.generation)
        {
            let subscribers = self.channel_subscribers(id)?;
            // What the members out held as subscribers goes on no further
            // under a reseed onto subscribers' keys.
            let occupied: Vec<u32> = subscribers
                .iter()
                .filter(|(subscriber, _)| !out.contains(subscriber))
                .flat_map(|(_, given)| given.iter().copied())
                .filter(|leaf| !removed.contains(leaf))
                .collect();
            let version = u32::try_from(removed.len()).map_err(|_| CoreError::InvalidState)?;
            // The members out lose what they held as subscribers, for good.
            if hard {
                for member in &out {
                    if let Some(given) = subscribers.get(member) {
                        gone_out.extend(given.iter().copied());
                    }
                }
            }
            let (entries, personal) = if hard {
                // Onto each subscriber's own key, as that subscriber signed
                // it for a leaf it holds; those without one get their keys
                // again. Nothing for the members out or the banned, nor for
                // a leaf two ids claim.
                let mut personal = BTreeMap::new();
                for (subscriber, given) in &subscribers {
                    let banned = identity_digest(subscriber).is_ok_and(|digest| {
                        view.removed.get(&hex::encode(digest)) == Some(&BANNED)
                    });
                    if banned || out.contains(subscriber) {
                        continue;
                    }
                    let known = self.personal_key(id, subscriber)?;
                    for leaf in given.iter().filter(|leaf| !removed.contains(leaf)) {
                        let known = match known.get(leaf) {
                            Some(key) => Some(key.clone()),
                            None => self.waiting_key(id, *leaf)?.remove(subscriber),
                        };
                        if subscribers
                            .iter()
                            .any(|(other, held)| other != subscriber && held.contains(leaf))
                        {
                            continue;
                        }
                        match known
                            .and_then(|(key, _)| key_of(&key))
                            .filter(key_tree::is_key)
                        {
                            Some(key) => {
                                personal.insert(*leaf, key);
                            }
                            None if !again.contains(subscriber) => again.push(subscriber.clone()),
                            None => {}
                        }
                    }
                }
                let mut ephemeral = zeroize::Zeroizing::new([0; 32]);
                getrandom::fill(ephemeral.as_mut()).map_err(|_| CoreError::Randomness)?;
                let entries = key_tree::hard_reseed(
                    &after.seed,
                    &group,
                    after.generation,
                    &occupied,
                    &personal,
                    &ephemeral,
                )?;
                let public =
                    x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(*ephemeral))
                        .to_bytes();
                (entries, Some(public))
            } else {
                let entries = key_tree::reseed(
                    &before.seed,
                    &after.seed,
                    &group,
                    before.generation,
                    &removed,
                    &occupied,
                )?;
                (entries, None)
            };
            for chunk in entries.chunks(RESEED_ENTRIES) {
                updates.push(KeyUpdate {
                    group,
                    generation: before.generation,
                    version,
                    removed: None,
                    entries: chunk.to_vec(),
                    roster: roster.clone(),
                    personal,
                });
            }
        }
        let (mut docs, revision) = self.key_docs(id)?;
        for update in updates {
            let wire = self
                .store
                .sign_document(DocumentDraft {
                    domain: self.domain,
                    kind: DocumentKind::ChannelKeys,
                    authority_epoch: 0,
                    issued_at: now,
                    expires_at: None,
                    body: update.encode(),
                    extensions: BTreeMap::new(),
                })?
                .to_wire();
            // Sealed under the key before: only those who held it read who
            // left and who signed.
            let envelope = key_tree::seal_entry(
                &root,
                &group,
                before.generation,
                start,
                &[&[key_tree::KEY_UPDATE][..], &wire].concat(),
            )?;
            docs.push(KeyDoc {
                hash: hex::encode(Sha256::digest(&envelope)),
                envelope: hex::encode(&envelope),
                secret: secret.clone(),
            });
        }
        let mut states = vec![
            Self::json_change(format!("{KEY_DOCS}{id}"), &docs, revision)?,
            Self::json_change(namespace, &None::<KeyPlan>, plan_revision)?,
        ];
        if !again.is_empty() {
            let (mut listed, listed_revision) =
                self.json_state::<Vec<String>>(&format!("{REISSUE}{id}"))?;
            for subscriber in again {
                if !listed.contains(&subscriber) {
                    listed.push(subscriber);
                }
            }
            states.push(Self::json_change(
                format!("{REISSUE}{id}"),
                &listed,
                listed_revision,
            )?);
        }
        self.store.commit_states(states)?;
        self.forget_subscribers(id, &[leaves, gone_out].concat())
    }

    /// The subscribers of closed channel `id` whose keys the owner gives
    /// again: none of their own keys was known at a reseed onto them. Only
    /// those still holding a leaf, and not banned.
    pub fn channel_reissues(&self, id: &str) -> Result<Vec<String>, CoreError> {
        let listed = self.json_state::<Vec<String>>(&format!("{REISSUE}{id}"))?.0;
        if listed.is_empty() {
            return Ok(listed);
        }
        let view = self.public_view(id)?;
        let Some(tree) = view.tree else {
            return Ok(vec![]);
        };
        let subscribers = self.channel_subscribers(id)?;
        Ok(listed
            .into_iter()
            .filter(|subscriber| {
                subscribers
                    .get(subscriber)
                    .is_some_and(|given| given.iter().any(|leaf| !tree.removed.contains(leaf)))
                    && !identity_digest(subscriber)
                        .is_ok_and(|digest| view.removed.get(&hex::encode(digest)) == Some(&BANNED))
            })
            .collect())
    }

    /// The owner's record of `subscriber`'s own keys in closed channel
    /// `id`: leaf → key and when it was signed.
    fn personal_key(&self, id: &str, subscriber: &str) -> Result<PersonalKeys, CoreError> {
        Ok(self.json_state(&format!("{PERSONAL}{id}/{subscriber}"))?.0)
    }

    /// Whether `leaf` may be one the team's notes have not told of yet: in
    /// a team member's branch, at most a few past the highest it gave.
    fn leaf_expected(
        &self,
        tree: &KeyTree,
        subscribers: &BTreeMap<String, Vec<u32>>,
        leaf: u32,
    ) -> bool {
        let branch = leaf >> BRANCH_SHIFT;
        if !tree.branches.iter().any(|(_, b)| u32::from(*b) == branch) {
            return false;
        }
        let given = subscribers
            .values()
            .flatten()
            .chain(&tree.removed)
            .filter(|l| *l >> BRANCH_SHIFT == branch)
            .map(|l| (l & ((1 << BRANCH_SHIFT) - 1)) + 1)
            .max()
            .unwrap_or(0);
        leaf & ((1 << BRANCH_SHIFT) - 1) < given + EXPECTED_AHEAD
    }

    /// Keys of their own for leaf `leaf` of closed channel `id` signed by
    /// ids the team's notes did not give it yet: network id → key and when
    /// signed, a few at most.
    fn waiting_key(
        &self,
        id: &str,
        leaf: u32,
    ) -> Result<BTreeMap<String, (String, u64)>, CoreError> {
        Ok(self
            .json_state(&format!("{PERSONAL_WAITING}{id}/{leaf}"))?
            .0)
    }

    /// The team members out of closed channel `id` a reseed onto
    /// subscribers' keys is due for; none when none is.
    pub(super) fn hard_reseed_due(&self, id: &str) -> Result<Vec<String>, CoreError> {
        Ok(self
            .json_state::<Vec<String>>(&format!("{HARD_DUE}{id}"))?
            .0)
    }

    /// Mark a reseed onto subscribers' keys due for `out` too, or, with
    /// none, done.
    pub(super) fn hard_reseed_change(
        &self,
        id: &str,
        out: &[String],
    ) -> Result<StateChange, CoreError> {
        let (mut due, revision) = self.json_state::<Vec<String>>(&format!("{HARD_DUE}{id}"))?;
        if out.is_empty() {
            due.clear();
        }
        for member in out {
            if !due.contains(member) {
                due.push(member.clone());
            }
        }
        Self::json_change(format!("{HARD_DUE}{id}"), &due, revision)
    }

    /// The owner's reseed of closed channel `id` onto its subscribers' own
    /// keys once a team member is out: a commit of its own, after the one
    /// that took the member out, so the new seed never reaches it. `None`
    /// when none is due, or a commit of this profile already waits.
    pub fn channel_hard_reseed(
        &mut self,
        id: &str,
        now: u64,
    ) -> Result<Option<super::groups::GroupCommitMade>, CoreError> {
        let view = self.public_view(id)?;
        let own = self.store.identity()?.public_key;
        if own != view.owner
            || view.tree.is_none()
            || self.hard_reseed_due(id)?.is_empty()
            || self.group_busy(id)?
        {
            return Ok(None);
        }
        let change = GroupChange {
            reseed: true,
            ..GroupChange::default()
        };
        Ok(Some(self.change_group(
            id,
            change,
            &format!("hard-reseed:{}", view.epoch),
            now,
        )?))
    }

    fn key_docs(&self, id: &str) -> Result<(Vec<KeyDoc>, u64), CoreError> {
        self.json_state(&format!("{KEY_DOCS}{id}"))
    }

    /// Closed channel `id`'s key updates this profile publishes, for the
    /// mailbox of the key before, stamped: the same, under the same stamp,
    /// until a copy is read back or stored at a quorum.
    pub fn channel_key_docs(
        &mut self,
        id: &str,
        now: u64,
    ) -> Result<Vec<SwarmDelivery>, CoreError> {
        // A plan the commit left, should the updates not have been made:
        // what is made goes out even if the plan cannot be made yet.
        let _ = self.channel_key_changes(id, now);
        let (docs, _) = self.key_docs(id)?;
        let pinned = period(now);
        let mut deliveries = vec![];
        for doc in docs {
            let secret = unhex32(&doc.secret)?;
            let mailbox = agentic_mailbox_swarm::address::mailbox_id(&self.domain, &secret, pinned);
            let envelope = unhex(&doc.envelope)?;
            let (stamp, changes) = self.stamp_changes(&mailbox, pinned, &envelope, now)?;
            if !changes.is_empty() {
                self.store.commit_states(changes)?;
            }
            deliveries.push(SwarmDelivery {
                message_id: String::new(),
                conversation_id: id.into(),
                period: pinned,
                mailbox,
                envelope,
                stamp,
            });
        }
        Ok(deliveries)
    }

    /// A key update of closed channel `id` (its entry's hash in hex) this
    /// node stored at a quorum, or read back: published.
    pub fn channel_key_doc_stored(&mut self, id: &str, hash: &str) -> Result<(), CoreError> {
        let (mut docs, revision) = self.key_docs(id)?;
        let before = docs.len();
        docs.retain(|doc| doc.hash != hash);
        if docs.len() != before {
            self.store.commit_states(vec![Self::json_change(
                format!("{KEY_DOCS}{id}"),
                &docs,
                revision,
            )?])?;
        }
        Ok(())
    }

    /// Take a key update read, sealed, from closed channel `conversation`'s
    /// mailbox (`entry` is what was read, `wire` the update inside): a
    /// subscriber applies it in the log's order; the team notes its own
    /// read back.
    pub(super) fn take_key_update(
        &mut self,
        conversation: &str,
        owner: &str,
        entry: &[u8],
        wire: &[u8],
        now: u64,
    ) -> Result<PublicEntry, CoreError> {
        let Some(mut keys) = self.follow_keys(conversation)? else {
            self.channel_key_doc_stored(conversation, &hex::encode(Sha256::digest(entry)))?;
            return Ok(PublicEntry::Ignored);
        };
        let group = parse_id(conversation)?;
        let mut known: Option<Roster> = None;
        let newer = |known: &mut Option<Roster>, roster: Roster| {
            if known.as_ref().is_none_or(|k| roster.version > k.version) {
                *known = Some(roster);
            }
        };
        match self.apply_update(conversation, &mut keys, &group, owner, wire, now)? {
            Update::Applied(roster) => newer(&mut known, roster),
            Update::Staged(roster) => {
                self.set_follow_keys(conversation, &keys)?;
                self.note_roster(conversation, &roster)?;
                return Ok(PublicEntry::Ignored);
            }
            Update::Wait => {
                let held = hex::encode(wire);
                let bytes: usize = keys.waiting.iter().map(String::len).sum();
                if !keys.waiting.contains(&held)
                    && keys.waiting.len() < MAX_WAITING_UPDATES
                    && bytes + held.len() <= MAX_WAITING_BYTES
                {
                    keys.waiting.push(held);
                    self.set_follow_keys(conversation, &keys)?;
                }
                return Ok(PublicEntry::Ignored);
            }
            Update::No => return Ok(PublicEntry::Ignored),
        }
        // Those that waited for it may now apply.
        loop {
            let waiting = std::mem::take(&mut keys.waiting);
            let mut progressed = false;
            for held in waiting {
                let Ok(held_wire) = hex::decode(&held) else {
                    continue;
                };
                match self.apply_update(conversation, &mut keys, &group, owner, &held_wire, now)? {
                    Update::Applied(roster) | Update::Staged(roster) => {
                        newer(&mut known, roster);
                        progressed = true;
                    }
                    Update::Wait => keys.waiting.push(held),
                    Update::No => {}
                }
            }
            if !progressed {
                break;
            }
        }
        self.set_follow_keys(conversation, &keys)?;
        // The roster of what it took is the newest known now, if newer:
        // whoever it left out signs for nobody.
        if let Some(roster) = known {
            self.note_roster(conversation, &roster)?;
        }
        Ok(PublicEntry::Rekeyed)
    }

    /// The documents of their own keys this profile's closed-channel follows
    /// still have to publish, each sealed under its leaf's key and the
    /// channel's key now, in its mailbox, stamped: the same, under the same
    /// stamp, until read back or stored at a quorum.
    pub fn channel_hellos(&mut self, now: u64) -> Result<Vec<SwarmDelivery>, CoreError> {
        let mut deliveries = vec![];
        for conversation in self.follow_ids()? {
            // One follow that cannot publish holds up none of the others.
            if let Ok(Some(delivery)) = self.channel_hello(&conversation, now) {
                deliveries.push(delivery);
            }
        }
        Ok(deliveries)
    }

    fn channel_hello(
        &mut self,
        conversation: &str,
        now: u64,
    ) -> Result<Option<SwarmDelivery>, CoreError> {
        let Some(mut keys) = self.follow_keys(conversation)? else {
            return Ok(None);
        };
        if keys.announced == Some(keys.generation) {
            return Ok(None);
        }
        let group = parse_id(conversation)?;
        let mut changed = false;
        if keys.personal.is_none() {
            let mut secret = zeroize::Zeroizing::new([0; 32]);
            getrandom::fill(secret.as_mut()).map_err(|_| CoreError::Randomness)?;
            keys.personal = Some(hex::encode(*secret));
            changed = true;
        }
        let public = keys.personal_public().ok_or(CoreError::InvalidState)?;
        if keys
            .hello
            .as_ref()
            .is_none_or(|(generation, _)| *generation != keys.generation)
        {
            let wire = self
                .store
                .sign_document(DocumentDraft {
                    domain: self.domain,
                    kind: DocumentKind::ChannelSubscriber,
                    authority_epoch: 0,
                    issued_at: now,
                    expires_at: None,
                    body: SubscriberKey {
                        group,
                        leaf: keys.leaf,
                        key: public,
                    }
                    .encode(),
                    extensions: BTreeMap::new(),
                })?
                .to_wire();
            keys.hello = Some((keys.generation, hex::encode(wire)));
            changed = true;
        }
        let Some((_, wire)) = keys.hello.clone() else {
            return Ok(None);
        };
        let leaf_node = key_tree::ancestor(keys.leaf, key_tree::DEPTH);
        let leaf_key = keys
            .path
            .get(&leaf_node)
            .and_then(|(_, key)| key_of(key))
            .ok_or(CoreError::InvalidState)?;
        let root = keys.root().ok_or(CoreError::InvalidState)?;
        let inner = key_tree::seal_hello(
            &leaf_key,
            &group,
            keys.generation,
            keys.leaf,
            &unhex(&wire)?,
        )?;
        let envelope = key_tree::seal_entry(&root, &group, keys.generation, keys.version, &inner)?;
        let sealed = hex::encode(Sha256::digest(&envelope));
        if keys.hello_sealed.as_deref() != Some(sealed.as_str()) {
            keys.hello_sealed = Some(sealed);
            changed = true;
        }
        if changed {
            self.set_follow_keys(conversation, &keys)?;
        }
        let pinned = period(now);
        let mailbox = channel_mailbox(&self.domain, &group, &root, pinned);
        let (stamp, changes) = self.stamp_changes(&mailbox, pinned, &envelope, now)?;
        if !changes.is_empty() {
            self.store.commit_states(changes)?;
        }
        Ok(Some(SwarmDelivery {
            message_id: String::new(),
            conversation_id: conversation.into(),
            period: pinned,
            mailbox,
            envelope,
            stamp,
        }))
    }

    /// This profile's key document for closed channel `conversation`, in the
    /// envelope of hash `hash` (hex), was stored at a quorum: published, if
    /// it is the one going out now.
    pub fn channel_hello_stored(
        &mut self,
        conversation: &str,
        hash: &str,
    ) -> Result<(), CoreError> {
        let Some(mut keys) = self.follow_keys(conversation)? else {
            return Ok(());
        };
        if let Some((generation, _)) = keys.hello
            && keys.hello_sealed.as_deref() == Some(hash)
            && keys.announced != Some(generation)
        {
            keys.announced = Some(generation);
            self.set_follow_keys(conversation, &keys)?;
        }
        Ok(())
    }

    /// A subscriber's key entry read from closed channel `conversation`'s
    /// mailbox (`inner`, as sealed under its leaf's key): the owner keeps
    /// the key by who signed it, for the leaf named, the newest signed; a
    /// subscriber that reads its own back has published it.
    pub(super) fn take_subscriber_key(
        &mut self,
        conversation: &str,
        team: bool,
        inner: &[u8],
        now: u64,
    ) -> Result<PublicEntry, CoreError> {
        let Some((leaf, generation)) = key_tree::hello_of(inner) else {
            return Ok(PublicEntry::Ignored);
        };
        if !team {
            let Some(mut keys) = self.follow_keys(conversation)? else {
                return Ok(PublicEntry::Ignored);
            };
            let group = parse_id(conversation)?;
            let leaf_node = key_tree::ancestor(keys.leaf, key_tree::DEPTH);
            let opened = keys
                .path
                .get(&leaf_node)
                .and_then(|(_, key)| key_of(key))
                .filter(|_| (leaf, generation) == (keys.leaf, keys.generation))
                .and_then(|leaf_key| key_tree::open_hello(&leaf_key, &group, inner));
            if let (Some(doc), Some((generation, hello))) = (opened, &keys.hello)
                && unhex(hello)? == doc
                && keys.announced != Some(*generation)
            {
                keys.announced = Some(*generation);
                self.set_follow_keys(conversation, &keys)?;
            }
            return Ok(PublicEntry::Ignored);
        }
        let view = self.public_view(conversation)?;
        let Some(tree) = view.tree.as_ref() else {
            return Ok(PublicEntry::Ignored);
        };
        if self.store.identity()?.public_key != view.owner || generation != tree.generation {
            return Ok(PublicEntry::Ignored);
        }
        let leaf_key = key_tree::node_key(
            &tree.seed,
            &view.group,
            generation,
            key_tree::ancestor(leaf, key_tree::DEPTH),
            0,
        );
        let Some(doc) = key_tree::open_hello(&leaf_key, &view.group, inner) else {
            return Ok(PublicEntry::Ignored);
        };
        let Ok(document) = VerifiedDocument::decode(&doc, self.domain, now) else {
            return Ok(PublicEntry::Ignored);
        };
        let Ok((key, signer)) = verify_subscriber_key(&doc, self.domain, now, &view.group) else {
            return Ok(PublicEntry::Ignored);
        };
        if key.leaf != leaf || !key_tree::is_key(&key.key) {
            return Ok(PublicEntry::Ignored);
        }
        let subscriber = network_id(&signer);
        let signed = document.issued_at();
        let key_hex = hex::encode(key.key);
        let subscribers = self.channel_subscribers(conversation)?;
        let holder = subscribers
            .iter()
            .find(|(_, given)| given.contains(&leaf))
            .map(|(id, _)| id.clone());
        match holder {
            // The leaf is another's.
            Some(holder) if holder != subscriber => Ok(PublicEntry::Ignored),
            Some(_) => {
                let namespace = format!("{PERSONAL}{conversation}/{subscriber}");
                let (mut known, revision) = self.json_state::<PersonalKeys>(&namespace)?;
                if known
                    .get(&leaf)
                    .is_some_and(|(held, at)| *held == key_hex || *at >= signed)
                {
                    return Ok(PublicEntry::Ignored);
                }
                known.insert(leaf, (key_hex, signed));
                self.store
                    .commit_states(vec![Self::json_change(namespace, &known, revision)?])?;
                Ok(PublicEntry::SubscriberKey)
            }
            // Not told of yet: kept aside, a few per leaf, and only just past
            // what its branch gave so far.
            None if !self.leaf_expected(tree, &subscribers, leaf) => Ok(PublicEntry::Ignored),
            None => {
                let namespace = format!("{PERSONAL_WAITING}{conversation}/{leaf}");
                let (mut waiting, revision) =
                    self.json_state::<BTreeMap<String, (String, u64)>>(&namespace)?;
                if waiting
                    .get(&subscriber)
                    .is_some_and(|(held, at)| *held == key_hex || *at >= signed)
                {
                    return Ok(PublicEntry::Ignored);
                }
                waiting.insert(subscriber, (key_hex, signed));
                while waiting.len() > WAITING_KEYS {
                    let Some(oldest) = waiting
                        .iter()
                        .min_by_key(|(_, (_, at))| *at)
                        .map(|(id, _)| id.clone())
                    else {
                        break;
                    };
                    waiting.remove(&oldest);
                }
                self.store
                    .commit_states(vec![Self::json_change(namespace, &waiting, revision)?])?;
                Ok(PublicEntry::SubscriberKey)
            }
        }
    }

    fn apply_update(
        &self,
        conversation: &str,
        keys: &mut FollowKeys,
        group: &[u8; 32],
        owner: &str,
        wire: &[u8],
        now: u64,
    ) -> Result<Update, CoreError> {
        let Ok(verified) = verify_key_update(wire, self.domain, now, group) else {
            return Ok(Update::No);
        };
        // Signed under the newest roster known, by someone not removed: a
        // former admin, who knows the seed, moves nobody's keys.
        if network_id(&verified.owner) != owner
            || !self.signer_in_force(conversation, &verified.roster, &verified.signer)?
        {
            return Ok(Update::No);
        }
        let update = verified.update;
        // The owner's reseed onto subscribers' keys says itself which
        // generation it counts from.
        if let Some(ephemeral) = update.personal {
            return Ok(self.apply_hard(keys, group, update, verified.roster, &ephemeral, now));
        }
        if update.generation != keys.generation {
            return Ok(Update::No);
        }
        if update.version > keys.version {
            return Ok(Update::Wait);
        }
        if update.version < keys.version {
            return Ok(Update::No);
        }
        let (Some(path), Some(_)) = (from_hex(&keys.path), keys.root()) else {
            return Ok(Update::No);
        };
        match update.removed {
            Some(leaf) => {
                let sealed: Vec<Vec<u8>> = update.entries.into_iter().map(|(_, s)| s).collect();
                let (Some(next), Some(version)) = (
                    key_tree::apply_rekey(&path, keys.leaf, group, keys.generation, leaf, &sealed),
                    keys.version.checked_add(1),
                ) else {
                    return Ok(Update::No);
                };
                if keys.retire(now).is_none() {
                    return Ok(Update::No);
                }
                keys.path = to_hex(&next);
                keys.version = version;
                keys.roster = keys.roster.max(verified.roster.version);
            }
            None => {
                let Some(mut staged) = from_hex(&keys.staged) else {
                    return Ok(Update::No);
                };
                let found = staged.len();
                key_tree::apply_reseed(&path, &mut staged, group, keys.generation, &update.entries);
                if !path.keys().all(|of| staged.contains_key(of)) {
                    if staged.len() == found {
                        return Ok(Update::No);
                    }
                    keys.staged = to_hex(&staged);
                    return Ok(Update::Staged(verified.roster));
                }
                let Some(generation) = keys.generation.checked_add(1) else {
                    return Ok(Update::No);
                };
                if keys.retire(now).is_none() {
                    return Ok(Update::No);
                }
                keys.generation = generation;
                keys.version = 0;
                keys.path = to_hex(&staged);
                keys.clear_staging();
                keys.roster = keys.roster.max(verified.roster.version);
            }
        }
        Ok(Update::Applied(verified.roster))
    }

    /// Take a reseed onto this subscriber's own key, signed by the owner:
    /// from any version of the generation it holds, or — when a former
    /// admin moved it on under an older roster than this one — of any
    /// generation before. Its keys come together over its documents, in any
    /// order.
    fn apply_hard(
        &self,
        keys: &mut FollowKeys,
        group: &[u8; 32],
        update: KeyUpdate,
        roster: Roster,
        ephemeral: &[u8; 32],
        now: u64,
    ) -> Update {
        let from = update.generation;
        let undone = from != keys.generation && keys.roster < roster.version;
        let (Some(next), Some(personal)) = (
            from.checked_add(1),
            keys.personal.as_deref().and_then(key_of),
        ) else {
            return Update::No;
        };
        if from != keys.generation && !undone {
            return Update::No;
        }
        if keys.hard != Some(from) {
            keys.hard = Some(from);
            keys.hard_staged.clear();
            keys.pending.clear();
        }
        let Some(mut staged) = from_hex(&keys.hard_staged) else {
            return Update::No;
        };
        let mut pending: BTreeMap<u64, Vec<Vec<u8>>> = keys
            .pending
            .iter()
            .map(|(of, waiting)| {
                (
                    *of,
                    waiting.iter().filter_map(|w| hex::decode(w).ok()).collect(),
                )
            })
            .collect();
        let before = (staged.len(), pending.values().map(Vec::len).sum::<usize>());
        key_tree::apply_hard_reseed(
            &personal,
            ephemeral,
            keys.leaf,
            group,
            next,
            &update.entries,
            &mut staged,
            &mut pending,
        );
        let path = key_tree::path(keys.leaf);
        if !path.iter().all(|of| staged.contains_key(of)) {
            if before == (staged.len(), pending.values().map(Vec::len).sum::<usize>()) {
                return Update::No;
            }
            keys.hard_staged = to_hex(&staged);
            keys.pending = pending
                .into_iter()
                .map(|(of, waiting)| (of, waiting.iter().map(hex::encode).collect()))
                .collect();
            return Update::Staged(roster);
        }
        if keys.retire(now).is_none() {
            return Update::No;
        }
        let published = keys.announced.is_some();
        keys.generation = next;
        keys.version = 0;
        keys.path = to_hex(&staged);
        keys.clear_staging();
        keys.hard = None;
        keys.hard_staged.clear();
        keys.pending.clear();
        keys.roster = keys.roster.max(roster.version);
        // The owner had its key: nothing to publish again.
        if published {
            keys.announced = Some(next);
            keys.hello = None;
        }
        Update::Applied(roster)
    }

    /// Take a closed channel's keys sent to this profile: from its owner or
    /// an admin under a roster no older than the one known, for one of this
    /// profile's cards, not behind the keys it holds; let in if it knocked
    /// at this very channel's door or its contact policy lets the sender
    /// in, else waiting for its decision.
    pub(super) fn take_channel_keys(
        &mut self,
        verified: &VerifiedDocument,
        packet: Packet,
        card_id: Option<&str>,
        now: u64,
    ) -> Result<IntroOutcome, CoreError> {
        let Packet::ChannelKeys { card, .. } = &packet else {
            return Err(CoreError::InvalidInput);
        };
        if card_id.is_some_and(|id| id != hex::encode(card))
            || !self.own_card_takes(card, verified.issued_at(), now)?
        {
            return Err(CoreError::InvalidInvitation);
        }
        self.take_keys_packet(verified, packet, now)
    }

    /// Keys of `packet` for one of this profile's cards: taken, waiting for
    /// its decision, or passed over.
    pub(super) fn take_keys_packet(
        &mut self,
        verified: &VerifiedDocument,
        packet: Packet,
        now: u64,
    ) -> Result<IntroOutcome, CoreError> {
        let Packet::ChannelKeys {
            group,
            owner,
            roster,
            generation,
            leaf,
            version,
            ..
        } = &packet
        else {
            return Err(CoreError::InvalidInput);
        };
        let signer = verified.author();
        if *signer == self.store.identity()?.public_key {
            return Err(CoreError::Unauthorized);
        }
        let reference = group_ref(&self.domain, owner, group);
        let verified_roster = verify_roster(roster, self.domain, now, owner, &reference)?;
        if verified_roster.kind != GroupKind::Channel
            || (signer != owner && !verified_roster.admins.contains(signer))
        {
            return Err(CoreError::Unauthorized);
        }
        let conversation = hex::encode(reference);
        if !self.signer_in_force(&conversation, &verified_roster, signer)? {
            return Ok(IntroOutcome::Ignored);
        }
        // Again, or behind the keys held: nothing new. The owner's keys
        // under a newer roster come first whatever their generation: a
        // former admin's, under the roster it was in, hold nobody back.
        let held = self.follow_keys(&conversation)?;
        if let Some(keys) = &held {
            let behind = if keys.roster == 0 || signer != owner {
                (*generation, *version) < (keys.generation, keys.version)
            } else {
                (verified_roster.version, *generation, *version)
                    < (keys.roster, keys.generation, keys.version)
            };
            if behind
                || (*generation, *version, *leaf) == (keys.generation, keys.version, keys.leaf)
            {
                return Ok(IntroOutcome::Ignored);
            }
        }
        // The owner's keys for a channel it follows already — given again
        // after new keys all round — are taken as the ones before were.
        let again = held.is_some() && signer == owner;
        if !again && !self.keys_allowed(verified, group, &reference, now)? {
            return self.keys_waiting(verified, &packet, now);
        }
        self.subscribe_with(packet, verified.issued_at().min(now), now)?;
        Ok(IntroOutcome::Subscribed(conversation))
    }

    /// Follow a closed channel with the keys of `packet`, reading what was
    /// stored from `since` on — when the keys were given.
    pub(super) fn subscribe_with(
        &mut self,
        packet: Packet,
        since: u64,
        now: u64,
    ) -> Result<String, CoreError> {
        let Packet::ChannelKeys {
            group,
            name,
            owner,
            roster,
            generation,
            leaf,
            version,
            keys,
            ..
        } = packet
        else {
            return Err(CoreError::InvalidInput);
        };
        let reference = group_ref(&self.domain, &owner, &group);
        let verified_roster = verify_roster(&roster, self.domain, now, &owner, &reference)?;
        let path = key_tree::path(leaf);
        if keys.first().is_none_or(|(v, _)| *v != version) || keys.len() != path.len() {
            return Err(CoreError::InvalidInput);
        }
        let conversation = hex::encode(reference);
        let held = self.follow_keys(&conversation)?;
        let held_roster = held.as_ref().map_or(0, |held| held.roster);
        // The keys held go on being read a while: what was sealed under
        // them — the owner's new keys among it — still opens.
        let before = match held.clone() {
            Some(mut held) => {
                held.retire(now);
                held.before
            }
            None => vec![],
        };
        // Its own key stays the same for the channel; given keys, it
        // publishes it again under them.
        let personal = match held.and_then(|held| held.personal) {
            Some(held) => held,
            None => {
                let mut secret = zeroize::Zeroizing::new([0; 32]);
                getrandom::fill(secret.as_mut()).map_err(|_| CoreError::Randomness)?;
                hex::encode(*secret)
            }
        };
        let follow = FollowKeys {
            generation,
            leaf,
            version,
            path: path
                .into_iter()
                .zip(keys)
                .map(|(of, (v, key))| (of, (v, hex::encode(key))))
                .collect(),
            before,
            staged: BTreeMap::new(),
            waiting: vec![],
            personal: Some(personal),
            announced: None,
            hello: None,
            roster: held_roster.max(verified_roster.version),
            hard: None,
            hard_staged: BTreeMap::new(),
            pending: BTreeMap::new(),
            hello_sealed: None,
        };
        self.follow_with_keys(
            reference,
            &name,
            &network_id(&owner),
            follow,
            &verified_roster,
            since,
        )?;
        self.forget_knock(&hex::encode(group))?;
        Ok(conversation)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use agentic_protocol::group::{PublicRoster, Retention, Roster};
    use ed25519_dalek::SigningKey;

    const DOMAIN: [u8; 32] = [3; 32];
    const NOW: u64 = 1_788_570_000;
    const MLS_GROUP: [u8; 32] = [9; 32];
    const LEAF: u32 = 0x0100_0002;
    const NEIGHBOUR: u32 = 0x0100_0003;

    fn signed(kind: DocumentKind, body: Vec<u8>, key: &SigningKey) -> Vec<u8> {
        agentic_protocol::SignedDocument::sign(
            DocumentDraft {
                domain: DOMAIN,
                kind,
                authority_epoch: 0,
                issued_at: NOW,
                expires_at: None,
                body,
                extensions: BTreeMap::new(),
            },
            key,
        )
        .unwrap()
        .to_wire()
    }

    /// A closed channel of `owner` with `admin` in its team, and Dave, who
    /// took keys to its leaf `LEAF` of seed `SEED0` under the roster with
    /// the admin (version 1).
    struct Channel {
        owner: SigningKey,
        admin: SigningKey,
        reference: [u8; 32],
        with_admin: Vec<u8>,
        without: Vec<u8>,
        dave: AppCore,
        conversation: String,
        _dir: tempfile::TempDir,
    }

    const SEED0: [u8; 32] = [21; 32];
    const SEED1: [u8; 32] = [77; 32];
    const EPHEMERAL: [u8; 32] = [88; 32];

    fn roster(
        owner: &SigningKey,
        reference: [u8; 32],
        version: u64,
        admins: &[&SigningKey],
    ) -> Vec<u8> {
        signed(
            DocumentKind::GroupRoster,
            Roster {
                group: reference,
                version,
                admins: admins
                    .iter()
                    .map(|a| a.verifying_key().to_bytes())
                    .collect(),
                access: Access::Private,
                kind: GroupKind::Channel,
            }
            .encode(),
            owner,
        )
    }

    fn channel() -> Channel {
        let owner = SigningKey::from_bytes(&[1; 32]);
        let admin = SigningKey::from_bytes(&[2; 32]);
        let o = owner.verifying_key().to_bytes();
        let reference = group_ref(&DOMAIN, &o, &MLS_GROUP);
        let dir = tempfile::TempDir::new().unwrap();
        let store = ProfileStore::open(dir.path().join("profile.db"), &[7; 32]).unwrap();
        let mut dave = AppCore::new(store, DOMAIN).unwrap();
        dave.create_profile("Dave").unwrap();
        let keys = key_tree::path(LEAF)
            .into_iter()
            .map(|of| (0, key_tree::node_key(&SEED0, &reference, 0, of, 0)))
            .collect();
        let with_admin = roster(&owner, reference, 1, &[&admin]);
        let without = roster(&owner, reference, 2, &[]);
        let conversation = dave
            .subscribe_with(
                Packet::ChannelKeys {
                    group: MLS_GROUP,
                    card: [0; 32],
                    name: "Club".into(),
                    owner: o,
                    roster: with_admin.clone(),
                    generation: 0,
                    leaf: LEAF,
                    version: 0,
                    keys,
                },
                NOW,
                NOW,
            )
            .unwrap();
        Channel {
            owner,
            admin,
            reference,
            with_admin,
            without,
            dave,
            conversation,
            _dir: dir,
        }
    }

    impl Channel {
        fn root(&self) -> (u32, [u8; 32]) {
            let keys = self.dave.follow_keys(&self.conversation).unwrap().unwrap();
            (keys.generation, keys.root().unwrap())
        }

        fn take(&mut self, signer: &SigningKey, update: &KeyUpdate) -> PublicEntry {
            let wire = signed(DocumentKind::ChannelKeys, update.encode(), signer);
            let owner = network_id(&self.owner.verifying_key().to_bytes());
            self.dave
                .take_key_update(&self.conversation, &owner, &[], &wire, NOW)
                .unwrap()
        }

        /// The owner's reseed onto Dave's own key, from `version` of
        /// generation 0, under `roster`, in `parts` documents.
        fn hard(&self, version: u32, roster: &[u8], parts: usize) -> Vec<KeyUpdate> {
            let personal = self
                .dave
                .follow_keys(&self.conversation)
                .unwrap()
                .unwrap()
                .personal_public()
                .unwrap();
            let entries = key_tree::hard_reseed(
                &SEED1,
                &self.reference,
                1,
                &[LEAF, NEIGHBOUR],
                &BTreeMap::from([(LEAF, personal)]),
                &EPHEMERAL,
            )
            .unwrap();
            let size = entries.len().div_ceil(parts);
            entries
                .chunks(size)
                .map(|chunk| KeyUpdate {
                    group: self.reference,
                    generation: 0,
                    version,
                    removed: None,
                    entries: chunk.to_vec(),
                    roster: roster.to_vec(),
                    personal: Some(
                        x25519_dalek::PublicKey::from(&x25519_dalek::StaticSecret::from(EPHEMERAL))
                            .to_bytes(),
                    ),
                })
                .collect()
        }

        /// What the owner publishes sealed after the admin is out: the
        /// roster without it, read by Dave under the key it still keeps.
        fn learn_roster(&mut self, root: &[u8; 32]) -> PublicEntry {
            let body = PublicRoster {
                group: self.reference,
                epoch: 2,
                roster: self.without.clone(),
                removed: vec![],
                retention: Retention::DEFAULT,
            }
            .encode();
            let wire = signed(DocumentKind::PublicRoster, body, &self.owner);
            let entry = key_tree::seal_entry(
                root,
                &self.reference,
                0,
                0,
                &[&[PUBLIC_ROSTER][..], &wire].concat(),
            )
            .unwrap();
            self.dave
                .receive_public_entry(
                    &self.conversation,
                    &[&[key_tree::SEALED_ENTRY][..], &entry[1..]].concat(),
                    NOW,
                    NOW,
                )
                .unwrap()
        }
    }

    /// An admin taken off the team still holds the old seed and may sign
    /// under the roster it was in. Dave takes its reseed before learning the
    /// roster without it; learning that roster does not undo it, the
    /// owner's reseed onto Dave's own key does — in two documents, either
    /// order, taken once — because it comes under a roster newer than the
    /// one Dave took his generation under. After that the former admin
    /// moves nobody; a reseed onto subscribers' keys signed by an admin, or
    /// under the roster Dave already took his keys under, never counts.
    #[test]
    fn the_owners_reseed_onto_a_subscribers_key_undoes_a_former_admins_reseed() {
        let mut c = channel();
        let root0 = key_tree::node_key(&SEED0, &c.reference, 0, ROOT, 0);
        let fake_seed = [66; 32];
        let fake = KeyUpdate {
            group: c.reference,
            generation: 0,
            version: 0,
            removed: None,
            entries: key_tree::reseed(&SEED0, &fake_seed, &c.reference, 0, &[], &[LEAF]).unwrap(),
            roster: c.with_admin.clone(),
            personal: None,
        };
        let admin = c.admin.clone();
        assert_eq!(c.take(&admin, &fake), PublicEntry::Rekeyed);
        let forked = key_tree::node_key(&fake_seed, &c.reference, 1, ROOT, 0);
        assert_eq!(c.root(), (1, forked));

        // Signed by the owner under the roster Dave took the fork under:
        // not newer, no reason to move.
        let owner = c.owner.clone();
        let stale = c.hard(0, &c.with_admin.clone(), 1);
        assert_eq!(c.take(&owner, &stale[0]), PublicEntry::Ignored);
        // Dave learns the roster without the admin; he is still forked.
        assert_eq!(c.learn_roster(&root0), PublicEntry::Roster);
        assert_eq!(c.root(), (1, forked));
        // Signed by an admin, a reseed onto subscribers' keys never counts.
        let without = c.without.clone();
        let docs = c.hard(0, &without, 1);
        assert_eq!(c.take(&admin, &docs[0]), PublicEntry::Ignored);
        assert_eq!(c.take(&owner, &docs[0]), PublicEntry::Rekeyed);
        let real = key_tree::node_key(&SEED1, &c.reference, 1, ROOT, 0);
        assert_eq!(c.root(), (1, real));
        // The former admin's reseed of the real generation — as if it knew
        // the seed — under the roster it was in moves nobody now.
        let again = KeyUpdate {
            group: c.reference,
            generation: 1,
            version: 0,
            removed: None,
            entries: key_tree::reseed(&SEED1, &[99; 32], &c.reference, 1, &[], &[LEAF]).unwrap(),
            roster: c.with_admin.clone(),
            personal: None,
        };
        assert_eq!(c.take(&admin, &again), PublicEntry::Ignored);
        assert_eq!(c.root(), (1, real));
    }

    /// A former admin's forged removal moves Dave one version on; the
    /// owner's reseed onto his key, from the version before, still takes
    /// him to the new generation.
    #[test]
    fn the_owners_reseed_onto_a_subscribers_key_counts_from_any_version_of_its_generation() {
        let mut c = channel();
        let removal = KeyUpdate {
            group: c.reference,
            generation: 0,
            version: 0,
            removed: Some(NEIGHBOUR),
            entries: key_tree::rekey(&SEED0, &c.reference, 0, &[NEIGHBOUR], NEIGHBOUR)
                .unwrap()
                .into_iter()
                .map(|sealed| (0, sealed))
                .collect(),
            roster: c.with_admin.clone(),
            personal: None,
        };
        let admin = c.admin.clone();
        assert_eq!(c.take(&admin, &removal), PublicEntry::Rekeyed);
        assert_eq!(
            c.dave
                .follow_keys(&c.conversation)
                .unwrap()
                .unwrap()
                .version,
            1
        );
        let owner = c.owner.clone();
        let without = c.without.clone();
        for doc in c.hard(0, &without, 1) {
            assert_eq!(c.take(&owner, &doc), PublicEntry::Rekeyed);
        }
        let real = key_tree::node_key(&SEED1, &c.reference, 1, ROOT, 0);
        assert_eq!(c.root(), (1, real));
        // The roster the owner's reseed came under is the one known now:
        // the former admin's reseed of the real generation — as if it knew
        // the seed — under the roster it was in moves nobody.
        let again = KeyUpdate {
            group: c.reference,
            generation: 1,
            version: 0,
            removed: None,
            entries: key_tree::reseed(&SEED1, &[99; 32], &c.reference, 1, &[], &[LEAF]).unwrap(),
            roster: c.with_admin.clone(),
            personal: None,
        };
        assert_eq!(c.take(&admin, &again), PublicEntry::Ignored);
        assert_eq!(c.root(), (1, real));
    }

    impl Channel {
        /// Keys to the same leaf, of `seed` at `generation`, as `signer`
        /// gives them under `roster`.
        fn keys(
            &self,
            signer: &SigningKey,
            roster: &[u8],
            seed: &[u8; 32],
            generation: u32,
        ) -> (VerifiedDocument, Packet) {
            let packet = Packet::ChannelKeys {
                group: MLS_GROUP,
                card: [0; 32],
                name: "Club".into(),
                owner: self.owner.verifying_key().to_bytes(),
                roster: roster.to_vec(),
                generation,
                leaf: LEAF,
                version: 0,
                keys: key_tree::path(LEAF)
                    .into_iter()
                    .map(|of| {
                        (
                            0,
                            key_tree::node_key(seed, &self.reference, generation, of, 0),
                        )
                    })
                    .collect(),
            };
            let wire = signed(packet.kind(), packet.encode().unwrap(), signer);
            (
                VerifiedDocument::decode(&wire, DOMAIN, NOW).unwrap(),
                packet,
            )
        }

        fn give(
            &mut self,
            signer: &SigningKey,
            roster: &[u8],
            seed: &[u8; 32],
            generation: u32,
        ) -> IntroOutcome {
            let (verified, packet) = self.keys(signer, roster, seed, generation);
            self.dave.take_keys_packet(&verified, packet, NOW).unwrap()
        }
    }

    /// Two reseeds of a former admin, one on the other, are undone by the
    /// owner's reseed onto Dave's key all the same.
    #[test]
    fn the_owners_reseed_onto_a_subscribers_key_undoes_two_reseeds_of_a_former_admin() {
        let mut c = channel();
        let admin = c.admin.clone();
        let (first, second) = ([66; 32], [67; 32]);
        for (generation, from, to) in [(0, SEED0, first), (1, first, second)] {
            let fake = KeyUpdate {
                group: c.reference,
                generation,
                version: 0,
                removed: None,
                entries: key_tree::reseed(&from, &to, &c.reference, generation, &[], &[LEAF])
                    .unwrap(),
                roster: c.with_admin.clone(),
                personal: None,
            };
            assert_eq!(c.take(&admin, &fake), PublicEntry::Rekeyed);
        }
        assert_eq!(c.root().0, 2);
        let owner = c.owner.clone();
        let without = c.without.clone();
        for doc in c.hard(0, &without, 1) {
            assert_eq!(c.take(&owner, &doc), PublicEntry::Rekeyed);
        }
        let real = key_tree::node_key(&SEED1, &c.reference, 1, ROOT, 0);
        assert_eq!(c.root(), (1, real));
    }

    /// A former admin's keys far ahead, under the roster it was in, move a
    /// subscriber that lets its admins in — but keep the keys before read,
    /// and the owner's reseed onto its key, under the newer roster, takes it
    /// back. A subscriber that lets no stranger in takes keys given again
    /// for a channel it follows from the owner alone.
    #[test]
    fn keys_given_under_an_older_roster_do_not_hold_a_subscriber_back() {
        let mut c = channel();
        let admin = c.admin.clone();
        let owner = c.owner.clone();
        let with_admin = c.with_admin.clone();
        let without = c.without.clone();
        let root0 = key_tree::node_key(&SEED0, &c.reference, 0, ROOT, 0);
        let real = key_tree::node_key(&SEED1, &c.reference, 1, ROOT, 0);
        let far = 1_000_000;
        // The owner seals to the key Dave published before the fork.
        let docs = c.hard(0, &without, 1);
        assert!(matches!(
            c.give(&admin, &with_admin, &[5; 32], far),
            IntroOutcome::Subscribed(_)
        ));
        assert_eq!(c.root().0, far);
        // What is sealed under the keys before still opens.
        assert_eq!(c.learn_roster(&root0), PublicEntry::Roster);
        for doc in &docs {
            assert_eq!(c.take(&owner, doc), PublicEntry::Rekeyed);
        }
        assert_eq!(c.root(), (1, real));
        // Keys under the roster it left behind move it no more.
        assert!(matches!(
            c.give(&admin, &with_admin, &[6; 32], far + 1),
            IntroOutcome::Ignored
        ));
        assert_eq!(c.root(), (1, real));

        // Keys given again by the owner, of a lower generation: behind
        // under the roster the fork came under, taken under a newer one.
        let mut c = channel();
        assert!(matches!(
            c.give(&admin, &with_admin, &[5; 32], far),
            IntroOutcome::Subscribed(_)
        ));
        assert!(matches!(
            c.give(&owner, &with_admin, &SEED1, 1),
            IntroOutcome::Ignored
        ));
        assert_eq!(c.root().0, far);
        assert!(matches!(
            c.give(&owner, &without, &SEED1, 1),
            IntroOutcome::Subscribed(_)
        ));
        assert_eq!(c.root(), (1, real));

        let mut c = channel();
        c.dave
            .set_intro_policy(IntroPolicy {
                mode: IntroMode::Manual,
                daily_limit: 20,
                allowed: vec![],
            })
            .unwrap();
        let (admin, owner) = (c.admin.clone(), c.owner.clone());
        let with_admin = c.with_admin.clone();
        assert!(matches!(
            c.give(&admin, &with_admin, &[5; 32], 3),
            IntroOutcome::Pending(_)
        ));
        assert_eq!(c.root().0, 0);
        assert!(matches!(
            c.give(&owner, &with_admin, &[7; 32], 3),
            IntroOutcome::Subscribed(_)
        ));
        assert_eq!(
            c.root(),
            (3, key_tree::node_key(&[7; 32], &c.reference, 3, ROOT, 0))
        );
    }

    /// An admin's keys of an older generation, under a roster newer than
    /// the one Dave took his keys under, do not move him back; the owner's
    /// reseed onto his key counts whichever generation it starts from, once
    /// under a newer roster.
    #[test]
    fn an_admins_keys_of_an_older_generation_do_not_move_a_subscriber_back() {
        let mut c = channel();
        let (admin, owner) = (c.admin.clone(), c.owner.clone());
        let with_admin = c.with_admin.clone();
        assert!(matches!(
            c.give(&owner, &with_admin, &[8; 32], 1),
            IntroOutcome::Subscribed(_)
        ));
        let newer = roster(&owner, c.reference, 3, &[&admin]);
        assert!(matches!(
            c.give(&admin, &newer, &SEED0, 0),
            IntroOutcome::Ignored
        ));
        assert_eq!(c.root().0, 1);
        // From a generation ahead of Dave's, under a newer roster: taken.
        let without = roster(&owner, c.reference, 4, &[]);
        let mut docs = c.hard(0, &without, 1);
        for doc in &mut docs {
            doc.generation = 2;
        }
        let personal = c
            .dave
            .follow_keys(&c.conversation)
            .unwrap()
            .unwrap()
            .personal_public()
            .unwrap();
        let entries = key_tree::hard_reseed(
            &SEED1,
            &c.reference,
            3,
            &[LEAF],
            &BTreeMap::from([(LEAF, personal)]),
            &EPHEMERAL,
        )
        .unwrap();
        docs[0].entries = entries;
        assert_eq!(c.take(&owner, &docs[0]), PublicEntry::Rekeyed);
        assert_eq!(
            c.root(),
            (3, key_tree::node_key(&SEED1, &c.reference, 3, ROOT, 0))
        );
    }

    /// Half of the owner's reseed onto Dave's key taken, the roster it came
    /// under is the one Dave knows: a former admin's reseed under the older
    /// one moves him nowhere, and the other half completes it, in either
    /// order.
    #[test]
    fn half_the_owners_reseed_taken_a_former_admins_reseed_moves_nobody() {
        for first in [0, 1] {
            let mut c = channel();
            let (admin, owner) = (c.admin.clone(), c.owner.clone());
            let without = c.without.clone();
            let docs = c.hard(0, &without, 2);
            assert_eq!(
                c.take(&owner, &docs[first]),
                PublicEntry::Ignored,
                "{first}"
            );
            let fake = KeyUpdate {
                group: c.reference,
                generation: 0,
                version: 0,
                removed: None,
                entries: key_tree::reseed(&SEED0, &[66; 32], &c.reference, 0, &[], &[LEAF])
                    .unwrap(),
                roster: c.with_admin.clone(),
                personal: None,
            };
            assert_eq!(c.take(&admin, &fake), PublicEntry::Ignored);
            assert_eq!(c.root().0, 0);
            assert_eq!(
                c.take(&owner, &docs[1 - first]),
                PublicEntry::Rekeyed,
                "{first}"
            );
            let real = key_tree::node_key(&SEED1, &c.reference, 1, ROOT, 0);
            assert_eq!(c.root(), (1, real));
        }
    }

    /// Keys the owner gave before, sent again after a removal moved Dave
    /// on, do not turn him back: the roster of what he took since counts.
    #[test]
    fn the_owners_old_keys_sent_again_do_not_turn_a_subscriber_back() {
        let mut c = channel();
        let owner = c.owner.clone();
        let newer = roster(&owner, c.reference, 2, &[]);
        let removal = KeyUpdate {
            group: c.reference,
            generation: 0,
            version: 0,
            removed: Some(NEIGHBOUR),
            entries: key_tree::rekey(&SEED0, &c.reference, 0, &[NEIGHBOUR], NEIGHBOUR)
                .unwrap()
                .into_iter()
                .map(|sealed| (0, sealed))
                .collect(),
            roster: newer.clone(),
            personal: None,
        };
        let (old_keys, old_packet) = c.keys(&owner, &newer, &SEED0, 0);
        assert_eq!(c.take(&owner, &removal), PublicEntry::Rekeyed);
        let moved = c.root();
        assert!(matches!(
            c.dave.take_keys_packet(&old_keys, old_packet, NOW).unwrap(),
            IntroOutcome::Ignored
        ));
        assert_eq!(c.root(), moved);
    }

    /// The owner's reseed onto a subscriber's key in two documents comes
    /// together in either order, each taken once.
    #[test]
    fn the_owners_reseed_onto_a_subscribers_key_comes_together_in_either_order() {
        for first in [0, 1] {
            let mut c = channel();
            let owner = c.owner.clone();
            let without = c.without.clone();
            let docs = c.hard(0, &without, 2);
            assert_eq!(docs.len(), 2);
            let start = c.root();
            assert_eq!(
                c.take(&owner, &docs[first]),
                PublicEntry::Ignored,
                "{first}"
            );
            assert_eq!(c.root(), start);
            assert_eq!(
                c.take(&owner, &docs[1 - first]),
                PublicEntry::Rekeyed,
                "{first}"
            );
            let real = key_tree::node_key(&SEED1, &c.reference, 1, ROOT, 0);
            assert_eq!(c.root(), (1, real));
            for doc in &docs {
                assert_eq!(c.take(&owner, doc), PublicEntry::Ignored);
            }
            assert_eq!(c.root(), (1, real));
        }
    }
}
