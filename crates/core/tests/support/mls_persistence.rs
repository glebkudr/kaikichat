//! S03: real Core send/receive, SQL write volume and rollback across cold reopen.
use super::profile_db::message_queue;
use super::*;
use agentic_crypto::{MlsClient, record_scope};
use rusqlite::Connection;

fn db(root: &TempDir) -> Connection {
    let db = Connection::open(root.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\";", hex::encode(KEY)))
        .unwrap();
    db
}

fn rows(root: &TempDir) -> BTreeMap<Vec<u8>, Vec<u8>> {
    db(root)
        .prepare(
            "SELECT record_key,bytes FROM state_records WHERE namespace='mls' ORDER BY record_key",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn head(root: &TempDir) -> (i64, Vec<u8>) {
    db(root)
        .query_row(
            "SELECT revision,bytes FROM states WHERE namespace='mls'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

type SqlSnapshot = BTreeMap<&'static str, Vec<Vec<rusqlite::types::Value>>>;

fn sql_snapshot(root: &TempDir) -> SqlSnapshot {
    let connection = db(root);
    [
        ("messages", "SELECT * FROM messages ORDER BY sequence"),
        (
            "operations",
            "SELECT * FROM operations ORDER BY operation_id",
        ),
        ("outbox", "SELECT * FROM outbox ORDER BY message_id"),
        ("states", "SELECT * FROM states ORDER BY namespace"),
        (
            "state_records",
            "SELECT * FROM state_records ORDER BY namespace,record_key",
        ),
    ]
    .into_iter()
    .map(|(table, sql)| {
        let mut query = connection.prepare(sql).unwrap();
        let columns = query.column_count();
        let rows = query
            .query_map([], |row| {
                (0..columns)
                    .map(|column| row.get::<_, rusqlite::types::Value>(column))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        (table, rows)
    })
    .collect()
}

fn assert_trigger<T>(result: Result<T, agentic_core::CoreError>, expected: &str) {
    match result {
        Err(agentic_core::CoreError::Store(agentic_store::StoreError::Database(
            rusqlite::Error::SqliteFailure(code, Some(message)),
        ))) => {
            assert_eq!(code.extended_code, rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER);
            assert_eq!(message, expected);
        }
        Err(error) => panic!("expected trigger {expected:?}, got {error:?}"),
        Ok(_) => panic!("expected trigger {expected:?} to abort the transaction"),
    }
}

fn pending_wire(core: &AppCore, message: &str) -> Vec<u8> {
    message_queue(core)
        .into_iter()
        .find(|(id, _)| id == message)
        .unwrap()
        .1
}

fn seed_history(alice: &mut AppCore, bob: &mut AppCore, group: &str) {
    let sent = alice
        .send_message(group, "existing Alice message", "s03-existing-alice", NOW)
        .unwrap();
    bob.receive(&pending_wire(alice, &sent.id), NOW).unwrap();
    let reply = bob
        .send_message(group, "existing Bob message", "s03-existing-bob", NOW)
        .unwrap();
    alice.receive(&pending_wire(bob, &reply.id), NOW).unwrap();
    // Leave both acknowledgments undelivered: both profiles have real incoming
    // and outgoing messages, operation hashes and nonempty outbox wire bytes.
    assert_eq!(message_queue(alice)[0].0, sent.id);
    assert_eq!(message_queue(bob)[0].0, reply.id);
}

fn legacy_snapshot(root: &TempDir) -> Vec<u8> {
    // The pre-S03 profile stored this exact CBOR prefix followed by each
    // provider record as one [key,value] item. Build that wire directly from
    // the encrypted tables so this fixture does not call the current
    // ProfileStore::state or MlsClient::restore adapters.
    let connection = db(root);
    let prefix: Vec<u8> = connection
        .query_row(
            "SELECT bytes FROM states WHERE namespace='mls'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        prefix.first(),
        Some(&0x84),
        "legacy MLS prefix contract changed"
    );
    assert_eq!(prefix.get(1), Some(&1), "legacy MLS version changed");
    let mut snapshot = prefix;
    let mut query = connection
        .prepare("SELECT bytes FROM state_records WHERE namespace='mls' ORDER BY record_key")
        .unwrap();
    let records = query
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for record in records {
        snapshot.extend_from_slice(&record);
    }
    snapshot
}

fn audit(root: &TempDir) {
    db(root)
        .execute_batch(
            "CREATE TABLE mls_writes(record_key BLOB NOT NULL, bytes INTEGER NOT NULL);
         CREATE TRIGGER mls_insert AFTER INSERT ON state_records WHEN NEW.namespace='mls'
         BEGIN INSERT INTO mls_writes VALUES(NEW.record_key,length(NEW.bytes)); END;
         CREATE TRIGGER mls_update AFTER UPDATE ON state_records WHEN NEW.namespace='mls'
         BEGIN INSERT INTO mls_writes VALUES(NEW.record_key,length(NEW.bytes)); END;
         CREATE TRIGGER mls_delete AFTER DELETE ON state_records WHEN OLD.namespace='mls'
         BEGIN INSERT INTO mls_writes VALUES(OLD.record_key,0); END;",
        )
        .unwrap();
}

fn assert_incremental(root: &TempDir, before: &BTreeMap<Vec<u8>, Vec<u8>>) {
    let after = rows(root);
    let changed: BTreeSet<_> = before
        .keys()
        .chain(after.keys())
        .filter(|key| before.get(*key) != after.get(*key))
        .cloned()
        .collect();
    let writes: Vec<(Vec<u8>, usize)> = db(root)
        .prepare("SELECT record_key,bytes FROM mls_writes")
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                usize::try_from(row.get::<_, i64>(1)?)
                    .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(1, i64::MAX))?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!changed.is_empty());
    assert!(
        changed.len() < before.len(),
        "rewrote the complete provider"
    );
    assert_eq!(
        writes.len(),
        changed.len(),
        "unchanged/duplicate SQL writes"
    );
    assert_eq!(
        writes
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<BTreeSet<_>>(),
        changed
    );
    let baseline = head(root).1.len() + before.values().map(Vec::len).sum::<usize>();
    let written = head(root).1.len() + writes.iter().map(|(_, bytes)| bytes).sum::<usize>();
    assert!(
        head(root).1.len() < 1024,
        "MLS head still contains the full snapshot"
    );
    assert!(
        written < baseline,
        "SQL bytes are not bounded by the changed records"
    );
    eprintln!(
        "S03 MLS snapshot_bytes={baseline} written_bytes={written} changed_rows={}",
        changed.len()
    );
}

#[test]
fn multiple_dialogs_write_only_changed_mls_records_and_retry_does_not_advance_them() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let first = connect(&mut alice, &mut bob);
    let second = connect(&mut alice, &mut carol);
    audit(&ar);
    audit(&br);
    let alice_before = rows(&ar);
    let bob_before = rows(&br);
    let sent = alice
        .send_message(&first, "first dialog", "s03-send", NOW)
        .unwrap();
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    assert_incremental(&ar, &alice_before);
    bob.receive(&wire, NOW).unwrap();
    assert_incremental(&br, &bob_before);
    let saved = sql_snapshot(&ar);
    db(&ar).execute("DELETE FROM mls_writes", []).unwrap();
    assert_eq!(
        alice
            .send_message(&first, "first dialog", "s03-send", NOW + 1)
            .unwrap()
            .id,
        sent.id
    );
    assert_eq!(sql_snapshot(&ar), saved);
    assert_eq!(
        db(&ar)
            .query_row("SELECT count(*) FROM mls_writes", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(alice);
    drop(bob);
    let mut alice = core(&ar, DOMAIN);
    let mut bob = core(&br, DOMAIN);
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "first dialog"
    );
    let sent = alice
        .send_message(&second, "second dialog", "s03-other", NOW + 2)
        .unwrap();
    let wire = alice
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|row| row.message_id == sent.id)
        .unwrap()
        .wire;
    carol.receive(&wire, NOW + 2).unwrap();
    let reply = bob
        .send_message(&first, "cold reply", "s03-reply", NOW + 3)
        .unwrap();
    let wire = bob
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|row| row.message_id == reply.id)
        .unwrap()
        .wire;
    alice.receive(&wire, NOW + 3).unwrap();
    assert!(
        alice
            .snapshot()
            .unwrap()
            .conversations
            .iter()
            .any(|c| c.messages.iter().any(|m| m.text == "cold reply"))
    );
}

#[test]
fn mls_record_deletion_is_atomic_in_core_sql_and_survives_cold_reopen() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let existing = connect(&mut bob, &mut carol);
    seed_history(&mut bob, &mut carol, &existing);
    bob.create_invitation(NOW, vec![]).unwrap(); // Unconsumed control package.
    let before_invitation = rows(&br);
    let invite = bob
        .create_invitation(NOW, vec!["/ip4/127.0.0.1/tcp/4002".into()])
        .unwrap();
    // Bind the expected deletion before join: only this real invitation call
    // minted this KeyPackage row. The other invitation must remain usable.
    let minted: Vec<_> = rows(&br)
        .into_keys()
        .filter(|key| key.starts_with(b"KeyPackage") && !before_invitation.contains_key(key))
        .collect();
    assert_eq!(minted.len(), 1);
    let consumed_key = minted[0].clone();
    let unused: BTreeMap<_, _> = before_invitation
        .into_iter()
        .filter(|(key, _)| key.starts_with(b"KeyPackage"))
        .collect();
    assert_eq!(unused.len(), 1);
    let conversation = alice.add_contact("Bob", &invite, NOW).unwrap();
    let welcome = alice.outbox(100).unwrap()[0].wire.clone();
    let before = (
        sql_snapshot(&br),
        bob.snapshot().unwrap(),
        message_queue(&bob),
    );
    db(&br)
        .execute_batch(&format!(
            "CREATE TRIGGER fail_s03 BEFORE DELETE ON state_records
             WHEN OLD.namespace='mls' AND OLD.record_key=x'{}'
             BEGIN SELECT RAISE(ABORT,'record delete failure'); END;",
            hex::encode(&consumed_key)
        ))
        .unwrap();
    assert_trigger(bob.receive(&welcome, NOW), "record delete failure");
    assert_eq!(
        (
            sql_snapshot(&br),
            bob.snapshot().unwrap(),
            message_queue(&bob)
        ),
        before
    );
    db(&br).execute_batch("DROP TRIGGER fail_s03").unwrap();

    let before_records = rows(&br);
    audit(&br);
    bob.receive(&welcome, NOW).unwrap();
    let after_records = rows(&br);
    let deleted: BTreeSet<_> = before_records
        .keys()
        .filter(|key| !after_records.contains_key(*key))
        .cloned()
        .collect();
    assert_eq!(
        deleted
            .iter()
            .filter(|key| key.starts_with(b"KeyPackage"))
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([consumed_key]),
        "join must delete the exact invitation's KeyPackage"
    );
    for (key, bytes) in unused {
        assert_eq!(after_records.get(&key), Some(&bytes));
    }
    let sql_deletes: BTreeSet<Vec<u8>> = db(&br)
        .prepare("SELECT record_key FROM mls_writes WHERE bytes=0")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(sql_deletes, deleted);
    let joined = bob.snapshot().unwrap();
    assert!(joined.conversations.iter().any(|c| c.id == conversation.id));
    assert_eq!(
        joined.conversations.iter().find(|c| c.id == existing),
        before.1.conversations.iter().find(|c| c.id == existing)
    );
    let committed = sql_snapshot(&br);

    drop(bob);
    let cold = core(&br, DOMAIN);
    assert_eq!(rows(&br), after_records);
    assert_eq!(sql_snapshot(&br), committed);
    assert_eq!(cold.snapshot().unwrap(), joined);
}

#[test]
fn late_sql_fault_rolls_back_mls_rows_messages_and_outbox_in_memory_and_cold() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    seed_history(&mut alice, &mut bob, &group);
    // Each iteration fails and retries on the SAME live instances. The second
    // iteration starts from the successful first iteration's cold-reopened SQL.
    for operation in ["s03-fault-live", "s03-fault-cold"] {
        let before = (
            sql_snapshot(&ar),
            alice.snapshot().unwrap(),
            message_queue(&alice),
        );
        assert!(before.0.values().all(|table| !table.is_empty()));
        db(&ar).execute_batch(&format!("CREATE TRIGGER fail_s03 AFTER INSERT ON operations WHEN NEW.operation_id='{operation}' BEGIN SELECT RAISE(ABORT,'late send failure'); END;")).unwrap();
        assert_trigger(
            alice.send_message(&group, "retry me", operation, NOW),
            "late send failure",
        );
        assert_eq!(
            (
                sql_snapshot(&ar),
                alice.snapshot().unwrap(),
                message_queue(&alice)
            ),
            before
        );
        db(&ar).execute_batch("DROP TRIGGER fail_s03").unwrap();
        let sent = alice
            .send_message(&group, "retry me", operation, NOW)
            .unwrap();
        let wire = pending_wire(&alice, &sent.id);
        let after_send = sql_snapshot(&ar);
        for table in ["messages", "operations", "outbox"] {
            assert!(
                before.0[table]
                    .iter()
                    .all(|row| after_send[table].contains(row))
            );
        }

        let before = (
            sql_snapshot(&br),
            bob.snapshot().unwrap(),
            message_queue(&bob),
        );
        assert!(before.0.values().all(|table| !table.is_empty()));
        db(&br).execute_batch(&format!("CREATE TRIGGER fail_s03 AFTER INSERT ON messages WHEN NEW.id='{}' BEGIN SELECT RAISE(ABORT,'late receive failure'); END;", sent.id)).unwrap();
        assert_trigger(bob.receive(&wire, NOW), "late receive failure");
        assert_eq!(
            (
                sql_snapshot(&br),
                bob.snapshot().unwrap(),
                message_queue(&bob)
            ),
            before
        );
        db(&br).execute_batch("DROP TRIGGER fail_s03").unwrap();
        bob.receive(&wire, NOW).unwrap();
        assert_eq!(
            bob.snapshot().unwrap().conversations[0]
                .messages
                .iter()
                .find(|m| m.id == sent.id)
                .unwrap()
                .text,
            "retry me"
        );
        let after_receive = sql_snapshot(&br);
        for table in ["messages", "operations", "outbox"] {
            assert!(
                before.0[table]
                    .iter()
                    .all(|row| after_receive[table].contains(row))
            );
        }
        let alice_saved = (after_send, alice.snapshot().unwrap(), message_queue(&alice));
        let bob_saved = (after_receive, bob.snapshot().unwrap(), message_queue(&bob));
        drop(alice);
        drop(bob);
        alice = core(&ar, DOMAIN);
        bob = core(&br, DOMAIN);
        assert_eq!(
            (
                sql_snapshot(&ar),
                alice.snapshot().unwrap(),
                message_queue(&alice)
            ),
            alice_saved
        );
        assert_eq!(
            (
                sql_snapshot(&br),
                bob.snapshot().unwrap(),
                message_queue(&bob)
            ),
            bob_saved
        );
    }
}

#[test]
fn legacy_mls_snapshot_migrates_with_the_message_and_remains_readable_through_state_api() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    seed_history(&mut alice, &mut bob, &group);
    drop(alice);
    let legacy = legacy_snapshot(&ar);
    let sql = db(&ar);
    sql.execute("DELETE FROM state_records WHERE namespace='mls'", [])
        .unwrap();
    sql.execute(
        "UPDATE states SET bytes=?1 WHERE namespace='mls'",
        [&legacy],
    )
    .unwrap();
    let before = head(&ar);
    let before_sql = sql_snapshot(&ar);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(head(&ar), before, "opening migrated the legacy profile");
    let before_view = alice.snapshot().unwrap();
    let before_queue = message_queue(&alice);
    sql.execute_batch("CREATE TRIGGER fail_s03 AFTER INSERT ON state_records WHEN NEW.namespace='mls' BEGIN SELECT RAISE(ABORT,'migration write failure'); END;").unwrap();
    assert_trigger(
        alice.send_message(&group, "after migration", "s03-legacy", NOW),
        "migration write failure",
    );
    assert_eq!(sql_snapshot(&ar), before_sql);
    assert_eq!(head(&ar), before);
    assert!(rows(&ar).is_empty());
    assert_eq!(alice.snapshot().unwrap(), before_view);
    assert_eq!(message_queue(&alice), before_queue);
    sql.execute_batch("DROP TRIGGER fail_s03").unwrap();
    let sent = alice
        .send_message(&group, "after migration", "s03-legacy", NOW)
        .unwrap();
    let wire = pending_wire(&alice, &sent.id);
    bob.receive(&wire, NOW).unwrap();
    assert!(!rows(&ar).is_empty());
    assert!(head(&ar).1.len() < legacy.len());
    drop(alice);
    let store = ProfileStore::open(ar.path().join("profile.db"), &KEY).unwrap();
    let snapshot = store.state("mls").unwrap().unwrap();
    MlsClient::restore(&snapshot.bytes).unwrap();
    assert_eq!(snapshot.revision as i64, before.0 + 1);
    drop(store);
    let mut alice = core(&ar, DOMAIN);
    let next = alice
        .send_message(&group, "next generation", "s03-after-cold", NOW + 1)
        .unwrap();
    bob.receive(&pending_wire(&alice, &next.id), NOW + 1)
        .unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0]
            .messages
            .iter()
            .find(|m| m.id == sent.id)
            .unwrap()
            .text,
        "after migration"
    );
    assert_eq!(
        bob.snapshot().unwrap().conversations[0]
            .messages
            .iter()
            .find(|m| m.id == next.id)
            .unwrap()
            .text,
        "next generation"
    );
}

#[test]
fn an_operation_in_one_dialog_reads_only_its_own_mls_records() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let first = connect(&mut alice, &mut bob);
    let second = connect(&mut alice, &mut carol);
    // What only the second dialog holds goes bad in Alice's database: a
    // record no restore accepts.
    let common = record_scope(None);
    let group: [u8; 32] = hex::decode(&second).unwrap().try_into().unwrap();
    let own: Vec<Vec<u8>> = record_scope(Some(group))
        .into_iter()
        .filter(|prefix| !common.contains(prefix))
        .collect();
    let kept: Vec<(Vec<u8>, Vec<u8>)> = rows(&ar)
        .into_iter()
        .filter(|(key, _)| own.iter().any(|prefix| key.starts_with(prefix)))
        .collect();
    assert!(kept.len() > 3, "{} records", kept.len());
    for (key, _) in &kept {
        db(&ar)
            .execute(
                "UPDATE state_records SET bytes=x'824040' WHERE namespace='mls' AND record_key=?1",
                [key],
            )
            .unwrap();
    }
    // The first dialog goes on without reading them.
    let sent = alice
        .send_message(&first, "first alone", "scoped-1", NOW)
        .unwrap();
    bob.receive(&pending_wire(&alice, &sent.id), NOW).unwrap();
    // The second one reads them, and changes nothing when it cannot.
    let before = sql_snapshot(&ar);
    assert!(matches!(
        alice.send_message(&second, "second", "scoped-2", NOW),
        Err(agentic_core::CoreError::Crypto(
            agentic_crypto::CryptoError::InvalidState
        ))
    ));
    assert_eq!(sql_snapshot(&ar), before);
    // Put back: the whole profile holds together, its count kept.
    for (key, bytes) in &kept {
        db(&ar)
            .execute(
                "UPDATE state_records SET bytes=?2 WHERE namespace='mls' AND record_key=?1",
                rusqlite::params![key, bytes],
            )
            .unwrap();
    }
    MlsClient::restore_records(&head(&ar).1, rows(&ar)).unwrap();
    let sent = alice
        .send_message(&second, "second", "scoped-3", NOW)
        .unwrap();
    carol.receive(&pending_wire(&alice, &sent.id), NOW).unwrap();
    assert!(
        carol
            .snapshot()
            .unwrap()
            .conversations
            .iter()
            .any(|c| c.messages.iter().any(|m| m.text == "second"))
    );
}
