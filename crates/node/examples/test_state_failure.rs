//! Test-image helper only. Never included in the packaged application or owner IPC API.
use sha2::{Digest, Sha256};
use std::io::Read;

fn retained(
    db: &rusqlite::Connection,
) -> rusqlite::Result<std::collections::BTreeMap<String, (u64, Vec<u8>)>> {
    let mut q = db.prepare("SELECT namespace,revision,bytes FROM states WHERE namespace<>'network/peer-records' ORDER BY namespace")?;
    q.query_map([], |row| {
        Ok((row.get(0)?, (row.get::<_, i64>(1)? as u64, row.get(2)?)))
    })?
    .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    std::io::stdin().take(4096).read_to_string(&mut input)?;
    let value: serde_json::Value = serde_json::from_str(&input)?;
    let key = value["key"].as_str().ok_or("missing key")?;
    if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid test key".into());
    }
    let path = value["path"].as_str().ok_or("missing path")?;
    let mut db = rusqlite::Connection::open(path)?;
    db.execute_batch(&format!("PRAGMA key=\"x'{key}'\";"))?;
    if value["action"] == "clear-bootstrap-cache" {
        // Acquire the ordinary profile lock too: this action only accepts a stopped test node.
        let secret = zeroize::Zeroizing::new(hex::decode(key)?);
        let _lock = agentic_store::ProfileStore::open(path, secret.as_slice().try_into()?)?;
        let before = retained(&db)?;
        let service = before
            .get("l2/service-discovery")
            .ok_or("missing saved service addresses")?;
        let digest = hex::encode(Sha256::digest(&service.1));
        let tx = db.transaction()?;
        let removed = tx.execute(
            "DELETE FROM states WHERE namespace='network/peer-records'",
            [],
        )?;
        tx.commit()?;
        if retained(&db)? != before {
            return Err("unrelated state changed".into());
        }
        let remaining: i64 = db.query_row(
            "SELECT COUNT(*) FROM states WHERE namespace='network/peer-records'",
            [],
            |row| row.get(0),
        )?;
        let mut result = serde_json::json!({"removedBootstrapStates":removed,"remainingBootstrapStates":remaining,
            "serviceCacheSha256":digest,"serviceCacheRevision":service.0,"allOtherStatesUnchanged":true});
        if value["includeServiceCache"] == true {
            result["serviceCacheBytes"] = hex::encode(&service.1).into();
        }
        println!("{result}");
        return Ok(());
    }
    if value["enabled"] == true {
        db.execute_batch("CREATE TRIGGER fail_lan_insert BEFORE INSERT ON states WHEN NEW.namespace='network/preferences' BEGIN SELECT RAISE(ABORT,'test disk full'); END; CREATE TRIGGER fail_lan_update BEFORE UPDATE ON states WHEN NEW.namespace='network/preferences' BEGIN SELECT RAISE(ABORT,'test disk full'); END;")?;
    } else {
        db.execute_batch("DROP TRIGGER fail_lan_insert; DROP TRIGGER fail_lan_update;")?;
    }
    Ok(())
}
