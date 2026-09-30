//! Encrypted single-profile persistence shared by desktop, daemon and agent services.
use agentic_protocol::{DocumentDraft, MAX_BODY_BYTES, SignedDocument, WireError};
use ed25519_dalek::SigningKey;
use fs2::FileExt;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    path::Path,
};
use thiserror::Error;
use zeroize::{Zeroize, Zeroizing};
mod records;
pub use records::{StateRecordBatch, StateRecordChange, StateRecords};
#[cfg(feature = "test-probes")]
#[doc(hidden)]
pub mod test_probes;

const MESSAGE_COLUMNS: &str = "sequence,id,conversation_id,author,content,created_at,own";
// Core events are JSON; other store clients may persist opaque bytes. Guard the
// projection so opening an existing encrypted profile also supports those rows.
const EVENT_KIND: &str = "CASE WHEN json_valid(CAST(content AS TEXT)) THEN json_extract(CAST(content AS TEXT),'$.type') END";
const PROFILE_SCHEMA_VERSION: i64 = 2;
const PROFILE_SUPPORTED_READ: i64 = 1;
const PROFILE_SUPPORTED_WRITE: i64 = 2;
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("profile is already open by another writer")]
    ProfileInUse,
    #[error("operation or message ID was already used for different content")]
    IdempotencyConflict,
    #[error("stored state changed; reload before preparing another operation")]
    StateConflict,
    #[error("invalid or oversized storage input")]
    InvalidInput,
    #[error("SQLCipher is unavailable")]
    EncryptionUnavailable,
    #[error("invalid profile identity or schema")]
    InvalidProfile,
    #[error("operating system randomness unavailable")]
    Randomness,
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("profile filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Wire(#[from] WireError),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalIdentity {
    pub network_id: String,
    pub public_key: [u8; 32],
}
#[derive(Clone)]
pub struct StateChange {
    pub namespace: String,
    pub expected_revision: u64,
    pub bytes: Vec<u8>,
}
pub struct StateValue {
    pub revision: u64,
    pub bytes: Vec<u8>,
}
// A fully populated custody index links its carrier proofs plus every retained
// location's presentation, origin and pooled evidence; the bound stays finite
// while covering that worst case with headroom.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProfileSchema {
    schema_version: i64,
    supported_read: i64,
    supported_write: i64,
    migration_step: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub conversation_id: String,
    pub author: String,
    pub content: Vec<u8>,
    pub created_at: u64,
    pub own: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredMessage {
    pub sequence: u64,
    pub record: MessageRecord,
}
pub struct OutgoingCommit {
    pub operation_id: String,
    pub request_hash: [u8; 32],
    pub message: MessageRecord,
    pub destination: String,
    pub wire: Vec<u8>,
    pub states: Vec<StateChange>,
}
#[derive(Clone)]
pub struct IncomingCommit {
    pub message: MessageRecord,
    pub states: Vec<StateChange>,
}
pub struct OutboxRecord {
    pub message: StoredMessage,
    pub destination: String,
    pub wire: Vec<u8>,
}
// No Debug: neither the connection nor key material belong in logs.
pub struct ProfileStore {
    connection: Connection,
    _lock: File,
}

const PROFILE_TABLES: &str =
    "CREATE TABLE IF NOT EXISTS profile (singleton INTEGER PRIMARY KEY CHECK(singleton=1), owner_seed BLOB NOT NULL CHECK(length(owner_seed)=32));
     CREATE TABLE IF NOT EXISTS states (namespace TEXT PRIMARY KEY, revision INTEGER NOT NULL CHECK(revision>0), bytes BLOB NOT NULL);
     CREATE TABLE IF NOT EXISTS messages (sequence INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT NOT NULL UNIQUE, conversation_id TEXT NOT NULL, author TEXT NOT NULL, content BLOB NOT NULL, created_at INTEGER NOT NULL CHECK(created_at>=0), own INTEGER NOT NULL CHECK(own IN (0,1)));
     CREATE INDEX IF NOT EXISTS messages_by_conversation ON messages(conversation_id,sequence);
     CREATE TABLE IF NOT EXISTS operations (operation_id TEXT PRIMARY KEY, request_hash BLOB NOT NULL CHECK(length(request_hash)=32), message_id TEXT NOT NULL REFERENCES messages(id));
     CREATE TABLE IF NOT EXISTS outbox (message_id TEXT PRIMARY KEY REFERENCES messages(id), destination TEXT NOT NULL, wire BLOB NOT NULL);";
const PROFILE_SCHEMA_TABLE: &str = "CREATE TABLE profile_schema (
         singleton INTEGER PRIMARY KEY CHECK(singleton=1),
         schema_version INTEGER NOT NULL CHECK(schema_version BETWEEN 1 AND 2),
         supported_read INTEGER NOT NULL CHECK(supported_read BETWEEN 1 AND 2),
         supported_write INTEGER NOT NULL CHECK(supported_write BETWEEN 1 AND 2),
         migration_step INTEGER NOT NULL CHECK(migration_step BETWEEN 1 AND 2)
     );";

fn migration_backup_path(path: &Path, version: i64) -> Result<std::path::PathBuf, StoreError> {
    let name = path
        .file_name()
        .ok_or(StoreError::InvalidInput)?
        .to_string_lossy();
    Ok(path.with_file_name(format!("{name}.migration-v{version}.bak")))
}

fn create_migration_backup(
    connection: &Connection,
    path: &Path,
    version: i64,
) -> Result<(), StoreError> {
    let backup = migration_backup_path(path, version)?;
    if backup.exists() {
        if !backup.is_file() {
            return Err(StoreError::InvalidProfile);
        }
        return Ok(());
    }
    // Ensure all authenticated WAL frames are in the main encrypted file before
    // copying it. The backup is never opened with a new key or re-encrypted.
    connection.execute_batch("PRAGMA wal_checkpoint(FULL);")?;
    fs::copy(path, &backup)?;
    Ok(())
}

fn read_profile_schema(connection: &Connection) -> Result<ProfileSchema, StoreError> {
    connection
        .query_row(
            "SELECT schema_version,supported_read,supported_write,migration_step FROM profile_schema WHERE singleton=1",
            [],
            |row| {
                Ok(ProfileSchema {
                    schema_version: row.get(0)?,
                    supported_read: row.get(1)?,
                    supported_write: row.get(2)?,
                    migration_step: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or(StoreError::InvalidProfile)
}

fn validate_current_profile_schema(connection: &Connection) -> Result<(), StoreError> {
    let schema = read_profile_schema(connection)?;
    if schema
        != (ProfileSchema {
            schema_version: PROFILE_SCHEMA_VERSION,
            supported_read: PROFILE_SUPPORTED_READ,
            supported_write: PROFILE_SUPPORTED_WRITE,
            migration_step: PROFILE_SCHEMA_VERSION,
        })
    {
        return Err(StoreError::InvalidProfile);
    }
    Ok(())
}

fn migrate_profile_schema(connection: &Connection) -> Result<(), StoreError> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='profile_schema')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        connection.execute_batch(PROFILE_SCHEMA_TABLE)?;
        connection.execute(
            "INSERT INTO profile_schema(singleton,schema_version,supported_read,supported_write,migration_step) VALUES(1,1,1,1,1)",
            [],
        )?;
    } else if read_profile_schema(connection)?
        != (ProfileSchema {
            schema_version: 1,
            supported_read: 1,
            supported_write: 1,
            migration_step: 1,
        })
    {
        return Err(StoreError::InvalidProfile);
    }
    let changed = connection.execute(
        "UPDATE profile_schema SET schema_version=?1,supported_read=?2,supported_write=?3,migration_step=?4 WHERE singleton=1",
        params![
            PROFILE_SCHEMA_VERSION,
            PROFILE_SUPPORTED_READ,
            PROFILE_SUPPORTED_WRITE,
            PROFILE_SCHEMA_VERSION
        ],
    )?;
    if changed != 1 {
        return Err(StoreError::InvalidProfile);
    }
    Ok(())
}

impl ProfileStore {
    pub fn open(path: impl AsRef<Path>, master_key: &[u8; 32]) -> Result<Self, StoreError> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        // Relative directory aliases must share the same lifetime OS lock.
        let path = parent
            .canonicalize()?
            .join(path.file_name().ok_or(StoreError::InvalidInput)?);
        let mut lock_path = path.as_os_str().to_owned();
        lock_path.push(".lock");
        let lock = private_file(Path::new(&lock_path))?;
        FileExt::try_lock_exclusive(&lock).map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                StoreError::ProfileInUse
            } else {
                StoreError::Io(e)
            }
        })?;
        private_file(&path)?;
        let mut connection = Connection::open(&path)?;
        let key_hex = Zeroizing::new(hex::encode(master_key));
        let key_pragma = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", key_hex.as_str()));
        connection.execute_batch(&key_pragma)?;
        let cipher_version: String = connection
            .query_row("PRAGMA cipher_version", [], |r| r.get(0))
            .map_err(|_| StoreError::EncryptionUnavailable)?;
        if cipher_version.is_empty() {
            return Err(StoreError::EncryptionUnavailable);
        }
        // Authenticate existing pages before schema or journal writes; a wrong key never resets data.
        connection.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if !(0..=PROFILE_SCHEMA_VERSION).contains(&version) {
            return Err(StoreError::InvalidProfile);
        }
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; PRAGMA temp_store=MEMORY;")?;
        if version > 0 && version < PROFILE_SCHEMA_VERSION {
            create_migration_backup(&connection, &path, version)?;
        }
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        match version {
            0 => {
                tx.execute_batch(PROFILE_TABLES)?;
                tx.execute_batch(PROFILE_SCHEMA_TABLE)?;
                tx.execute(
                    "INSERT INTO profile_schema(singleton,schema_version,supported_read,supported_write,migration_step) VALUES(1,?1,?2,?3,?4)",
                    params![
                        PROFILE_SCHEMA_VERSION,
                        PROFILE_SUPPORTED_READ,
                        PROFILE_SUPPORTED_WRITE,
                        PROFILE_SCHEMA_VERSION
                    ],
                )?;
            }
            1 => {
                tx.execute_batch(PROFILE_TABLES)?;
                migrate_profile_schema(&tx)?;
            }
            PROFILE_SCHEMA_VERSION => {
                validate_current_profile_schema(&tx)?;
            }
            _ => unreachable!(),
        }
        if version < PROFILE_SCHEMA_VERSION {
            tx.execute_batch("PRAGMA user_version=2;")?;
        }
        tx.execute_batch(records::TABLES)?;
        tx.execute_batch(&format!("CREATE INDEX IF NOT EXISTS messages_by_event ON messages(conversation_id,({EVENT_KIND}),own,sequence); CREATE INDEX IF NOT EXISTS messages_by_event_sequence ON messages(conversation_id,({EVENT_KIND}),sequence);"))?;
        let has_identity: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM profile WHERE singleton=1)",
            [],
            |r| r.get(0),
        )?;
        if !has_identity {
            let mut seed = Zeroizing::new([0u8; 32]);
            getrandom::fill(&mut *seed).map_err(|_| StoreError::Randomness)?;
            tx.execute(
                "INSERT INTO profile(singleton,owner_seed) VALUES(1,?1)",
                [seed.as_slice()],
            )?;
        }
        tx.commit()?;
        Ok(Self {
            connection,
            _lock: lock,
        })
    }
    fn signing_key(&self) -> Result<SigningKey, StoreError> {
        let seed = Zeroizing::new(self.connection.query_row(
            "SELECT owner_seed FROM profile WHERE singleton=1",
            [],
            |r| r.get::<_, Vec<u8>>(0),
        )?);
        let bytes: &[u8; 32] = seed
            .as_slice()
            .try_into()
            .map_err(|_| StoreError::InvalidProfile)?;
        Ok(SigningKey::from_bytes(bytes))
    }
    pub fn identity(&self) -> Result<LocalIdentity, StoreError> {
        let public_key = self.signing_key()?.verifying_key().to_bytes();
        Ok(LocalIdentity {
            network_id: agentic_protocol::network_id(&public_key),
            public_key,
        })
    }
    pub fn sign_document(&self, draft: DocumentDraft) -> Result<SignedDocument, StoreError> {
        Ok(SignedDocument::sign(draft, &self.signing_key()?)?)
    }
    /// A document too big for one envelope; it travels in parts.
    pub fn sign_large_document(&self, draft: DocumentDraft) -> Result<SignedDocument, StoreError> {
        Ok(SignedDocument::sign_large(draft, &self.signing_key()?)?)
    }
    /// The parts of a whole wire, signed with the profile's key: the same
    /// whole and time give the same parts.
    pub fn split_document(
        &self,
        whole: &[u8],
        domain: [u8; 32],
        issued_at: u64,
    ) -> Result<Vec<SignedDocument>, StoreError> {
        Ok(agentic_protocol::parts::split_wire(
            whole,
            domain,
            &self.signing_key()?,
            issued_at,
        )?)
    }
    pub fn state(&self, namespace: &str) -> Result<Option<StateValue>, StoreError> {
        #[cfg(feature = "test-probes")]
        test_probes::state_read(namespace);
        Ok(self.state_records(namespace)?.map(|mut saved| {
            for (_, bytes) in saved.records {
                saved.state.bytes.extend_from_slice(&bytes);
            }
            saved.state
        }))
    }
    /// Read at most `limit` names in the binary-ordered range `(after, through]`.
    /// Resume with the last returned name; an absent cursor row is valid. This
    /// is a live range, not a snapshot: callers retain their scope and upper
    /// bound, and must account for any insertion behind an advancing cursor.
    /// The primary-key index supplies names without loading state payloads.
    pub fn state_namespaces_between(
        &self,
        after: &str,
        through: &str,
        limit: usize,
    ) -> Result<Vec<String>, StoreError> {
        identifier(after)?;
        identifier(through)?;
        if after > through
            || after.chars().chain(through.chars()).any(char::is_control)
            || !(1..=64).contains(&limit)
        {
            return Err(StoreError::InvalidInput);
        }
        let mut query = self.connection.prepare(
            "SELECT namespace FROM states WHERE namespace>?1 COLLATE BINARY AND namespace<=?2 COLLATE BINARY ORDER BY namespace LIMIT ?3",
        )?;
        Ok(query
            .query_map(params![after, through, limit as i64], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn commit_outgoing(&mut self, commit: OutgoingCommit) -> Result<StoredMessage, StoreError> {
        self.commit_outgoing_with_retry_states(commit, vec![])
    }
    /// A matching retry may consume fresh authorization state, but never staged crypto state.
    /// Both branches apply their selected changes in the same transaction as the operation result.
    pub fn commit_outgoing_with_retry_states(
        &mut self,
        commit: OutgoingCommit,
        retry_states: Vec<StateChange>,
    ) -> Result<StoredMessage, StoreError> {
        self.commit_outgoing_with_retry_states_and_records(commit, retry_states, vec![])
    }
    /// Record deltas share the message/outbox transaction and are discarded on
    /// an idempotent retry, just like the staged parent state.
    pub fn commit_outgoing_with_retry_states_and_records(
        &mut self,
        commit: OutgoingCommit,
        retry_states: Vec<StateChange>,
        records: Vec<StateRecordBatch>,
    ) -> Result<StoredMessage, StoreError> {
        identifier(&commit.operation_id)?;
        identifier(&commit.destination)?;
        validate_message(&commit.message)?;
        validate_states(&commit.states)?;
        validate_states(&retry_states)?;
        records::validate(&commit.states, &records)?;
        if commit.wire.is_empty() || commit.wire.len() > 1024 * 1024 {
            return Err(StoreError::InvalidInput);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous: Option<(Vec<u8>, String)> = tx
            .query_row(
                "SELECT request_hash,message_id FROM operations WHERE operation_id=?1",
                [&commit.operation_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((hash, message_id)) = previous {
            if hash != commit.request_hash {
                return Err(StoreError::IdempotencyConflict);
            }
            let saved = find_message(&tx, &message_id)?.ok_or(StoreError::InvalidProfile)?;
            apply_states(&tx, &retry_states)?;
            tx.commit()?;
            return Ok(saved);
        }
        if find_message(&tx, &commit.message.id)?.is_some() {
            return Err(StoreError::IdempotencyConflict);
        }
        apply_states_with_records(&tx, &commit.states, &records)?;
        let saved = insert_message(&tx, commit.message)?;
        tx.execute(
            "INSERT INTO outbox(message_id,destination,wire) VALUES(?1,?2,?3)",
            params![saved.record.id, commit.destination, commit.wire],
        )?;
        tx.execute(
            "INSERT INTO operations(operation_id,request_hash,message_id) VALUES(?1,?2,?3)",
            params![
                commit.operation_id,
                commit.request_hash.as_slice(),
                saved.record.id
            ],
        )?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn commit_states(&mut self, states: Vec<StateChange>) -> Result<(), StoreError> {
        self.commit_states_with_records(states, vec![])
    }
    /// Host housekeeping can retire a full 128-job queue, its policies and
    /// queue index atomically. Ordinary message commits keep their 16-row bound.
    /// Every removal is revision-fenced and rolls back with the replacement rows.
    pub fn commit_state_maintenance(
        &mut self,
        states: Vec<StateChange>,
        removals: Vec<(String, u64)>,
    ) -> Result<(), StoreError> {
        if states.len().saturating_add(removals.len()) > 257 {
            return Err(StoreError::InvalidInput);
        }
        for batch in states.chunks(16) {
            validate_states(batch)?;
        }
        let mut namespaces = HashSet::new();
        for state in &states {
            if !namespaces.insert(state.namespace.as_str()) {
                return Err(StoreError::InvalidInput);
            }
        }
        for (namespace, revision) in &removals {
            identifier(namespace)?;
            sql_integer(*revision)?;
            if *revision == 0 || !namespaces.insert(namespace.as_str()) {
                return Err(StoreError::InvalidInput);
            }
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        apply_states(&tx, &states)?;
        for (namespace, revision) in removals {
            if tx.execute(
                "DELETE FROM states WHERE namespace=?1 AND revision=?2",
                params![namespace, sql_integer(revision)?],
            )? != 1
            {
                return Err(StoreError::StateConflict);
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn commit_states_with_records(
        &mut self,
        states: Vec<StateChange>,
        records: Vec<StateRecordBatch>,
    ) -> Result<(), StoreError> {
        validate_states(&states)?;
        records::validate(&states, &records)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        apply_states_with_records(&tx, &states, &records)?;
        tx.commit()?;
        Ok(())
    }

    pub fn is_pending(&self, id: &str) -> Result<bool, StoreError> {
        identifier(id)?;
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM outbox WHERE message_id=?1)",
            [id],
            |r| r.get(0),
        )?)
    }
    pub fn message(&self, id: &str) -> Result<Option<StoredMessage>, StoreError> {
        identifier(id)?;
        find_message(&self.connection, id)
    }
    /// The operation result survives removal from the delivery outbox.
    pub fn operation_message(
        &self,
        operation_id: &str,
    ) -> Result<Option<StoredMessage>, StoreError> {
        identifier(operation_id)?;
        Ok(self.connection.query_row(
            &format!("SELECT {MESSAGE_COLUMNS} FROM messages JOIN operations ON messages.id=operations.message_id WHERE operation_id=?1"),
            [operation_id], message_row,
        ).optional()?)
    }

    pub fn commit_incoming(&mut self, commit: IncomingCommit) -> Result<StoredMessage, StoreError> {
        self.commit_incoming_with_records(commit, vec![])
    }
    pub fn commit_incoming_with_records(
        &mut self,
        commit: IncomingCommit,
        records: Vec<StateRecordBatch>,
    ) -> Result<StoredMessage, StoreError> {
        validate_message(&commit.message)?;
        validate_states(&commit.states)?;
        records::validate(&commit.states, &records)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(previous) = find_message(&tx, &commit.message.id)? {
            if previous.record != commit.message {
                return Err(StoreError::IdempotencyConflict);
            }
            return Ok(previous);
        }
        apply_states_with_records(&tx, &commit.states, &records)?;
        let saved = insert_message(&tx, commit.message)?;
        tx.commit()?;
        Ok(saved)
    }
    pub fn messages(
        &self,
        conversation_id: &str,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        identifier(conversation_id)?;
        let cursor = sql_integer(after_sequence)?;
        let limit = page_size(limit)?;
        let mut query=self.connection.prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE conversation_id=?1 AND sequence>?2 ORDER BY sequence LIMIT ?3"))?;
        Ok(query
            .query_map(params![conversation_id, cursor, limit], message_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn pending_outbox(&self, limit: usize) -> Result<Vec<OutboxRecord>, StoreError> {
        let limit = page_size(limit)?;
        let mut query=self.connection.prepare("SELECT m.sequence,m.id,m.conversation_id,m.author,m.content,m.created_at,m.own,o.destination,o.wire FROM outbox o JOIN messages m ON m.id=o.message_id ORDER BY m.sequence LIMIT ?1")?;
        Ok(query
            .query_map([limit], outbox_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Bounded reverse scan of one event kind, newest first. No cursor mutation.
    pub fn event_messages_before(
        &self,
        conversation: &str,
        kind: &str,
        before: Option<u64>,
        limit: usize,
    ) -> Result<Vec<StoredMessage>, StoreError> {
        identifier(conversation)?;
        identifier(kind)?;
        let before = before.map(sql_integer).transpose()?.unwrap_or(i64::MAX);
        let mut query = self.connection.prepare(&format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE conversation_id=?1 AND ({EVENT_KIND})=?2 AND sequence<?3 ORDER BY sequence DESC LIMIT ?4"))?;
        Ok(query
            .query_map(
                params![conversation, kind, before, page_size(limit)?],
                message_row,
            )?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn unread_events(
        &self,
        conversation: &str,
        kind: &str,
        after: u64,
    ) -> Result<u64, StoreError> {
        identifier(conversation)?;
        identifier(kind)?;
        let count: i64 = self.connection.query_row(&format!("SELECT COUNT(*) FROM messages WHERE conversation_id=?1 AND ({EVENT_KIND})=?2 AND own=0 AND sequence>?3"), params![conversation,kind,sql_integer(after)?], |r| r.get(0))?;
        u64::try_from(count).map_err(|_| StoreError::InvalidProfile)
    }

    /// Connection-local invalidation token; meaningful only for this open profile.
    pub fn change_count(&self) -> u64 {
        self.connection.total_changes()
    }
    /// Pending deliveries of own messages whose event kind is not `excluded`,
    /// oldest first, as (message id, conversation id) without their wires.
    pub fn pending_outbox_ids_excluding(
        &self,
        excluded: &str,
        limit: usize,
    ) -> Result<Vec<(String, String)>, StoreError> {
        identifier(excluded)?;
        let mut query = self.connection.prepare(&format!("SELECT m.id,m.conversation_id FROM outbox o JOIN messages m ON m.id=o.message_id WHERE m.own=1 AND ({EVENT_KIND}) IS NOT ?1 ORDER BY m.sequence LIMIT ?2"))?;
        Ok(query
            .query_map(params![excluded, page_size(limit)?], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// One pending delivery by message id.
    pub fn pending_outbox_item(
        &self,
        message_id: &str,
    ) -> Result<Option<OutboxRecord>, StoreError> {
        identifier(message_id)?;
        Ok(self.connection.query_row("SELECT m.sequence,m.id,m.conversation_id,m.author,m.content,m.created_at,m.own,o.destination,o.wire FROM outbox o JOIN messages m ON m.id=o.message_id WHERE o.message_id=?1", [message_id], outbox_row).optional()?)
    }

    /// A message still waiting, sealed again: its record and operation result
    /// stay, its wire is replaced, and the state sealing changed is applied
    /// and fenced states removed, all in one transaction.
    pub fn replace_outbox_wire(
        &mut self,
        message_id: &str,
        wire: Vec<u8>,
        states: Vec<StateChange>,
        records: Vec<StateRecordBatch>,
        removals: Vec<(String, u64)>,
    ) -> Result<(), StoreError> {
        identifier(message_id)?;
        validate_states(&states)?;
        records::validate(&states, &records)?;
        if wire.is_empty() || wire.len() > 1024 * 1024 {
            return Err(StoreError::InvalidInput);
        }
        for (namespace, revision) in &removals {
            identifier(namespace)?;
            sql_integer(*revision)?;
            if *revision == 0 {
                return Err(StoreError::InvalidInput);
            }
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        apply_states_with_records(&tx, &states, &records)?;
        for (namespace, revision) in removals {
            if tx.execute(
                "DELETE FROM states WHERE namespace=?1 AND revision=?2",
                params![namespace, sql_integer(revision)?],
            )? != 1
            {
                return Err(StoreError::StateConflict);
            }
        }
        if tx.execute(
            "UPDATE outbox SET wire=?2 WHERE message_id=?1",
            params![message_id, wire],
        )? != 1
        {
            return Err(StoreError::StateConflict);
        }
        tx.commit()?;
        Ok(())
    }

    pub fn acknowledge_outbox(&mut self, message_id: &str) -> Result<(), StoreError> {
        identifier(message_id)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Move the exact wire before consuming delivery work. A failed write keeps
        // the whole acknowledgment retryable; a duplicate receipt inserts nothing.
        tx.execute("DELETE FROM outbox WHERE message_id=?1", [message_id])?;
        tx.commit()?;
        Ok(())
    }
}
fn outbox_row(r: &Row<'_>) -> rusqlite::Result<OutboxRecord> {
    Ok(OutboxRecord {
        message: message_row(r)?,
        destination: r.get(7)?,
        wire: r.get(8)?,
    })
}
fn private_file(path: &Path) -> Result<File, std::io::Error> {
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
fn sql_integer(value: u64) -> Result<i64, StoreError> {
    value.try_into().map_err(|_| StoreError::InvalidInput)
}
fn identifier(value: &str) -> Result<(), StoreError> {
    if value.is_empty() || value.len() > 256 {
        Err(StoreError::InvalidInput)
    } else {
        Ok(())
    }
}
fn page_size(limit: usize) -> Result<i64, StoreError> {
    if !(1..=1000).contains(&limit) {
        return Err(StoreError::InvalidInput);
    }
    Ok(limit as i64)
}
fn validate_message(message: &MessageRecord) -> Result<(), StoreError> {
    identifier(&message.id)?;
    identifier(&message.conversation_id)?;
    identifier(&message.author)?;
    sql_integer(message.created_at)?;
    if message.content.len() > MAX_BODY_BYTES {
        return Err(StoreError::InvalidInput);
    }
    Ok(())
}
fn validate_states(states: &[StateChange]) -> Result<(), StoreError> {
    if states.len() > 16 {
        return Err(StoreError::InvalidInput);
    }
    let mut namespaces = HashSet::new();
    for state in states {
        identifier(&state.namespace)?;
        sql_integer(
            state
                .expected_revision
                .checked_add(1)
                .ok_or(StoreError::InvalidInput)?,
        )?;
        if state.bytes.len() > 32 * 1024 * 1024 || !namespaces.insert(&state.namespace) {
            return Err(StoreError::InvalidInput);
        }
    }
    Ok(())
}
fn apply_states(connection: &Connection, states: &[StateChange]) -> Result<(), StoreError> {
    apply_states_with_records(connection, states, &[])
}
fn apply_states_with_records(
    connection: &Connection,
    states: &[StateChange],
    records: &[StateRecordBatch],
) -> Result<(), StoreError> {
    for state in states {
        let changed = if state.expected_revision == 0 {
            connection.execute("INSERT INTO states(namespace,revision,bytes) VALUES(?1,1,?2) ON CONFLICT(namespace) DO NOTHING",params![state.namespace,state.bytes])?
        } else {
            connection.execute(
                "UPDATE states SET revision=revision+1,bytes=?1 WHERE namespace=?2 AND revision=?3",
                params![
                    state.bytes,
                    state.namespace,
                    sql_integer(state.expected_revision)?
                ],
            )?
        };
        if changed != 1 {
            return Err(StoreError::StateConflict);
        }
        if let Some(batch) = records
            .iter()
            .find(|batch| batch.namespace == state.namespace)
        {
            records::apply(connection, batch)?;
        } else {
            // A complete legacy write replaces any fragmented representation.
            // This also keeps existing snapshot-based tools compatible.
            connection.execute(
                "DELETE FROM state_records WHERE namespace=?1",
                [&state.namespace],
            )?;
        }
    }
    Ok(())
}
fn message_row(row: &Row<'_>) -> rusqlite::Result<StoredMessage> {
    Ok(StoredMessage {
        sequence: read_u64(row, 0)?,
        record: MessageRecord {
            id: row.get(1)?,
            conversation_id: row.get(2)?,
            author: row.get(3)?,
            content: row.get(4)?,
            created_at: read_u64(row, 5)?,
            own: row.get(6)?,
        },
    })
}
fn find_message(connection: &Connection, id: &str) -> Result<Option<StoredMessage>, StoreError> {
    Ok(connection
        .query_row(
            &format!("SELECT {MESSAGE_COLUMNS} FROM messages WHERE id=?1"),
            [id],
            message_row,
        )
        .optional()?)
}
fn insert_message(
    connection: &Connection,
    message: MessageRecord,
) -> Result<StoredMessage, StoreError> {
    connection.execute("INSERT INTO messages(id,conversation_id,author,content,created_at,own) VALUES(?1,?2,?3,?4,?5,?6)",params![message.id,message.conversation_id,message.author,message.content,sql_integer(message.created_at)?,message.own])?;
    let sequence = connection
        .last_insert_rowid()
        .try_into()
        .map_err(|_| StoreError::InvalidProfile)?;
    Ok(StoredMessage {
        sequence,
        record: message,
    })
}

fn read_u64(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    value
        .try_into()
        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

impl Drop for StateChange {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}
impl Drop for StateValue {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}
