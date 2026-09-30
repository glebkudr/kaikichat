//! Staged RFC 9420 operations. Persist `next_state` atomically before releasing wire bytes.
pub mod mailbox;

use agentic_protocol::MAX_BODY_BYTES;
use minicbor::{Decoder, Encoder};
use openmls::framing::errors::{MessageDecryptionError, SecretTreeError};
use openmls::prelude::{
    tls_codec::{Deserialize, Serialize},
    *,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};
mod storage;
pub use storage::{PersistenceStats, SecretState, record_scope};

const SUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
const MAX_STATE: usize = 32 * 1024 * 1024;
const MAX_CONTROL: usize = 1024 * 1024;
const MAX_WIRE: usize = 65_536;
/// A ratchet tree given apart from a Welcome: a group of 2000 has one of
/// about 0.5 MB.
const MAX_TREE: usize = 4 * 1024 * 1024;
/// Application data every member of a group agrees on: a private-use
/// GroupContext extension (RFC 9420 §17.3) that only commits change.
const GROUP_DATA: u16 = 0xF1A0;
fn group_data_type() -> ExtensionType {
    ExtensionType::Unknown(GROUP_DATA)
}
/// Leaf capabilities of this client: it carries group data.
fn capabilities(mut extensions: Vec<ExtensionType>) -> Capabilities {
    extensions.push(group_data_type());
    Capabilities::builder().extensions(extensions).build()
}
fn leaf_carries_data(leaf: &LeafNode) -> bool {
    leaf.capabilities()
        .extensions()
        .contains(&group_data_type())
}
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid or corrupted MLS snapshot")]
    InvalidState,
    #[error("MLS operation rejected")]
    Mls,
    #[error("MLS application awaits an earlier sender generation")]
    ReceiveGap,
    #[error("MLS input exceeds resource limits")]
    TooLarge,
    #[error("invalid MLS input")]
    InvalidInput,
    #[error("unknown MLS group")]
    UnknownGroup,
    #[error("group already exists in this profile")]
    AlreadyJoined,
    #[error("membership change awaits finalization")]
    PendingCommit,
    #[error("local device is no longer an active group member")]
    Inactive,
    #[error("message does not match expected group or application context")]
    ContextMismatch,
    #[error("unsupported MLS ciphersuite")]
    UnsupportedSuite,
    #[error("a member's or invitee's MLS client cannot carry this group's data")]
    Outdated,
    #[error("MLS operation outside the records this client loaded")]
    OutOfScope,
}
pub struct Prepared<T> {
    pub next_state: SecretState,
    pub value: T,
}
pub struct EncryptedMessage {
    pub wire: Vec<u8>,
    pub epoch: u64,
}
pub struct DecryptedMessage {
    pub plaintext: Vec<u8>,
    pub sender: Vec<u8>,
    pub epoch: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationMessageMetadata {
    pub epoch: u64,
    pub sender: Vec<u8>,
    pub generation: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationMessageOrder {
    pub input_index: usize,
    pub epoch: u64,
    pub sender: Vec<u8>,
    pub generation: u32,
}
pub struct AddedMembers {
    pub commit: Vec<u8>,
    pub welcome: Vec<u8>,
}
/// A commit this client made and holds pending.
pub struct CommitMade {
    pub commit: Vec<u8>,
    /// For the members it adds.
    pub welcome: Option<Vec<u8>>,
}
/// What a commit does, as its receiver sees it before or while applying it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitView {
    /// The committer's credential identity.
    pub committer: Vec<u8>,
    /// The commit's authenticated data.
    pub aad: Vec<u8>,
    pub removed: Vec<Vec<u8>>,
    pub added: Vec<Vec<u8>>,
    /// This client is among the removed.
    pub self_removed: bool,
    /// The group data after the commit.
    pub data: Option<Vec<u8>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberIdentity {
    pub identity: Vec<u8>,
    pub signature_key: Vec<u8>,
}
pub struct MlsClient {
    snapshot: SecretState,
    persisted_records: bool,
}

impl MlsClient {
    pub fn new(identity: &[u8]) -> Result<Self, CryptoError> {
        if identity.is_empty() || identity.len() > 256 {
            return Err(CryptoError::InvalidInput);
        }
        let provider = OpenMlsRustCrypto::default();
        let signer = SignatureKeyPair::new(SUITE.signature_algorithm()).map_err(mls_error)?;
        signer.store(provider.storage()).map_err(mls_error)?;
        let engine = Engine {
            provider,
            identity: identity.to_vec(),
            signer_public: signer.to_public_vec(),
        };
        Ok(Self {
            snapshot: engine.dump(None)?,
            persisted_records: false,
        })
    }
    pub fn restore(bytes: &[u8]) -> Result<Self, CryptoError> {
        // Validate a bounded snapshot and signer before allowing any future operation.
        let engine = Engine::restore(bytes)?;
        Ok(Self {
            snapshot: engine.dump(None)?,
            persisted_records: false,
        })
    }
    /// Move an already validated staged state into the next local operation.
    /// This does not acknowledge persistence or release its wire to a peer.
    pub fn from_state(snapshot: SecretState) -> Self {
        Self {
            snapshot,
            persisted_records: false,
        }
    }
    pub fn restore_records(
        prefix: &[u8],
        records: impl IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
    ) -> Result<Self, CryptoError> {
        let mut records = records.into_iter().peekable();
        if records.peek().is_none() {
            return Self::restore(prefix);
        }
        let snapshot = SecretState::restore_records(prefix, records)?;
        snapshot.engine()?.signer()?;
        Ok(Self {
            snapshot,
            persisted_records: true,
        })
    }
    /// Restore what one operation needs of a record-backed state: the
    /// profile's own records and, with `group`, that group's. It acts on
    /// that group alone. A legacy snapshot, with no records apart, restores
    /// whole.
    pub fn restore_scope(
        prefix: &[u8],
        records: impl IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
        group: Option<[u8; 32]>,
    ) -> Result<Self, CryptoError> {
        let mut records = records.into_iter().peekable();
        if records.peek().is_none() {
            return Self::restore(prefix);
        }
        let snapshot = SecretState::restore_scope(prefix, records, group)?;
        snapshot.engine()?.signer()?;
        Ok(Self {
            snapshot,
            persisted_records: true,
        })
    }
    /// Only a record-backed input can serve as the SQL delta base. The first
    /// commit from a legacy snapshot must populate every provider record.
    pub fn persisted_record_state(&self) -> Option<&SecretState> {
        self.persisted_records.then_some(&self.snapshot)
    }
    pub fn snapshot(&self) -> &SecretState {
        &self.snapshot
    }
    fn prepare<T>(
        &self,
        operation: impl FnOnce(&Engine) -> Result<T, CryptoError>,
    ) -> Result<Prepared<T>, CryptoError> {
        let engine = self.snapshot.engine()?;
        let value = operation(&engine)?;
        Ok(Prepared {
            next_state: engine.dump(Some(&self.snapshot))?,
            value,
        })
    }
    pub fn key_package(&self) -> Result<Prepared<Vec<u8>>, CryptoError> {
        self.prepare(|e| {
            let bundle = KeyPackage::builder()
                .leaf_node_capabilities(capabilities(vec![]))
                .build(SUITE, &e.provider, &e.signer()?, e.credential())
                .map_err(mls_error)?;
            bounded(
                bundle
                    .key_package()
                    .tls_serialize_detached()
                    .map_err(mls_error)?,
                MAX_WIRE,
            )
        })
    }
    /// A KeyPackage kept after it is used, so one published package serves
    /// every Welcome made with it.
    pub fn last_resort_key_package(&self) -> Result<Prepared<Vec<u8>>, CryptoError> {
        self.prepare(|e| {
            // A key package's extensions must be among its leaf's capabilities.
            let bundle = KeyPackage::builder()
                .leaf_node_capabilities(capabilities(vec![ExtensionType::LastResort]))
                .mark_as_last_resort()
                .build(SUITE, &e.provider, &e.signer()?, e.credential())
                .map_err(mls_error)?;
            bounded(
                bundle
                    .key_package()
                    .tls_serialize_detached()
                    .map_err(mls_error)?,
                MAX_WIRE,
            )
        })
    }
    pub fn create_group(&self, id: [u8; 32]) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.prepare(|e| {
            let id = GroupId::from_slice(&id);
            if MlsGroup::load(e.provider.storage(), &id)
                .map_err(mls_error)?
                .is_some()
            {
                return Err(CryptoError::AlreadyJoined);
            }
            let config = MlsGroupCreateConfig::builder()
                .ciphersuite(SUITE)
                .padding_size(256)
                .sender_ratchet_configuration(SenderRatchetConfiguration::new(128, 1000))
                .max_past_epochs(3)
                .use_ratchet_tree_extension(true)
                .wire_format_policy(PURE_CIPHERTEXT_WIRE_FORMAT_POLICY)
                .capabilities(capabilities(vec![]))
                .build();
            MlsGroup::new_with_group_id(&e.provider, &e.signer()?, &config, id, e.credential())
                .map_err(mls_error)?;
            Ok(())
        })
    }
    pub fn add_members(
        &self,
        id: [u8; 32],
        packages: &[Vec<u8>],
    ) -> Result<Prepared<AddedMembers>, CryptoError> {
        self.snapshot.check_scope(id)?;
        if packages.is_empty() {
            return Err(CryptoError::InvalidInput);
        }
        if packages.len() > 1024 || packages.iter().map(Vec::len).sum::<usize>() > MAX_CONTROL {
            return Err(CryptoError::TooLarge);
        }
        self.prepare(|e| {
            let mut group = e.group(id)?;
            let mut validated = Vec::with_capacity(packages.len());
            for bytes in packages {
                let package = validate_key_package(bytes, &e.provider)?;
                validated.push(package);
            }
            let (commit, welcome, _) = group
                .add_members(&e.provider, &e.signer()?, &validated)
                .map_err(mls_error)?;
            Ok(AddedMembers {
                commit: bounded(commit.to_bytes().map_err(mls_error)?, MAX_CONTROL)?,
                welcome: bounded(welcome.to_bytes().map_err(mls_error)?, MAX_CONTROL)?,
            })
        })
    }
    /// One commit adding `packages` and removing the members with the
    /// `removes` identities, carrying `aad`, and replacing the group data
    /// with `data` when given; held pending until activated or cleared.
    pub fn commit_changes(
        &self,
        id: [u8; 32],
        packages: &[Vec<u8>],
        removes: &[Vec<u8>],
        aad: &[u8],
        data: Option<&[u8]>,
    ) -> Result<Prepared<CommitMade>, CryptoError> {
        self.snapshot.check_scope(id)?;
        if packages.len() + removes.len() > 1024
            || packages.iter().map(Vec::len).sum::<usize>() > MAX_CONTROL
        {
            return Err(CryptoError::TooLarge);
        }
        check_size(aad, MAX_CONTROL)?;
        check_size(data.unwrap_or_default(), MAX_WIRE)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            if !group.is_active() {
                return Err(CryptoError::Inactive);
            }
            if group.pending_commit().is_some() {
                return Err(CryptoError::PendingCommit);
            }
            let mut validated = Vec::with_capacity(packages.len());
            for bytes in packages {
                validated.push(validate_key_package(bytes, &e.provider)?);
            }
            // Whoever joins a group with data carries it; the first data
            // needs every member to.
            let has_data = group.extensions().unknown(GROUP_DATA).is_some();
            let invitees_carry = validated.iter().all(|p| leaf_carries_data(p.leaf_node()));
            let members_carry = || {
                group.members().all(|m| {
                    group
                        .public_group()
                        .leaf(m.index)
                        .is_some_and(leaf_carries_data)
                })
            };
            if ((has_data || data.is_some()) && !invitees_carry)
                || (!has_data && data.is_some() && !members_carry())
            {
                return Err(CryptoError::Outdated);
            }
            let mut leaves = Vec::with_capacity(removes.len());
            for identity in removes {
                let matching: Vec<_> = group
                    .members()
                    .filter(|m| m.credential.serialized_content() == identity.as_slice())
                    .map(|m| m.index)
                    .collect();
                let [leaf] = matching.as_slice() else {
                    return Err(CryptoError::InvalidInput);
                };
                leaves.push(*leaf);
            }
            let extensions = data
                .map(|data| with_data(group.extensions(), data))
                .transpose()?;
            group.set_aad(aad.to_vec());
            let mut builder = group
                .commit_builder()
                .force_self_update(validated.is_empty() && leaves.is_empty())
                .propose_removals(leaves)
                .propose_adds(validated);
            if let Some(extensions) = extensions {
                builder = builder
                    .propose_group_context_extensions(extensions)
                    .map_err(mls_error)?;
            }
            let bundle = builder
                .load_psks(e.provider.storage())
                .map_err(mls_error)?
                .build(e.provider.rand(), e.provider.crypto(), &e.signer()?, |_| {
                    true
                })
                .map_err(mls_error)?
                .stage_commit(&e.provider)
                .map_err(mls_error)?;
            let (commit, welcome, _) = bundle.into_messages();
            Ok(CommitMade {
                commit: bounded(commit.to_bytes().map_err(mls_error)?, MAX_CONTROL)?,
                welcome: welcome
                    .map(|w| w.to_bytes().map_err(mls_error))
                    .transpose()?
                    .map(|w| bounded(w, MAX_CONTROL))
                    .transpose()?,
            })
        })
    }
    /// Drop this client's pending commit: another one took its epoch.
    pub fn clear_pending_commit(&self, id: [u8; 32]) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            group
                .clear_pending_commit(e.provider.storage())
                .map_err(mls_error)?;
            Ok(())
        })
    }
    /// What another member's commit does, without applying it.
    pub fn view_commit(&self, id: [u8; 32], wire: &[u8]) -> Result<CommitView, CryptoError> {
        self.snapshot.check_scope(id)?;
        Ok(self.process_commit(id, wire, false)?.value)
    }
    /// Apply another member's commit: the group moves to its next epoch.
    pub fn apply_commit(
        &self,
        id: [u8; 32],
        wire: &[u8],
    ) -> Result<Prepared<CommitView>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.process_commit(id, wire, true)
    }
    fn process_commit(
        &self,
        id: [u8; 32],
        wire: &[u8],
        merge: bool,
    ) -> Result<Prepared<CommitView>, CryptoError> {
        check_size(wire, MAX_CONTROL)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            if !group.is_active() {
                return Err(CryptoError::Inactive);
            }
            let protocol = protocol(wire, id)?;
            let processed = group
                .process_message(&e.provider, protocol)
                .map_err(mls_error)?;
            let identity = |index| {
                group
                    .member(index)
                    .map(|credential| credential.serialized_content().to_vec())
            };
            let committer = match processed.sender() {
                Sender::Member(index) => identity(*index).ok_or(CryptoError::InvalidInput)?,
                _ => return Err(CryptoError::InvalidInput),
            };
            let aad = processed.aad().to_vec();
            let ProcessedMessageContent::StagedCommitMessage(staged) = processed.into_content()
            else {
                return Err(CryptoError::InvalidInput);
            };
            let removed = staged
                .remove_proposals()
                .map(|p| identity(p.remove_proposal().removed()).ok_or(CryptoError::InvalidInput))
                .collect::<Result<Vec<_>, _>>()?;
            let added = staged
                .add_proposals()
                .map(|p| {
                    p.add_proposal()
                        .key_package()
                        .leaf_node()
                        .credential()
                        .serialized_content()
                        .to_vec()
                })
                .collect();
            let view = CommitView {
                committer,
                aad,
                removed,
                added,
                self_removed: staged.self_removed(),
                data: staged
                    .group_context()
                    .extensions()
                    .unknown(GROUP_DATA)
                    .map(|data| data.0.clone()),
            };
            if merge {
                group
                    .merge_staged_commit(&e.provider, *staged)
                    .map_err(mls_error)?;
            }
            Ok(view)
        })
    }
    /// The group data in force, if the group has any.
    pub fn group_data(&self, id: [u8; 32]) -> Result<Option<Vec<u8>>, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        Ok(e.group(id)?
            .extensions()
            .unknown(GROUP_DATA)
            .map(|data| data.0.clone()))
    }
    /// The group's current epoch.
    pub fn epoch(&self, id: [u8; 32]) -> Result<u64, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        Ok(e.group(id)?.epoch().as_u64())
    }
    /// Whether this client is still a member of the group.
    pub fn is_active(&self, id: [u8; 32]) -> Result<bool, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        Ok(e.group(id)?.is_active())
    }
    /// Whether this client keeps any state of the group.
    pub fn knows_group(&self, id: [u8; 32]) -> Result<bool, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        Ok(
            MlsGroup::load(e.provider.storage(), &GroupId::from_slice(&id))
                .map_err(mls_error)?
                .is_some(),
        )
    }
    /// Forget a group this client was removed from, so it can join again.
    pub fn forget_group(&self, id: [u8; 32]) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            group.delete(e.provider.storage()).map_err(mls_error)?;
            storage::drop_group_records(e, id)
        })
    }
    /// The secret of the group mailbox of the current epoch: one for every
    /// member, derived from the epoch's exporter.
    pub fn group_mailbox_secret(
        &self,
        id: [u8; 32],
        domain: [u8; 32],
    ) -> Result<mailbox::EpochMailbox, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        let group = e.group(id)?;
        if !group.is_active() {
            return Err(CryptoError::Inactive);
        }
        let mut context = Encoder::new(Vec::new());
        context
            .array(2)
            .map_err(mls_error)?
            .bytes(&domain)
            .map_err(mls_error)?
            .bytes(&id)
            .map_err(mls_error)?;
        let exported = Zeroizing::new(
            group
                .export_secret(
                    e.provider.crypto(),
                    "AIN_GROUP_MAILBOX",
                    &context.into_writer(),
                    32,
                )
                .map_err(mls_error)?,
        );
        Ok(mailbox::EpochMailbox {
            epoch: group.epoch().as_u64(),
            secret: mailbox::MailboxSecret::from_bytes(
                exported.as_slice().try_into().map_err(mls_error)?,
            ),
        })
    }
    pub fn remove_member(
        &self,
        id: [u8; 32],
        identity: &[u8],
    ) -> Result<Prepared<Vec<u8>>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            let matching: Vec<_> = group
                .members()
                .filter(|m| m.credential.serialized_content() == identity)
                .map(|m| m.index)
                .collect();
            if matching.len() != 1 {
                return Err(CryptoError::InvalidInput);
            }
            let (commit, _, _) = group
                .remove_members(&e.provider, &e.signer()?, &matching)
                .map_err(mls_error)?;
            bounded(commit.to_bytes().map_err(mls_error)?, MAX_CONTROL)
        })
    }
    /// Trusted finalizer/application boundary; never expose this as an unrestricted agent tool.
    pub fn activate_pending_commit(&self, id: [u8; 32]) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            if group.pending_commit().is_none() {
                return Err(CryptoError::InvalidInput);
            }
            group.merge_pending_commit(&e.provider).map_err(mls_error)?;
            Ok(())
        })
    }
    /// The caller must authorize and finalize the exact control wire before applying it.
    pub fn apply_finalized_commit(
        &self,
        id: [u8; 32],
        wire: &[u8],
    ) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        check_size(wire, MAX_CONTROL)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            let protocol = protocol(wire, id)?;
            let processed = group
                .process_message(&e.provider, protocol)
                .map_err(mls_error)?;
            match processed.into_content() {
                ProcessedMessageContent::StagedCommitMessage(commit) => group
                    .merge_staged_commit(&e.provider, *commit)
                    .map_err(mls_error),
                _ => Err(CryptoError::InvalidInput),
            }
        })
    }
    /// From now on this group's Welcomes carry no ratchet tree: newcomers
    /// join with the tree given apart (`ratchet_tree`, `join_with_tree`).
    pub fn set_tree_apart(&self, id: [u8; 32]) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            tree_along(group.configuration()).ok_or(CryptoError::InvalidState)?;
            group
                .set_configuration(e.provider.storage(), &join_config(1000, false))
                .map_err(mls_error)
        })
    }
    /// The group's current ratchet tree, for newcomers joining with it apart.
    pub fn ratchet_tree(&self, id: [u8; 32]) -> Result<Vec<u8>, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        let tree = e
            .group(id)?
            .export_ratchet_tree()
            .tls_serialize_detached()
            .map_err(mls_error)?;
        bounded(tree, MAX_TREE)
    }
    pub fn join(&self, id: [u8; 32], wire: &[u8]) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.join_welcome(id, wire, None)
    }
    /// Join from a Welcome that carries no ratchet tree, with the tree
    /// fetched apart; the group keeps its tree apart from then on.
    pub fn join_with_tree(
        &self,
        id: [u8; 32],
        wire: &[u8],
        tree: &[u8],
    ) -> Result<Prepared<()>, CryptoError> {
        self.snapshot.check_scope(id)?;
        check_size(tree, MAX_TREE)?;
        self.join_welcome(id, wire, Some(tree))
    }
    fn join_welcome(
        &self,
        id: [u8; 32],
        wire: &[u8],
        tree: Option<&[u8]>,
    ) -> Result<Prepared<()>, CryptoError> {
        check_size(wire, MAX_CONTROL)?;
        let tree = tree
            .map(|bytes| RatchetTreeIn::tls_deserialize_exact(bytes).map_err(mls_error))
            .transpose()?;
        self.prepare(|e| {
            if MlsGroup::load(e.provider.storage(), &GroupId::from_slice(&id))
                .map_err(mls_error)?
                .is_some()
            {
                return Err(CryptoError::AlreadyJoined);
            }
            let welcome = match MlsMessageIn::tls_deserialize_exact(wire)
                .map_err(mls_error)?
                .extract()
            {
                MlsMessageBodyIn::Welcome(w) => w,
                _ => return Err(CryptoError::InvalidInput),
            };
            if welcome.ciphersuite() != SUITE {
                return Err(CryptoError::UnsupportedSuite);
            }
            let config = join_config(1000, tree.is_none());
            let group = StagedWelcome::new_from_welcome(&e.provider, &config, welcome, tree)
                .map_err(mls_error)?
                .into_group(&e.provider)
                .map_err(mls_error)?;
            if group.group_id().as_slice() != id {
                return Err(CryptoError::ContextMismatch);
            }
            Ok(())
        })
    }
    pub fn encrypt(
        &self,
        id: [u8; 32],
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<Prepared<EncryptedMessage>, CryptoError> {
        self.snapshot.check_scope(id)?;
        check_size(plaintext, MAX_BODY_BYTES)?;
        check_size(aad, 1024)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            if !group.is_active() {
                return Err(CryptoError::Inactive);
            }
            if group.pending_commit().is_some() {
                return Err(CryptoError::PendingCommit);
            }
            group.set_aad(aad.to_vec());
            let epoch = group.epoch().as_u64();
            let wire = group
                .create_message(&e.provider, &e.signer()?, plaintext)
                .map_err(mls_error)?
                .to_bytes()
                .map_err(mls_error)?;
            Ok(EncryptedMessage {
                wire: bounded(wire, MAX_WIRE)?,
                epoch,
            })
        })
    }
    pub fn decrypt(
        &self,
        id: [u8; 32],
        wire: &[u8],
        expected_aad: &[u8],
    ) -> Result<Prepared<DecryptedMessage>, CryptoError> {
        self.snapshot.check_scope(id)?;
        self.decrypt_application(id, wire, expected_aad, false)
    }

    /// Authenticate sender data and expose its exact sender generation without
    /// consuming an application ratchet or changing the committed snapshot.
    pub fn inspect_application_message(
        &self,
        id: [u8; 32],
        wire: &[u8],
        expected_aad: &[u8],
    ) -> Result<ApplicationMessageMetadata, CryptoError> {
        self.snapshot.check_scope(id)?;
        check_size(wire, MAX_WIRE)?;
        check_size(expected_aad, 1024)?;
        let protocol = protocol(wire, id)?;
        validate_application_protocol(&protocol, expected_aad)?;
        let max_forward_distance = self.application_forward_distance(id)?;
        let mut low = 0;
        let mut high = max_forward_distance;
        if !matches!(
            self.process_application_at_distance(
                id,
                &protocol,
                expected_aad,
                max_forward_distance,
            )?,
            ApplicationProbe::Accepted(_)
        ) {
            return Err(CryptoError::Mls);
        }
        while low < high {
            let middle = low + (high - low) / 2;
            match self.process_application_at_distance(id, &protocol, expected_aad, middle)? {
                ApplicationProbe::Accepted(_) => high = middle,
                ApplicationProbe::Future => low = middle + 1,
            }
        }
        let ApplicationProbe::Accepted(processed) =
            self.process_application_at_distance(id, &protocol, expected_aad, low)?
        else {
            return Err(CryptoError::Mls);
        };
        let head_generation = self.application_sender_generation(id, processed.sender())?;
        let generation = head_generation
            .checked_add(low)
            .ok_or(CryptoError::InvalidState)?;
        Ok(application_metadata(processed, generation))
    }

    /// Process a bounded set against a strict local ratchet copy. Only bodies
    /// whose predecessors are present in this set are returned; committed MLS
    /// state remains untouched.
    pub fn inspect_application_messages(
        &self,
        id: [u8; 32],
        messages: &[(&[u8], &[u8])],
    ) -> Result<Vec<ApplicationMessageOrder>, CryptoError> {
        self.snapshot.check_scope(id)?;
        let protocols = parse_application_messages(id, messages)?;
        let accepted = self.process_application_messages(id, &protocols, messages)?;
        accepted
            .into_iter()
            .map(|input_index| {
                let metadata = self.inspect_application_message(
                    id,
                    messages[input_index].0,
                    messages[input_index].1,
                )?;
                Ok(ApplicationMessageOrder {
                    input_index,
                    epoch: metadata.epoch,
                    sender: metadata.sender,
                    generation: metadata.generation,
                })
            })
            .collect()
    }

    /// Return only the inputs that are currently processable by a strict local
    /// ratchet copy. This avoids an exact-generation probe for callers that
    /// already persist authenticated ordering metadata.
    pub fn ready_application_messages(
        &self,
        id: [u8; 32],
        messages: &[(&[u8], &[u8])],
    ) -> Result<Vec<usize>, CryptoError> {
        self.snapshot.check_scope(id)?;
        let protocols = parse_application_messages(id, messages)?;
        self.process_application_messages(id, &protocols, messages)
    }

    fn process_application_messages(
        &self,
        id: [u8; 32],
        protocols: &[ProtocolMessage],
        messages: &[(&[u8], &[u8])],
    ) -> Result<Vec<usize>, CryptoError> {
        if protocols.is_empty() {
            return Ok(Vec::new());
        }
        let e = self.snapshot.engine()?;
        let mut group = e.group(id)?;
        let along = tree_along(group.configuration()).ok_or(CryptoError::InvalidState)?;
        group
            .set_configuration(e.provider.storage(), &join_config(0, along))
            .map_err(mls_error)?;
        let mut pending: Vec<usize> = (0..protocols.len()).collect();
        let mut accepted = vec![false; protocols.len()];
        while !pending.is_empty() {
            let mut next = Vec::new();
            let mut progressed = false;
            for index in pending {
                match group.process_message(&e.provider, protocols[index].clone()) {
                    Ok(processed) => {
                        if processed.aad() != messages[index].1
                            || !matches!(
                                processed.content(),
                                ProcessedMessageContent::ApplicationMessage(_)
                            )
                        {
                            return Err(CryptoError::InvalidInput);
                        }
                        accepted[index] = true;
                        progressed = true;
                    }
                    Err(error) if is_future_gap(&error) => next.push(index),
                    Err(error) if is_replayed_message(&error) => {
                        accepted[index] = true;
                        progressed = true;
                    }
                    Err(_) => return Err(CryptoError::Mls),
                }
            }
            if !progressed {
                break;
            }
            pending = next;
        }
        Ok(accepted
            .into_iter()
            .enumerate()
            .filter_map(|(input_index, accepted)| accepted.then_some(input_index))
            .collect())
    }

    fn application_forward_distance(&self, id: [u8; 32]) -> Result<u32, CryptoError> {
        let e = self.snapshot.engine()?;
        let group = e.group(id)?;
        let max = group
            .configuration()
            .sender_ratchet_configuration()
            .maximum_forward_distance();
        if max > 1000 {
            return Err(CryptoError::InvalidState);
        }
        Ok(max)
    }

    fn application_sender_generation(
        &self,
        id: [u8; 32],
        sender: &Sender,
    ) -> Result<u32, CryptoError> {
        let Sender::Member(index) = sender else {
            return Err(CryptoError::InvalidInput);
        };
        let e = self.snapshot.engine()?;
        let mut key = b"MessageSecrets".to_vec();
        key.extend(
            serde_json::to_vec(&GroupId::from_slice(&id)).map_err(|_| CryptoError::InvalidState)?,
        );
        key.extend_from_slice(&u16::to_be_bytes(1));
        let values = e
            .provider
            .storage()
            .values
            .read()
            .map_err(|_| CryptoError::InvalidState)?;
        let bytes = values.get(&key).ok_or(CryptoError::InvalidState)?;
        let stored: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| CryptoError::InvalidState)?;
        let ratchets = stored
            .get("message_secrets")
            .and_then(|secrets| secrets.get("secret_tree"))
            .and_then(|tree| tree.get("application_sender_ratchets"))
            .and_then(serde_json::Value::as_array)
            .ok_or(CryptoError::InvalidState)?;
        let ratchet = ratchets
            .get(index.u32() as usize)
            .ok_or(CryptoError::InvalidState)?;
        if ratchet.is_null() {
            return Ok(0);
        }
        let head = ["DecryptionRatchet", "DualUse", "EncryptionRatchet"]
            .into_iter()
            .find_map(|name| ratchet.get(name))
            .and_then(|ratchet| ratchet.get("ratchet_head"))
            .and_then(|head| head.get("generation"))
            .and_then(serde_json::Value::as_u64)
            .ok_or(CryptoError::InvalidState)?;
        u32::try_from(head).map_err(|_| CryptoError::InvalidState)
    }

    fn process_application_at_distance(
        &self,
        id: [u8; 32],
        protocol: &ProtocolMessage,
        expected_aad: &[u8],
        maximum_forward_distance: u32,
    ) -> Result<ApplicationProbe, CryptoError> {
        let e = self.snapshot.engine()?;
        let mut group = e.group(id)?;
        if group
            .configuration()
            .sender_ratchet_configuration()
            .maximum_forward_distance()
            != maximum_forward_distance
        {
            let along = tree_along(group.configuration()).ok_or(CryptoError::InvalidState)?;
            group
                .set_configuration(
                    e.provider.storage(),
                    &join_config(maximum_forward_distance, along),
                )
                .map_err(mls_error)?;
        }
        match group.process_message(&e.provider, protocol.clone()) {
            Ok(processed) => {
                if processed.aad() != expected_aad
                    || !matches!(
                        processed.content(),
                        ProcessedMessageContent::ApplicationMessage(_)
                    )
                {
                    return Err(CryptoError::InvalidInput);
                }
                Ok(ApplicationProbe::Accepted(processed))
            }
            Err(error) if is_future_gap(&error) => Ok(ApplicationProbe::Future),
            Err(_) => Err(CryptoError::Mls),
        }
    }

    /// Prepare a receive without creating a new skipped sender generation.
    /// This does not protect older gaps already present in the input snapshot;
    /// the caller still owns durable catch-up admission and direct-path fencing.
    pub fn decrypt_contiguous(
        &self,
        id: [u8; 32],
        wire: &[u8],
        expected_aad: &[u8],
    ) -> Result<Prepared<DecryptedMessage>, CryptoError> {
        self.snapshot.check_scope(id)?;
        match self.decrypt_application(id, wire, expected_aad, true) {
            Err(CryptoError::ReceiveGap) => {
                // The strict ratchet rejects before authenticating application
                // content. Distinguish invalid input using a bounded candidate
                // from the original snapshot, then discard it without commit.
                let mut candidate = self.decrypt(id, wire, expected_aad)?;
                candidate.value.plaintext.zeroize();
                Err(CryptoError::ReceiveGap)
            }
            result => result,
        }
    }

    fn decrypt_application(
        &self,
        id: [u8; 32],
        wire: &[u8],
        expected_aad: &[u8],
        contiguous: bool,
    ) -> Result<Prepared<DecryptedMessage>, CryptoError> {
        check_size(wire, MAX_WIRE)?;
        check_size(expected_aad, 1024)?;
        self.prepare(|e| {
            let mut group = e.group(id)?;
            let original_config = group.configuration().clone();
            if contiguous {
                // Do not rebuild an unknown snapshot policy with defaults:
                // changing retention here can immediately remove old secrets.
                let along = tree_along(&original_config).ok_or(CryptoError::InvalidState)?;
                group
                    .set_configuration(e.provider.storage(), &join_config(0, along))
                    .map_err(mls_error)?;
            }
            let protocol = protocol(wire, id)?;
            if protocol.wire_format() != WireFormat::PrivateMessage {
                return Err(CryptoError::InvalidInput);
            }
            let epoch = protocol.epoch().as_u64();
            let processed = group
                .process_message(&e.provider, protocol)
                .map_err(|error| match error {
                    ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
                        MessageDecryptionError::SecretTreeError(
                            SecretTreeError::TooDistantInTheFuture,
                        ),
                    )) if contiguous => CryptoError::ReceiveGap,
                    _ => CryptoError::Mls,
                })?;
            if processed.aad() != expected_aad {
                return Err(CryptoError::ContextMismatch);
            }
            let sender = processed.credential().serialized_content().to_vec();
            let plaintext = match processed.into_content() {
                ProcessedMessageContent::ApplicationMessage(m) => m.into_bytes(),
                _ => return Err(CryptoError::InvalidInput),
            };
            if contiguous {
                group
                    .set_configuration(e.provider.storage(), &original_config)
                    .map_err(mls_error)?;
            }
            Ok(DecryptedMessage {
                plaintext: bounded(plaintext, MAX_BODY_BYTES)?,
                sender,
                epoch,
            })
        })
    }
    pub fn members(&self, id: [u8; 32]) -> Result<Vec<MemberIdentity>, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        Ok(e.group(id)?
            .members()
            .map(|m| MemberIdentity {
                identity: m.credential.serialized_content().to_vec(),
                signature_key: m.signature_key,
            })
            .collect())
    }

    /// Export a private, direction-specific rendezvous capability from the committed epoch.
    /// This read does not advance application ratchets or change the stored MLS snapshot.
    pub fn mailbox_secret(
        &self,
        id: [u8; 32],
        domain: [u8; 32],
        recipient: &[u8],
    ) -> Result<mailbox::EpochMailbox, CryptoError> {
        self.snapshot.check_scope(id)?;
        let e = self.snapshot.engine()?;
        let group = e.group(id)?;
        if !group.is_active() {
            return Err(CryptoError::Inactive);
        }
        if group.pending_commit().is_some() {
            return Err(CryptoError::PendingCommit);
        }
        if recipient.is_empty()
            || recipient.len() > 256
            || !group
                .members()
                .any(|m| m.credential.serialized_content() == recipient)
        {
            return Err(CryptoError::InvalidInput);
        }
        let mut context = Encoder::new(Vec::new());
        context
            .array(3)
            .map_err(mls_error)?
            .bytes(&domain)
            .map_err(mls_error)?
            .bytes(&id)
            .map_err(mls_error)?
            .bytes(recipient)
            .map_err(mls_error)?;
        let exported = Zeroizing::new(
            group
                .export_secret(
                    e.provider.crypto(),
                    "AgenticInternet/mailbox-export/v1",
                    &context.into_writer(),
                    32,
                )
                .map_err(mls_error)?,
        );
        Ok(mailbox::EpochMailbox {
            epoch: group.epoch().as_u64(),
            secret: mailbox::MailboxSecret::from_bytes(
                exported.as_slice().try_into().map_err(mls_error)?,
            ),
        })
    }
}

enum ApplicationProbe {
    Accepted(ProcessedMessage),
    Future,
}

fn parse_application_messages(
    id: [u8; 32],
    messages: &[(&[u8], &[u8])],
) -> Result<Vec<ProtocolMessage>, CryptoError> {
    if messages.len() > 256 {
        return Err(CryptoError::TooLarge);
    }
    messages
        .iter()
        .map(|(wire, aad)| {
            check_size(wire, MAX_WIRE)?;
            check_size(aad, 1024)?;
            let protocol = protocol(wire, id)?;
            validate_application_protocol(&protocol, aad)?;
            Ok(protocol)
        })
        .collect()
}

fn validate_application_protocol(
    protocol: &ProtocolMessage,
    expected_aad: &[u8],
) -> Result<(), CryptoError> {
    let ProtocolMessage::PrivateMessage(ciphertext) = protocol else {
        return Err(CryptoError::InvalidInput);
    };
    if ciphertext.content_type() != ContentType::Application {
        return Err(CryptoError::InvalidInput);
    }
    if ciphertext.aad() != expected_aad {
        return Err(CryptoError::ContextMismatch);
    }
    Ok(())
}

fn application_metadata(
    processed: ProcessedMessage,
    generation: u32,
) -> ApplicationMessageMetadata {
    ApplicationMessageMetadata {
        epoch: processed.epoch().as_u64(),
        sender: processed.credential().serialized_content().to_vec(),
        generation,
    }
}

fn is_future_gap<StorageError>(error: &ProcessMessageError<StorageError>) -> bool {
    matches!(
        error,
        ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
            MessageDecryptionError::SecretTreeError(SecretTreeError::TooDistantInTheFuture)
        ))
    )
}

fn is_replayed_message<StorageError>(error: &ProcessMessageError<StorageError>) -> bool {
    matches!(
        error,
        ProcessMessageError::ValidationError(ValidationError::UnableToDecrypt(
            MessageDecryptionError::SecretTreeError(SecretTreeError::SecretReuseError)
        ))
    )
}

/// `tree_along`: the group's Welcomes carry its ratchet tree; else the tree
/// goes apart.
fn join_config(maximum_forward_distance: u32, tree_along: bool) -> MlsGroupJoinConfig {
    MlsGroupJoinConfig::builder()
        .padding_size(256)
        .sender_ratchet_configuration(SenderRatchetConfiguration::new(
            128,
            maximum_forward_distance,
        ))
        .max_past_epochs(3)
        .use_ratchet_tree_extension(tree_along)
        .wire_format_policy(PURE_CIPHERTEXT_WIRE_FORMAT_POLICY)
        .build()
}

/// Whether a group made or joined here sends its tree along in Welcomes;
/// `None` for a policy this adapter never sets, which is not rebuilt.
fn tree_along(config: &MlsGroupJoinConfig) -> Option<bool> {
    [true, false]
        .into_iter()
        .find(|along| *config == join_config(1000, *along))
}

/// The group context extensions with `data` as the group data, required of
/// every member from then on.
fn with_data(
    current: &Extensions<GroupContext>,
    data: &[u8],
) -> Result<Extensions<GroupContext>, CryptoError> {
    let required = current.required_capabilities();
    let mut types = required.map_or(vec![], |r| r.extension_types().to_vec());
    if !types.contains(&group_data_type()) {
        types.push(group_data_type());
    }
    let required = RequiredCapabilitiesExtension::new(
        &types,
        required.map_or(&[], |r| r.proposal_types()),
        required.map_or(&[], |r| r.credential_types()),
    );
    let mut extensions: Vec<Extension> = current
        .iter()
        .filter(|e| {
            !matches!(e.extension_type(), ExtensionType::RequiredCapabilities)
                && e.extension_type() != group_data_type()
        })
        .cloned()
        .collect();
    extensions.push(Extension::RequiredCapabilities(required));
    extensions.push(Extension::Unknown(
        GROUP_DATA,
        UnknownExtension(data.to_vec()),
    ));
    Extensions::from_vec(extensions).map_err(mls_error)
}
fn mls_error<E>(_: E) -> CryptoError {
    CryptoError::Mls
}
fn state_error<E>(_: E) -> CryptoError {
    CryptoError::InvalidState
}
fn check_size(bytes: &[u8], max: usize) -> Result<(), CryptoError> {
    if bytes.len() > max {
        Err(CryptoError::TooLarge)
    } else {
        Ok(())
    }
}
fn bounded(bytes: Vec<u8>, max: usize) -> Result<Vec<u8>, CryptoError> {
    check_size(&bytes, max)?;
    Ok(bytes)
}
fn protocol(wire: &[u8], id: [u8; 32]) -> Result<ProtocolMessage, CryptoError> {
    let protocol = MlsMessageIn::tls_deserialize_exact(wire)
        .map_err(mls_error)?
        .try_into_protocol_message()
        .map_err(mls_error)?;
    if protocol.group_id().as_slice() != id {
        return Err(CryptoError::ContextMismatch);
    }
    Ok(protocol)
}

struct Engine {
    provider: OpenMlsRustCrypto,
    identity: Vec<u8>,
    signer_public: Vec<u8>,
}
impl Engine {
    fn signer(&self) -> Result<SignatureKeyPair, CryptoError> {
        SignatureKeyPair::read(
            self.provider.storage(),
            &self.signer_public,
            SUITE.signature_algorithm(),
        )
        .ok_or(CryptoError::InvalidState)
    }
    fn credential(&self) -> CredentialWithKey {
        CredentialWithKey {
            credential: BasicCredential::new(self.identity.clone()).into(),
            signature_key: self.signer_public.clone().into(),
        }
    }
    fn group(&self, id: [u8; 32]) -> Result<MlsGroup, CryptoError> {
        let group = MlsGroup::load(self.provider.storage(), &GroupId::from_slice(&id))
            .map_err(mls_error)?
            .ok_or(CryptoError::UnknownGroup)?;
        if group.ciphersuite() != SUITE {
            return Err(CryptoError::UnsupportedSuite);
        }
        Ok(group)
    }
    fn dump(self, previous: Option<&SecretState>) -> Result<SecretState, CryptoError> {
        SecretState::from_engine(self, previous)
    }
    fn restore(bytes: &[u8]) -> Result<Self, CryptoError> {
        check_size(bytes, MAX_STATE)?;
        let mut decoder = Decoder::new(bytes);
        if decoder.array().map_err(state_error)? != Some(4)
            || decoder.u8().map_err(state_error)? != 1
        {
            return Err(CryptoError::InvalidState);
        }
        let identity = decoder.bytes().map_err(state_error)?;
        let public = decoder.bytes().map_err(state_error)?;
        if identity.is_empty() || identity.len() > 256 || public.len() != 32 {
            return Err(CryptoError::InvalidState);
        }
        let e = Self {
            provider: OpenMlsRustCrypto::default(),
            identity: identity.to_vec(),
            signer_public: public.to_vec(),
        };
        let count = decoder
            .array()
            .map_err(state_error)?
            .ok_or(CryptoError::InvalidState)?;
        if count > 100_000 {
            return Err(CryptoError::TooLarge);
        }
        {
            let mut values = e.provider.storage().values.write().map_err(state_error)?;
            for _ in 0..count {
                if decoder.array().map_err(state_error)? != Some(2) {
                    return Err(CryptoError::InvalidState);
                }
                let key = decoder.bytes().map_err(state_error)?;
                check_size(key, 4096)?;
                let value = decoder.bytes().map_err(state_error)?;
                if values.contains_key(key) {
                    return Err(CryptoError::InvalidState);
                }
                values.insert(key.to_vec(), value.to_vec());
            }
        }
        if decoder.position() != bytes.len() {
            return Err(CryptoError::InvalidState);
        }
        e.signer()?;
        Ok(e)
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        // Access is local and single-threaded; recover a poisoned guard only to erase secrets.
        let mut values = match self.provider.storage().values.write() {
            Ok(v) => v,
            Err(e) => e.into_inner(),
        };
        for (mut key, mut value) in values.drain() {
            key.zeroize();
            value.zeroize();
        }
    }
}

fn validate_key_package(
    bytes: &[u8],
    provider: &OpenMlsRustCrypto,
) -> Result<KeyPackage, CryptoError> {
    check_size(bytes, MAX_WIRE)?;
    let package = KeyPackageIn::tls_deserialize_exact(bytes)
        .map_err(mls_error)?
        .validate(provider.crypto(), ProtocolVersion::Mls10)
        .map_err(mls_error)?;
    if package.ciphersuite() != SUITE {
        return Err(CryptoError::UnsupportedSuite);
    }
    Ok(package)
}
/// Validate before root-credential binding; the outer application signature must attest these bytes.
/// The Welcome of one newcomer of a batch: the batch's Welcome with only the
/// entry of `key_package`, whose owner alone can take it. A Welcome is not
/// signed as a whole, so the entry and the same encrypted GroupInfo remain a
/// valid one, however many the batch added.
pub fn welcome_for(welcome: &[u8], key_package: &[u8]) -> Result<Vec<u8>, CryptoError> {
    check_size(welcome, MAX_CONTROL)?;
    let provider = OpenMlsRustCrypto::default();
    let wanted = validate_key_package(key_package, &provider)?
        .hash_ref(provider.crypto())
        .map_err(mls_error)?;
    let MlsMessageBodyIn::Welcome(parsed) = MlsMessageIn::tls_deserialize_exact(welcome)
        .map_err(mls_error)?
        .extract()
    else {
        return Err(CryptoError::InvalidInput);
    };
    let entry = parsed
        .secrets()
        .iter()
        .find(|secrets| secrets.new_member() == wanted)
        .ok_or(CryptoError::InvalidInput)?
        .tls_serialize_detached()
        .map_err(mls_error)?;
    // Protocol version, wire format and cipher suite; then the entries and
    // the encrypted GroupInfo.
    let (head, mut rest) = welcome
        .split_at_checked(6)
        .ok_or(CryptoError::InvalidInput)?;
    VLBytes::tls_deserialize(&mut rest).map_err(mls_error)?;
    let info = VLBytes::tls_deserialize(&mut rest).map_err(mls_error)?;
    if !rest.is_empty() {
        return Err(CryptoError::InvalidInput);
    }
    let mut out = head.to_vec();
    VLBytes::new(entry)
        .tls_serialize(&mut out)
        .map_err(mls_error)?;
    info.tls_serialize(&mut out).map_err(mls_error)?;
    Ok(out)
}

/// The epoch a group message was written in, read from its clear header:
/// no key is needed, nothing is authenticated.
pub fn message_epoch(wire: &[u8]) -> Result<u64, CryptoError> {
    check_size(wire, MAX_WIRE)?;
    let protocol = MlsMessageIn::tls_deserialize_exact(wire)
        .map_err(mls_error)?
        .try_into_protocol_message()
        .map_err(mls_error)?;
    Ok(protocol.epoch().as_u64())
}
pub fn inspect_key_package(bytes: &[u8]) -> Result<MemberIdentity, CryptoError> {
    let package = validate_key_package(bytes, &OpenMlsRustCrypto::default())?;
    Ok(MemberIdentity {
        identity: package
            .leaf_node()
            .credential()
            .serialized_content()
            .to_vec(),
        signature_key: package.leaf_node().signature_key().as_slice().to_vec(),
    })
}
