#![allow(clippy::unwrap_used, clippy::expect_used)]

use agentic_protocol::{DocumentDraft, DocumentKind, VerifiedDocument};
use agentic_store::{
    IncomingCommit, MessageRecord, OutgoingCommit, ProfileStore, StateChange, StateRecordBatch,
    StateRecordChange, StoreError,
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
use tempfile::TempDir;

const KEY: [u8; 32] = [0x23; 32];
const SECRET: &[u8] = b"private-history-sentinel-and-epoch-key-state";

#[path = "support/state_range.rs"]
mod state_range;

fn open(root: &TempDir) -> ProfileStore {
    ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap()
}
fn message(id: &str, own: bool) -> MessageRecord {
    MessageRecord {
        id: id.into(),
        conversation_id: "conversation-alice-bob".into(),
        author: if own { "alice" } else { "bob" }.into(),
        content: SECRET.to_vec(),
        created_at: 1_788_480_000,
        own,
    }
}
fn state(expected_revision: u64, bytes: &[u8]) -> StateChange {
    StateChange {
        namespace: "mls/conversation-alice-bob".into(),
        expected_revision,
        bytes: bytes.to_vec(),
    }
}
fn outgoing() -> OutgoingCommit {
    OutgoingCommit {
        operation_id: "send-1".into(),
        request_hash: [1; 32],
        message: message("message-1", true),
        destination: "bob-network-id".into(),
        wire: b"opaque-signed-ciphertext".to_vec(),
        states: vec![state(0, SECRET)],
    }
}
fn raw_encrypted_db(path: &Path) -> Connection {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", hex::encode(KEY)))
        .unwrap();
    conn
}

fn migration_backup(root: &TempDir) -> std::path::PathBuf {
    root.path().join("profile.db.migration-v1.bak")
}

fn schema_envelope(root: &TempDir) -> (i64, i64, i64, i64) {
    let conn = raw_encrypted_db(&root.path().join("profile.db"));
    conn.query_row(
        "SELECT schema_version,supported_read,supported_write,migration_step FROM profile_schema WHERE singleton=1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .unwrap()
}

#[test]
fn new_profile_has_one_explicit_schema_envelope_and_current_write_version() {
    let root = TempDir::new().unwrap();
    let store = open(&root);
    drop(store);

    let db = raw_encrypted_db(&root.path().join("profile.db"));
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(schema_envelope(&root), (2, 1, 2, 2));
    assert!(!migration_backup(&root).exists());
}

#[test]
fn forward_profile_upgrade_preserves_identity_outbox_mls_and_grants_in_one_reopen() {
    let root = TempDir::new().unwrap();
    let (identity, saved) = {
        let mut store = open(&root);
        let identity = store.identity().unwrap();
        let saved = store.commit_outgoing(outgoing()).unwrap();
        store
            .commit_states(vec![StateChange {
                namespace: "grants/company/alice".into(),
                expected_revision: 0,
                bytes: b"grant-state-that-must-survive".to_vec(),
            }])
            .unwrap();
        (identity, saved)
    };

    // Reproduce a pre-envelope v1 profile while retaining every real profile row.
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    db.execute_batch("DROP TABLE IF EXISTS profile_schema; PRAGMA user_version=1;")
        .unwrap();
    drop(db);
    assert!(!migration_backup(&root).exists());

    let reopened = open(&root);
    assert_eq!(reopened.identity().unwrap(), identity);
    assert_eq!(
        reopened.messages("conversation-alice-bob", 0, 100).unwrap(),
        vec![saved.clone()]
    );
    let pending = reopened.pending_outbox(100).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].destination, "bob-network-id");
    assert_eq!(pending[0].wire, b"opaque-signed-ciphertext");
    assert_eq!(
        reopened.operation_message("send-1").unwrap(),
        Some(saved.clone())
    );
    assert_eq!(
        reopened
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        SECRET
    );
    assert_eq!(
        reopened
            .state("grants/company/alice")
            .unwrap()
            .unwrap()
            .bytes,
        b"grant-state-that-must-survive"
    );
    drop(reopened);

    assert!(migration_backup(&root).is_file());
    let backup = fs::read(migration_backup(&root)).unwrap();
    assert!(!backup.is_empty());
    assert!(!backup.starts_with(b"SQLite format 3"));
    let backup_db = raw_encrypted_db(&migration_backup(&root));
    assert_eq!(
        backup_db
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='profile_schema'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        0
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT length(owner_seed) FROM profile WHERE singleton=1",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        32
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT content FROM messages WHERE id='message-1'",
                [],
                |row| row.get::<_, Vec<u8>>(0)
            )
            .unwrap(),
        SECRET
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT destination,wire FROM outbox WHERE message_id='message-1'",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .unwrap(),
        (
            "bob-network-id".to_string(),
            b"opaque-signed-ciphertext".to_vec()
        )
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT count(*) FROM operations WHERE operation_id='send-1'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    drop(backup_db);
    let backup_before_reopen = fs::read(migration_backup(&root)).unwrap();
    drop(ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap());
    assert_eq!(
        fs::read(migration_backup(&root)).unwrap(),
        backup_before_reopen
    );
    assert_eq!(schema_envelope(&root), (2, 1, 2, 2));
}

#[test]
fn wrong_key_cannot_start_a_forward_migration_or_create_a_backup() {
    let root = TempDir::new().unwrap();
    {
        let mut store = open(&root);
        store.commit_outgoing(outgoing()).unwrap();
        store
            .commit_states(vec![StateChange {
                namespace: "grants/company/alice".into(),
                expected_revision: 0,
                bytes: b"grant-state-that-must-survive".to_vec(),
            }])
            .unwrap();
        drop(store);
    }
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    db.execute_batch("DROP TABLE IF EXISTS profile_schema; PRAGMA user_version=1;")
        .unwrap();
    drop(db);

    assert!(ProfileStore::open(root.path().join("profile.db"), &[0x24; 32]).is_err());
    assert!(!migration_backup(&root).exists());

    let store = open(&root);
    let pending = store.pending_outbox(100).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].destination, "bob-network-id");
    assert_eq!(pending[0].wire, b"opaque-signed-ciphertext");
    assert_eq!(
        store
            .operation_message("send-1")
            .unwrap()
            .unwrap()
            .record
            .content,
        SECRET
    );
    assert_eq!(
        store.state("grants/company/alice").unwrap().unwrap().bytes,
        b"grant-state-that-must-survive"
    );
    drop(store);
    assert!(migration_backup(&root).is_file());
}

#[test]
fn invalid_schema_envelope_is_rejected_without_resetting_profile_rows() {
    let root = TempDir::new().unwrap();
    {
        let mut store = open(&root);
        store.commit_outgoing(outgoing()).unwrap();
    }
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    db.execute_batch(
        "DROP TABLE IF EXISTS profile_schema;
         CREATE TABLE profile_schema (
             singleton INTEGER PRIMARY KEY CHECK(singleton=1),
             schema_version INTEGER NOT NULL,
             supported_read INTEGER NOT NULL,
             supported_write INTEGER NOT NULL,
             migration_step INTEGER NOT NULL
         );
         INSERT INTO profile_schema VALUES(1, 1, 1, 1, 0);
         PRAGMA user_version=1;",
    )
    .unwrap();
    drop(db);

    assert!(matches!(
        ProfileStore::open(root.path().join("profile.db"), &KEY),
        Err(StoreError::InvalidProfile),
    ));
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM messages WHERE id='message-1'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM outbox WHERE message_id='message-1'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT content FROM messages WHERE id='message-1'",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        SECRET
    );
    assert_eq!(
        db.query_row(
            "SELECT wire FROM outbox WHERE message_id='message-1'",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        b"opaque-signed-ciphertext"
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operations WHERE operation_id='send-1'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    drop(db);
}

#[test]
fn failed_forward_migration_rolls_back_marker_and_resumes_from_the_same_backup() {
    let root = TempDir::new().unwrap();
    let identity = {
        let mut store = open(&root);
        let identity = store.identity().unwrap();
        store.commit_outgoing(outgoing()).unwrap();
        store
            .commit_states(vec![StateChange {
                namespace: "grants/company/alice".into(),
                expected_revision: 0,
                bytes: b"grant-state-that-must-survive".to_vec(),
            }])
            .unwrap();
        identity
    };
    let owner_seed = {
        let db = raw_encrypted_db(&root.path().join("profile.db"));
        db.query_row(
            "SELECT owner_seed FROM profile WHERE singleton=1",
            [],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .unwrap()
    };
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    db.execute_batch(
        "DROP TABLE IF EXISTS profile_schema;
         CREATE TABLE profile_schema (
             singleton INTEGER PRIMARY KEY CHECK(singleton=1),
             schema_version INTEGER NOT NULL,
             supported_read INTEGER NOT NULL,
             supported_write INTEGER NOT NULL,
             migration_step INTEGER NOT NULL
         );
         INSERT INTO profile_schema VALUES(1, 1, 1, 1, 1);
         CREATE TRIGGER fail_profile_migration
         BEFORE UPDATE ON profile_schema
         WHEN NEW.schema_version=2
         BEGIN SELECT RAISE(ABORT,'injected migration failure'); END;
         PRAGMA user_version=1;",
    )
    .unwrap();
    drop(db);

    assert!(ProfileStore::open(root.path().join("profile.db"), &KEY).is_err());
    assert!(migration_backup(&root).is_file());
    let backup_before_retry = fs::read(migration_backup(&root)).unwrap();
    let backup_db = raw_encrypted_db(&migration_backup(&root));
    assert_eq!(
        backup_db
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT content FROM messages WHERE id='message-1'",
                [],
                |row| row.get::<_, Vec<u8>>(0)
            )
            .unwrap(),
        SECRET
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT wire FROM outbox WHERE message_id='message-1'",
                [],
                |row| row.get::<_, Vec<u8>>(0)
            )
            .unwrap(),
        b"opaque-signed-ciphertext"
    );
    assert_eq!(
        backup_db
            .query_row(
                "SELECT bytes FROM states WHERE namespace='grants/company/alice'",
                [],
                |row| row.get::<_, Vec<u8>>(0)
            )
            .unwrap(),
        b"grant-state-that-must-survive"
    );
    drop(backup_db);
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row(
            "SELECT schema_version,supported_read,supported_write,migration_step FROM profile_schema WHERE singleton=1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?)),
        )
        .unwrap(),
        (1, 1, 1, 1)
    );
    assert_eq!(
        db.query_row(
            "SELECT content FROM messages WHERE id='message-1'",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        SECRET
    );
    assert_eq!(
        db.query_row(
            "SELECT wire FROM outbox WHERE message_id='message-1'",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        b"opaque-signed-ciphertext"
    );
    assert_eq!(
        db.query_row(
            "SELECT bytes FROM states WHERE namespace='grants/company/alice'",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        b"grant-state-that-must-survive"
    );
    assert_eq!(
        db.query_row(
            "SELECT owner_seed FROM profile WHERE singleton=1",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        owner_seed
    );
    assert_eq!(
        db.query_row(
            "SELECT bytes FROM states WHERE namespace='mls/conversation-alice-bob'",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        SECRET
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM operations WHERE operation_id='send-1'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    drop(db);

    let db = raw_encrypted_db(&root.path().join("profile.db"));
    db.execute_batch("DROP TRIGGER fail_profile_migration")
        .unwrap();
    drop(db);
    let store = open(&root);
    assert_eq!(store.identity().unwrap(), identity);
    let pending = store.pending_outbox(100).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].destination, "bob-network-id");
    assert_eq!(pending[0].wire, b"opaque-signed-ciphertext");
    assert_eq!(
        store
            .operation_message("send-1")
            .unwrap()
            .unwrap()
            .record
            .content,
        SECRET
    );
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        SECRET
    );
    assert_eq!(
        store.state("grants/company/alice").unwrap().unwrap().bytes,
        b"grant-state-that-must-survive"
    );
    drop(store);
    assert_eq!(schema_envelope(&root), (2, 1, 2, 2));
    assert_eq!(
        fs::read(migration_backup(&root)).unwrap(),
        backup_before_retry
    );
}

#[test]
fn operation_lookup_returns_exact_committed_message_after_reopen_without_consuming_delivery() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    assert!(store.operation_message("send-1").unwrap().is_none());
    let first = store.commit_outgoing(outgoing()).unwrap();
    let mut other = outgoing();
    other.operation_id = "send-2".into();
    other.message.id = "message-2".into();
    other.states.clear();
    let second = store.commit_outgoing(other).unwrap();
    assert_eq!(
        store.operation_message("send-1").unwrap(),
        Some(first.clone())
    );
    assert_eq!(
        store.operation_message("send-2").unwrap(),
        Some(second.clone())
    );
    assert!(store.operation_message("message-1").unwrap().is_none());
    assert_eq!(store.pending_outbox(10).unwrap().len(), 2);
    drop(store);
    let mut store = open(&root);
    store.acknowledge_outbox(&first.record.id).unwrap();
    assert_eq!(
        store.operation_message("send-1").unwrap(),
        Some(first.clone())
    );
    assert_eq!(store.operation_message("send-2").unwrap(), Some(second));
    assert_eq!(store.pending_outbox(10).unwrap().len(), 1);
    assert!(!store.is_pending(&first.record.id).unwrap());
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert!(store.operation_message("").is_err());
}

#[test]
fn authorized_retry_commits_only_retry_state_without_replacing_original_message_or_mls() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let nonce = |revision, bytes: &[u8]| StateChange {
        namespace: "authorization/nonces".into(),
        expected_revision: revision,
        bytes: bytes.to_vec(),
    };
    let mut first = outgoing();
    first.states.push(nonce(0, b"nonce-one"));
    let saved = store
        .commit_outgoing_with_retry_states(first, vec![nonce(999, b"retry-only")])
        .unwrap();
    assert_eq!(
        store.state("authorization/nonces").unwrap().unwrap().bytes,
        b"nonce-one"
    );
    drop(store);
    let mut store = open(&root);
    let mut retry = outgoing();
    retry.message.id = "fresh-preparation-id".into();
    retry.wire = b"must not replace original wire".to_vec();
    retry.states = vec![
        state(1, b"must not advance"),
        nonce(1, b"wrong nonce state"),
    ];
    assert_eq!(
        store
            .commit_outgoing_with_retry_states(retry, vec![nonce(1, b"nonce-one,two")])
            .unwrap(),
        saved
    );
    drop(store);
    let mut store = open(&root);
    let persisted = store.state("authorization/nonces").unwrap().unwrap();
    assert_eq!(
        (persisted.revision, persisted.bytes.as_slice()),
        (2, b"nonce-one,two".as_slice())
    );
    let mls = store.state("mls/conversation-alice-bob").unwrap().unwrap();
    assert_eq!((mls.revision, mls.bytes.as_slice()), (1, SECRET));
    assert_eq!(
        store.messages("conversation-alice-bob", 0, 100).unwrap(),
        vec![saved]
    );
    assert_eq!(store.pending_outbox(100).unwrap().len(), 1);
    assert_eq!(
        store.pending_outbox(100).unwrap()[0].wire,
        b"opaque-signed-ciphertext"
    );
    let mut conflict = outgoing();
    conflict.request_hash = [2; 32];
    assert!(matches!(
        store.commit_outgoing_with_retry_states(conflict, vec![nonce(2, b"conflict")]),
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(
        store
            .state("authorization/nonces")
            .unwrap()
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn authorization_retry_failure_rolls_back_earlier_retry_states_and_original_work_survives() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let saved = store.commit_outgoing(outgoing()).unwrap();
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    db.execute_batch("CREATE TRIGGER fail_nonce BEFORE INSERT ON states WHEN NEW.namespace='authorization/nonces' BEGIN SELECT RAISE(ABORT,'injected nonce write failure'); END;").unwrap();
    let states = vec![
        StateChange {
            namespace: "authorization/fence".into(),
            expected_revision: 0,
            bytes: b"fence".to_vec(),
        },
        StateChange {
            namespace: "authorization/nonces".into(),
            expected_revision: 0,
            bytes: b"nonce".to_vec(),
        },
    ];
    assert!(
        store
            .commit_outgoing_with_retry_states(outgoing(), states.clone())
            .is_err()
    );
    drop(store);
    let mut store = open(&root);
    assert!(store.state("authorization/fence").unwrap().is_none());
    assert!(store.state("authorization/nonces").unwrap().is_none());
    let mls = store.state("mls/conversation-alice-bob").unwrap().unwrap();
    assert_eq!((mls.revision, mls.bytes.as_slice()), (1, SECRET));
    assert_eq!(
        store.messages("conversation-alice-bob", 0, 100).unwrap(),
        vec![saved.clone()]
    );
    assert_eq!(store.pending_outbox(100).unwrap().len(), 1);
    assert_eq!(
        store.pending_outbox(100).unwrap()[0].wire,
        b"opaque-signed-ciphertext"
    );
    db.execute_batch("DROP TRIGGER fail_nonce").unwrap();
    assert_eq!(
        store
            .commit_outgoing_with_retry_states(outgoing(), states)
            .unwrap(),
        saved
    );
    assert_eq!(
        store.state("authorization/nonces").unwrap().unwrap().bytes,
        b"nonce"
    );
    assert_eq!(
        store.state("authorization/fence").unwrap().unwrap().bytes,
        b"fence"
    );
}

#[test]
fn a_profile_keeps_its_address_and_signing_authority_across_restarts() {
    let root = TempDir::new().unwrap();
    let identity = {
        let store = open(&root);
        store.identity().unwrap()
    };
    let store = open(&root);
    assert_eq!(store.identity().unwrap(), identity);
    assert_eq!(
        identity.network_id,
        format!("ain1{}", hex::encode(Sha256::digest(identity.public_key)))
    );
    let draft = DocumentDraft {
        domain: [7; 32],
        kind: DocumentKind::Identity,
        authority_epoch: 0,
        issued_at: 100,
        expires_at: None,
        body: b"public identity announcement".to_vec(),
        extensions: BTreeMap::new(),
    };
    let wire = store.sign_document(draft).unwrap().to_wire();
    let verified = VerifiedDocument::decode(&wire, [7; 32], 100).unwrap();
    assert_eq!(verified.author(), &identity.public_key);
}

#[test]
fn independently_created_profiles_do_not_share_an_identity_or_default_seed() {
    let alice = TempDir::new().unwrap();
    let bob = TempDir::new().unwrap();
    assert_ne!(
        open(&alice).identity().unwrap(),
        open(&bob).identity().unwrap()
    );
}

#[test]
fn wrong_master_key_does_not_open_or_reinitialize_existing_history() {
    let root = TempDir::new().unwrap();
    let original = {
        let mut store = open(&root);
        store.commit_outgoing(outgoing()).unwrap();
        store.identity().unwrap()
    };
    assert!(ProfileStore::open(root.path().join("profile.db"), &[0x24; 32]).is_err());
    let store = open(&root);
    assert_eq!(store.identity().unwrap(), original);
    assert_eq!(
        store.messages("conversation-alice-bob", 0, 100).unwrap()[0]
            .record
            .content,
        SECRET
    );
}

#[test]
fn plaintext_and_master_key_are_absent_from_database_and_wal_files() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    store.commit_outgoing(outgoing()).unwrap();
    let conn = raw_encrypted_db(&root.path().join("profile.db"));
    assert_eq!(
        conn.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert!(
        !fs::read(root.path().join("profile.db"))
            .unwrap()
            .starts_with(b"SQLite format 3")
    );
    for file in fs::read_dir(root.path()).unwrap() {
        let bytes = fs::read(file.unwrap().path()).unwrap();
        assert!(!bytes.windows(SECRET.len()).any(|part| part == SECRET));
        assert!(!bytes.windows(KEY.len()).any(|part| part == KEY));
    }
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        SECRET
    );
}

#[test]
fn a_second_writer_cannot_open_an_already_active_profile() {
    let root = TempDir::new().unwrap();
    let first = open(&root);
    assert!(matches!(
        ProfileStore::open(root.path().join("profile.db"), &KEY),
        Err(StoreError::ProfileInUse)
    ));
    drop(first);
    assert!(ProfileStore::open(root.path().join("profile.db"), &KEY).is_ok());
}

#[test]
fn outgoing_state_history_and_transport_work_survive_reopen_together() {
    let root = TempDir::new().unwrap();
    let saved = {
        let mut store = open(&root);
        store.commit_outgoing(outgoing()).unwrap()
    };
    let store = open(&root);
    let pending = store.pending_outbox(100).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].message, saved);
    assert_eq!(pending[0].destination, "bob-network-id");
    assert_eq!(pending[0].wire, b"opaque-signed-ciphertext");
    assert_eq!(
        store.messages("conversation-alice-bob", 0, 100).unwrap(),
        vec![saved]
    );
    let saved_state = store.state("mls/conversation-alice-bob").unwrap().unwrap();
    assert_eq!(saved_state.revision, 1);
    assert_eq!(saved_state.bytes, SECRET);
}

#[test]
fn a_lost_response_retry_returns_the_original_ciphertext_without_advancing_crypto_state() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let saved = store.commit_outgoing(outgoing()).unwrap();
    let mut retry = outgoing();
    retry.wire = b"newly prepared but must never be sent".to_vec();
    retry.states = vec![state(1, b"must not commit")];
    assert_eq!(store.commit_outgoing(retry).unwrap(), saved);
    assert_eq!(store.pending_outbox(100).unwrap().len(), 1);
    assert_eq!(
        store.pending_outbox(100).unwrap()[0].wire,
        b"opaque-signed-ciphertext"
    );
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        SECRET
    );
}

#[test]
fn ordinary_send_receive_send_advances_crypto_state_and_reopens_at_the_latest_revision() {
    let root = TempDir::new().unwrap();
    let first = outgoing();
    let first_record = first.message.clone();
    let received = message("received", false);
    let mut second = outgoing();
    second.operation_id = "send-2".into();
    second.request_hash = [2; 32];
    second.message.id = "message-2".into();
    second.message.content = b"third message".to_vec();
    second.states = vec![state(2, b"epoch-state-3")];
    let second_record = second.message.clone();
    {
        let mut store = open(&root);
        assert_eq!(store.commit_outgoing(first).unwrap().record, first_record);
        assert_eq!(
            store
                .state("mls/conversation-alice-bob")
                .unwrap()
                .unwrap()
                .revision,
            1
        );
        assert_eq!(
            store
                .commit_incoming(IncomingCommit {
                    message: received.clone(),
                    states: vec![state(1, b"epoch-state-2")]
                })
                .unwrap()
                .record,
            received
        );
        let updated = store.state("mls/conversation-alice-bob").unwrap().unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.bytes, b"epoch-state-2");
        assert_eq!(store.commit_outgoing(second).unwrap().record, second_record);
    }
    let mut store = open(&root);
    let updated = store.state("mls/conversation-alice-bob").unwrap().unwrap();
    assert_eq!(updated.revision, 3);
    assert_eq!(updated.bytes, b"epoch-state-3");
    let records: Vec<_> = store
        .messages("conversation-alice-bob", 0, 100)
        .unwrap()
        .into_iter()
        .map(|m| m.record)
        .collect();
    assert_eq!(
        records,
        vec![first_record.clone(), received, second_record.clone()]
    );
    let queued: Vec<_> = store
        .pending_outbox(100)
        .unwrap()
        .into_iter()
        .map(|o| o.message.record)
        .collect();
    assert_eq!(queued, vec![first_record, second_record.clone()]);
    store.acknowledge_outbox("message-1").unwrap();
    let queued = store.pending_outbox(100).unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].message.record, second_record);
    assert_eq!(
        store
            .messages("conversation-alice-bob", 0, 100)
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn changed_command_cannot_reuse_an_operation_id() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    store.commit_outgoing(outgoing()).unwrap();
    let mut changed = outgoing();
    changed.request_hash = [2; 32];
    changed.message.content = b"another command".to_vec();
    assert!(matches!(
        store.commit_outgoing(changed),
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(
        store
            .messages("conversation-alice-bob", 0, 100)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        SECRET
    );
}

#[test]
fn stale_crypto_revision_cannot_commit_a_message_or_outbox_item() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    store.commit_outgoing(outgoing()).unwrap();
    let mut second = outgoing();
    second.operation_id = "send-2".into();
    second.request_hash = [2; 32];
    second.message.id = "message-2".into();
    assert!(matches!(
        store.commit_outgoing(second),
        Err(StoreError::StateConflict)
    ));
    assert_eq!(
        store
            .messages("conversation-alice-bob", 0, 100)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(store.pending_outbox(100).unwrap().len(), 1);
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn database_failure_at_each_write_boundary_rolls_back_the_whole_operation() {
    for table in ["states", "messages", "outbox"] {
        let root = TempDir::new().unwrap();
        let mut store = open(&root);
        let conn = raw_encrypted_db(&root.path().join("profile.db"));
        conn.execute_batch(&format!("CREATE TRIGGER fail_write BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'injected write failure'); END;")).unwrap();
        assert!(
            store.commit_outgoing(outgoing()).is_err(),
            "fault table {table}"
        );
        assert!(store.state("mls/conversation-alice-bob").unwrap().is_none());
        assert!(
            store
                .messages("conversation-alice-bob", 0, 100)
                .unwrap()
                .is_empty()
        );
        assert!(store.pending_outbox(100).unwrap().is_empty());
        conn.execute_batch("DROP TRIGGER fail_write").unwrap();
        // The failed operation must not leave behind a dedup claim either.
        assert!(store.commit_outgoing(outgoing()).is_ok());
    }
}

#[test]
fn delivered_outbox_ack_preserves_history_and_dedup_after_restart() {
    let root = TempDir::new().unwrap();
    {
        let mut store = open(&root);
        store.commit_outgoing(outgoing()).unwrap();
        store.acknowledge_outbox("message-1").unwrap();
    }
    let mut store = open(&root);
    assert!(store.pending_outbox(100).unwrap().is_empty());
    let previous = store.messages("conversation-alice-bob", 0, 100).unwrap();
    assert_eq!(previous.len(), 1);
    assert_eq!(store.commit_outgoing(outgoing()).unwrap(), previous[0]);
    assert!(store.pending_outbox(100).unwrap().is_empty());
}

#[test]
fn duplicate_incoming_delivery_is_one_message_and_one_crypto_transition() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let incoming = IncomingCommit {
        message: message("received-1", false),
        states: vec![state(0, SECRET)],
    };
    let saved = store.commit_incoming(incoming.clone()).unwrap();
    assert_eq!(store.commit_incoming(incoming).unwrap(), saved);
    assert_eq!(
        store.messages("conversation-alice-bob", 0, 100).unwrap(),
        vec![saved]
    );
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert!(store.pending_outbox(100).unwrap().is_empty());
}

#[test]
fn conflicting_incoming_message_id_cannot_replace_accepted_content() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    store
        .commit_incoming(IncomingCommit {
            message: message("received-1", false),
            states: vec![],
        })
        .unwrap();
    let mut changed = message("received-1", false);
    changed.content = b"forged replacement".to_vec();
    assert!(matches!(
        store.commit_incoming(IncomingCommit {
            message: changed,
            states: vec![]
        }),
        Err(StoreError::IdempotencyConflict)
    ));
    assert_eq!(
        store.messages("conversation-alice-bob", 0, 100).unwrap()[0]
            .record
            .content,
        SECRET
    );
}

#[test]
fn messages_can_be_resumed_by_cursor_without_crossing_conversations() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let first = store
        .commit_incoming(IncomingCommit {
            message: message("first", false),
            states: vec![],
        })
        .unwrap();
    let mut foreign = message("foreign", false);
    foreign.conversation_id = "another-room".into();
    store
        .commit_incoming(IncomingCommit {
            message: foreign,
            states: vec![],
        })
        .unwrap();
    let second = store
        .commit_incoming(IncomingCommit {
            message: message("second", false),
            states: vec![],
        })
        .unwrap();
    assert!(second.sequence > first.sequence);
    assert_eq!(
        store
            .messages("conversation-alice-bob", first.sequence, 1)
            .unwrap(),
        vec![second]
    );
}

#[test]
fn multi_state_changes_are_atomic_and_duplicate_namespaces_are_invalid() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let mut commit = outgoing();
    commit.states.push(StateChange {
        namespace: "projection/conversation-alice-bob".into(),
        expected_revision: 0,
        bytes: b"local view".to_vec(),
    });
    store.commit_outgoing(commit).unwrap();
    assert_eq!(
        store
            .state("projection/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        b"local view"
    );
    let mut bad = outgoing();
    bad.operation_id = "second".into();
    bad.request_hash = [2; 32];
    bad.message.id = "second".into();
    bad.states = vec![state(1, b"one"), state(2, b"two")];
    assert!(matches!(
        store.commit_outgoing(bad),
        Err(StoreError::InvalidInput)
    ));
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn oversized_content_and_invalid_page_sizes_fail_without_persisting() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let mut commit = outgoing();
    commit.message.content = vec![1; 49_153];
    assert!(matches!(
        store.commit_outgoing(commit),
        Err(StoreError::InvalidInput)
    ));
    assert!(store.pending_outbox(100).unwrap().is_empty());
    assert!(store.messages("conversation-alice-bob", 0, 0).is_err());
    assert!(store.messages("conversation-alice-bob", 0, 1001).is_err());
}

#[test]
fn stale_later_state_rolls_back_earlier_state_changes_and_incoming_history() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    store
        .commit_incoming(IncomingCommit {
            message: message("first", false),
            states: vec![state(0, SECRET)],
        })
        .unwrap();
    let changes = vec![
        StateChange {
            namespace: "new-state".into(),
            expected_revision: 0,
            bytes: b"must roll back".to_vec(),
        },
        state(0, b"stale"),
    ];
    assert!(matches!(
        store.commit_incoming(IncomingCommit {
            message: message("second", false),
            states: changes
        }),
        Err(StoreError::StateConflict)
    ));
    assert!(store.state("new-state").unwrap().is_none());
    assert_eq!(
        store
            .messages("conversation-alice-bob", 0, 100)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        SECRET
    );
}

#[test]
fn incoming_message_write_failure_rolls_back_its_crypto_transition() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let conn = raw_encrypted_db(&root.path().join("profile.db"));
    conn.execute_batch("CREATE TRIGGER fail_incoming BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT,'disk write failed'); END;").unwrap();
    let commit = IncomingCommit {
        message: message("received", false),
        states: vec![state(0, SECRET)],
    };
    assert!(store.commit_incoming(commit.clone()).is_err());
    assert!(store.state("mls/conversation-alice-bob").unwrap().is_none());
    assert!(
        store
            .messages("conversation-alice-bob", 0, 100)
            .unwrap()
            .is_empty()
    );
    conn.execute_batch("DROP TRIGGER fail_incoming").unwrap();
    assert!(store.commit_incoming(commit).is_ok());
}

#[test]
fn unbounded_wire_identifiers_state_batches_and_integer_overflow_are_rejected() {
    for case in 0..6 {
        let root = TempDir::new().unwrap();
        let mut store = open(&root);
        let mut commit = outgoing();
        match case {
            0 => commit.wire = vec![0; 1_048_577],
            1 => commit.operation_id = "x".repeat(257),
            2 => {
                commit.states = (0..17)
                    .map(|i| StateChange {
                        namespace: format!("state/{i}"),
                        expected_revision: 0,
                        bytes: vec![],
                    })
                    .collect()
            }
            3 => commit.message.created_at = u64::MAX,
            4 => commit.states[0].expected_revision = u64::MAX,
            _ => commit.states[0].bytes = vec![0; 32 * 1024 * 1024 + 1],
        }
        assert!(
            matches!(store.commit_outgoing(commit), Err(StoreError::InvalidInput)),
            "case {case}"
        );
        assert!(store.pending_outbox(100).unwrap().is_empty());
        assert!(
            store
                .messages("conversation-alice-bob", 0, 100)
                .unwrap()
                .is_empty()
        );
        assert!(store.state("mls/conversation-alice-bob").unwrap().is_none());
    }
}

#[test]
fn metadata_only_updates_reuse_atomic_cas_without_creating_chat_or_outbox_records() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    store
        .commit_states(vec![
            state(0, b"first state"),
            StateChange {
                namespace: "profile".into(),
                expected_revision: 0,
                bytes: b"Alice".to_vec(),
            },
        ])
        .unwrap();
    assert!(store.pending_outbox(100).unwrap().is_empty());
    assert!(
        store
            .messages("conversation-alice-bob", 0, 100)
            .unwrap()
            .is_empty()
    );
    let bad = vec![
        state(1, b"must roll back"),
        StateChange {
            namespace: "profile".into(),
            expected_revision: 0,
            bytes: b"stale profile".to_vec(),
        },
    ];
    assert!(matches!(
        store.commit_states(bad),
        Err(StoreError::StateConflict)
    ));
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .bytes,
        b"first state"
    );
    drop(store);
    let store = open(&root);
    assert_eq!(store.state("profile").unwrap().unwrap().bytes, b"Alice");
    assert!(store.message("missing-message").unwrap().is_none());
}

#[test]
fn direct_message_lookup_returns_the_exact_persisted_record_after_reopen() {
    let root = TempDir::new().unwrap();
    let expected = message("lookup-1", false);
    let saved = {
        let mut store = open(&root);
        store
            .commit_incoming(IncomingCommit {
                message: expected.clone(),
                states: vec![],
            })
            .unwrap()
    };
    let store = open(&root);
    let found = store.message("lookup-1").unwrap().unwrap();
    assert_eq!(found.record, expected);
    assert_eq!(found.sequence, saved.sequence);
    assert!(store.message("another-id").unwrap().is_none());
}

#[test]
fn a_waiting_message_sealed_again_keeps_its_record_and_moves_its_state_in_one_step() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let saved = store.commit_outgoing(outgoing()).unwrap();
    let pin = "swarm/send/message-1";
    store
        .commit_states(vec![StateChange {
            namespace: pin.into(),
            expected_revision: 0,
            bytes: b"pinned".to_vec(),
        }])
        .unwrap();
    // A stale fence, on the pin or on the state, or an empty wire, changes
    // nothing: not the wire, the state or the pin.
    assert!(matches!(
        store.replace_outbox_wire(
            "message-1",
            b"sealed-again".to_vec(),
            vec![state(2, b"next")],
            vec![],
            vec![(pin.into(), 1)],
        ),
        Err(StoreError::StateConflict)
    ));
    assert!(matches!(
        store.replace_outbox_wire(
            "message-1",
            vec![],
            vec![state(1, b"next")],
            vec![],
            vec![(pin.into(), 1)],
        ),
        Err(StoreError::InvalidInput)
    ));
    assert!(matches!(
        store.replace_outbox_wire(
            "message-2",
            b"sealed-again".to_vec(),
            vec![state(1, b"next")],
            vec![],
            vec![(pin.into(), 1)],
        ),
        Err(StoreError::StateConflict)
    ));
    assert!(matches!(
        store.replace_outbox_wire(
            "message-1",
            b"sealed-again".to_vec(),
            vec![state(1, b"next")],
            vec![],
            vec![(pin.into(), 2)],
        ),
        Err(StoreError::StateConflict)
    ));
    let item = store.pending_outbox_item("message-1").unwrap().unwrap();
    assert_eq!(item.wire, b"opaque-signed-ciphertext");
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert!(store.state(pin).unwrap().is_some());

    store
        .replace_outbox_wire(
            "message-1",
            b"sealed-again".to_vec(),
            vec![state(1, b"next")],
            vec![],
            vec![(pin.into(), 1)],
        )
        .unwrap();
    drop(store);
    let mut store = open(&root);
    let item = store.pending_outbox_item("message-1").unwrap().unwrap();
    assert_eq!(
        (item.wire.as_slice(), &item.message),
        (&b"sealed-again"[..], &saved)
    );
    assert_eq!(
        store
            .state("mls/conversation-alice-bob")
            .unwrap()
            .unwrap()
            .revision,
        2
    );
    assert!(store.state(pin).unwrap().is_none());
    assert_eq!(store.operation_message("send-1").unwrap(), Some(saved));

    // Only a message still waiting is sealed again.
    store.acknowledge_outbox("message-1").unwrap();
    assert!(matches!(
        store.replace_outbox_wire("message-1", b"late".to_vec(), vec![], vec![], vec![]),
        Err(StoreError::StateConflict)
    ));
    assert!(matches!(
        store.replace_outbox_wire("message-1", vec![], vec![], vec![], vec![]),
        Err(StoreError::InvalidInput)
    ));
}

/// Records written under `namespace`, each its key prefixed by `v:`.
fn put_records(store: &mut ProfileStore, namespace: &str, revision: u64, keys: &[&[u8]]) {
    store
        .commit_states_with_records(
            vec![StateChange {
                namespace: namespace.into(),
                expected_revision: revision,
                bytes: b"prefix".to_vec(),
            }],
            vec![StateRecordBatch {
                namespace: namespace.into(),
                changes: keys
                    .iter()
                    .map(|key| StateRecordChange {
                        key: key.to_vec(),
                        bytes: Some([b"v:", *key].concat()),
                    })
                    .collect(),
            }],
        )
        .unwrap();
}

fn keys_in(store: &ProfileStore, namespace: &str, prefixes: &[&[u8]]) -> Vec<Vec<u8>> {
    let prefixes: Vec<Vec<u8>> = prefixes.iter().map(|p| p.to_vec()).collect();
    let read = store
        .state_records_in(namespace, &prefixes)
        .unwrap()
        .unwrap();
    assert!(
        read.records
            .iter()
            .all(|(key, bytes)| *bytes == [b"v:", key.as_slice()].concat())
    );
    read.records.into_iter().map(|(key, _)| key).collect()
}

#[test]
fn a_scoped_read_returns_the_records_under_its_prefixes_and_the_parent() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let ns = "mls";
    put_records(
        &mut store,
        ns,
        0,
        &[
            b"Psk{1}",
            b"PSK{1}",
            b"Tree{\"a\"}",
            b"Tree{\"a\"}\x00\x01",
            b"tree{\"a\"}",
            b"Tree{\"b\"}",
            b"Tree{\"a\xff",
            b"Tree{\"a\xff\xff\x01",
            b"TreeX",
            b"Q%_1",
            b"Q%_2",
            b"QX_1",
            b"Q[{\"g\"},1]",
            b"Q[{\"h\"},1]",
            b"\xff\xff",
            b"\xff\xff\x01",
            b"Zz",
        ],
    );
    let read = store
        .state_records_in(ns, &[b"Psk".to_vec()])
        .unwrap()
        .unwrap();
    assert_eq!(
        (read.state.revision, read.state.bytes.as_slice()),
        (1, &b"prefix"[..])
    );
    // Sorted, each once however the prefixes overlap; bytes compared as
    // bytes, not as a pattern or case-blind.
    assert_eq!(
        keys_in(
            &store,
            ns,
            &[b"Tree{\"a\"", b"Psk", b"Tree{\"a\"}", b"Tree{\"a\""]
        ),
        vec![
            b"Psk{1}".to_vec(),
            b"Tree{\"a\"}".to_vec(),
            b"Tree{\"a\"}\x00\x01".to_vec()
        ]
    );
    assert_eq!(
        keys_in(&store, ns, &[b"Q%_"]),
        vec![b"Q%_1".to_vec(), b"Q%_2".to_vec()]
    );
    assert_eq!(
        keys_in(&store, ns, &[b"Q[{\"g\"}"]),
        vec![b"Q[{\"g\"},1]".to_vec()]
    );
    // A prefix that ends in 0xff reads what begins with it and no further;
    // one of 0xff alone has no end.
    assert_eq!(
        keys_in(&store, ns, &[b"Tree{\"a\xff"]),
        vec![b"Tree{\"a\xff".to_vec(), b"Tree{\"a\xff\xff\x01".to_vec()]
    );
    assert_eq!(
        keys_in(&store, ns, &[b"\xff\xff"]),
        vec![b"\xff\xff".to_vec(), b"\xff\xff\x01".to_vec()]
    );
    assert!(keys_in(&store, ns, &[b"Nothing"]).is_empty());
    // A state kept inline, without records: its parent, no records.
    store.commit_states(vec![state(0, b"inline")]).unwrap();
    let inline = store
        .state_records_in("mls/conversation-alice-bob", &[b"Tree".to_vec()])
        .unwrap()
        .unwrap();
    assert_eq!(
        (inline.state.bytes.as_slice(), inline.records.len()),
        (&b"inline"[..], 0)
    );
    assert!(
        store
            .state_records_in("missing", &[b"Tree".to_vec()])
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        store.state_records_in(ns, &[]),
        Err(StoreError::InvalidInput)
    ));
    assert!(matches!(
        store.state_records_in(ns, &[vec![]]),
        Err(StoreError::InvalidInput)
    ));
}

#[test]
fn a_scoped_read_is_bounded_by_what_it_selects_not_the_whole_state() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let big = vec![0x5a; 17 * 1024 * 1024];
    for (revision, key) in [(0, b"A-big"), (1, b"B-big")] {
        store
            .commit_states_with_records(
                vec![StateChange {
                    namespace: "mls".into(),
                    expected_revision: revision,
                    bytes: b"prefix".to_vec(),
                }],
                vec![StateRecordBatch {
                    namespace: "mls".into(),
                    changes: vec![StateRecordChange {
                        key: key.to_vec(),
                        bytes: Some(big.clone()),
                    }],
                }],
            )
            .unwrap();
    }
    assert!(
        store.state_records("mls").is_err(),
        "the whole state is over the bound"
    );
    let one = store
        .state_records_in("mls", &[b"A".to_vec()])
        .unwrap()
        .unwrap();
    assert_eq!(one.records.len(), 1);
    assert!(matches!(
        store.state_records_in("mls", &[b"A".to_vec(), b"B".to_vec()]),
        Err(StoreError::InvalidProfile)
    ));
}
