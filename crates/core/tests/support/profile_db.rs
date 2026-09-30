//! Profile helpers shared by the storage-boundary tests.
use super::*;
use agentic_crypto::MlsClient;
pub(super) fn encrypted_db(root: &TempDir) -> rusqlite::Connection {
    let db = rusqlite::Connection::open(root.path().join("profile.db")).unwrap();
    db.pragma_update(None, "key", format!("x'{}'", hex::encode(KEY)))
        .unwrap();
    db
}
pub(super) fn mls(root: &TempDir) -> MlsClient {
    MlsClient::restore(&mls_logical_bytes(root)).unwrap()
}
pub(super) fn message_queue(core: &AppCore) -> Vec<(String, Vec<u8>)> {
    core.outbox(100)
        .unwrap()
        .into_iter()
        .map(|p| (p.message_id, p.wire))
        .collect()
}
