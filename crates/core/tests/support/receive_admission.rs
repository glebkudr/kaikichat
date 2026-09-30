use super::profile_db::{encrypted_db, mls};
use super::*;
use agentic_core::CoreError;
use agentic_crypto::CryptoError;

pub(super) fn application(root: &TempDir) -> serde_json::Value {
    let bytes: Vec<u8> = encrypted_db(root)
        .query_row(
            "SELECT bytes FROM states WHERE namespace='application'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
fn write_application(root: &TempDir, value: &serde_json::Value) {
    encrypted_db(root)
        .execute(
            "UPDATE states SET bytes=?1 WHERE namespace='application'",
            [serde_json::to_vec(value).unwrap()],
        )
        .unwrap();
}
pub(super) fn legacy_profile(root: &TempDir) {
    let mut value = application(root);
    value["version"] = json!(1);
    for contact in value["contacts"].as_object_mut().unwrap().values_mut() {
        contact.as_object_mut().unwrap().remove("receive_order");
    }
    write_application(root, &value);
}
fn wire(sender: &mut AppCore, group: &str, n: u8) -> Vec<u8> {
    let message = sender
        .send_message(
            group,
            &format!("Original {n}"),
            &format!("receive-admission-{n}"),
            NOW,
        )
        .unwrap();
    sender
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|p| p.message_id == message.id)
        .unwrap()
        .wire
}
fn gap(core: &mut AppCore, root: &TempDir, bytes: &[u8]) {
    let state = persisted_state(root);
    let snapshot = core.snapshot().unwrap();
    let crypto = mls(root).snapshot().as_bytes().to_vec();
    assert!(matches!(
        core.receive(bytes, NOW),
        Err(CoreError::Crypto(CryptoError::ReceiveGap))
    ));
    assert_eq!(persisted_state(root), state);
    assert_eq!(core.snapshot().unwrap(), snapshot);
    assert_eq!(mls(root).snapshot().as_bytes(), crypto);
}

#[test]
fn both_initial_roles_defer_direct_future_messages_after_cold_restart_without_ack() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    for root in [&ar, &br] {
        let saved = application(root);
        assert_eq!(
            saved["version"], 2,
            "old readers must reject the new policy"
        );
        assert_eq!(saved["contacts"][&group]["receive_order"], "contiguous");
    }
    let a = [wire(&mut alice, &group, 0), wire(&mut alice, &group, 1)];
    let b = [wire(&mut bob, &group, 0), wire(&mut bob, &group, 1)];
    let aq = alice.outbox(100).unwrap().len();
    let bq = bob.outbox(100).unwrap().len();
    drop(alice);
    drop(bob);
    let mut alice = core(&ar, DOMAIN);
    let mut bob = core(&br, DOMAIN);
    gap(&mut bob, &br, &a[1]);
    gap(&mut alice, &ar, &b[1]);
    assert_eq!(alice.outbox(100).unwrap().len(), aq);
    assert_eq!(bob.outbox(100).unwrap().len(), bq);
    for bytes in &a {
        let ack = bob.receive(bytes, NOW).unwrap().reply.unwrap();
        alice.receive(&ack, NOW).unwrap();
    }
    for bytes in &b {
        let ack = alice.receive(bytes, NOW).unwrap().reply.unwrap();
        bob.receive(&ack, NOW).unwrap();
    }
    let state = persisted_state(&br);
    let crypto = mls(&br).snapshot().as_bytes().to_vec();
    assert!(bob.receive(&a[1], NOW).unwrap().reply.is_some());
    assert_eq!(persisted_state(&br), state);
    assert_eq!(mls(&br).snapshot().as_bytes(), crypto);
    for who in [&alice, &bob] {
        let s = who.snapshot().unwrap();
        let c = s.conversations.iter().find(|c| c.id == group).unwrap();
        assert_eq!(c.messages.len(), 4);
        assert!(
            c.messages
                .iter()
                .filter(|m| m.own)
                .all(|m| m.delivery.phase == "delivered")
        );
        assert_eq!(
            c.messages
                .iter()
                .filter(|m| !m.own)
                .map(|m| m.text.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["Original 0", "Original 1"])
        );
    }
}

#[test]
fn legacy_read_is_unchanged_and_new_contact_policy_migration_is_atomic_and_explicit() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let old = connect(&mut alice, &mut bob);
    let first = wire(&mut bob, &old, 0);
    let second = wire(&mut bob, &old, 1);
    legacy_profile(&ar);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let before = persisted_state(&ar);
    alice.snapshot().unwrap();
    assert_eq!(persisted_state(&ar), before);
    alice.receive(&second, NOW).unwrap(); // genuine skipped key in the legacy ratchet
    let invite = carol.create_invitation(NOW, vec![]).unwrap();
    let before = persisted_state(&ar);
    let view = alice.snapshot().unwrap();
    let crypto = mls(&ar).snapshot().as_bytes().to_vec();
    let db = encrypted_db(&ar);
    db.execute_batch("CREATE TRIGGER fail_policy BEFORE UPDATE ON states WHEN NEW.namespace='application' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    assert!(alice.add_contact("Carol", &invite, NOW).is_err());
    assert_eq!(persisted_state(&ar), before);
    assert_eq!(alice.snapshot().unwrap(), view);
    assert_eq!(mls(&ar).snapshot().as_bytes(), crypto);
    assert_eq!(application(&ar)["version"], 1);
    db.execute_batch("DROP TRIGGER fail_policy;").unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let fresh = alice.add_contact("Carol", &invite, NOW).unwrap().id;
    let welcome = alice
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|p| p.destination == carol.snapshot().unwrap().identity.unwrap().network_id)
        .unwrap()
        .wire;
    let ack = carol.receive(&welcome, NOW).unwrap().reply.unwrap();
    alice.receive(&ack, NOW).unwrap();
    let saved = application(&ar);
    assert_eq!(saved["version"], 2);
    assert_eq!(saved["contacts"][&old]["receive_order"], "legacy_bounded");
    assert_eq!(saved["contacts"][&fresh]["receive_order"], "contiguous");
    alice.receive(&first, NOW).unwrap();
    let next = [wire(&mut bob, &old, 2), wire(&mut bob, &old, 3)];
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    alice.receive(&next[1], NOW).unwrap();
    alice.receive(&next[0], NOW).unwrap();
    let c = [wire(&mut carol, &fresh, 0), wire(&mut carol, &fresh, 1)];
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    gap(&mut alice, &ar, &c[1]);
    alice.receive(&c[0], NOW).unwrap();
    alice.receive(&c[1], NOW).unwrap();
    let s = alice.snapshot().unwrap();
    for (id, count) in [(&old, 4), (&fresh, 2)] {
        assert_eq!(
            s.conversations
                .iter()
                .find(|c| &c.id == id)
                .unwrap()
                .messages
                .len(),
            count
        );
    }
}

#[test]
fn missing_or_unknown_v2_policy_cannot_silently_fall_back_to_legacy_receive() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let original = application(&br);
    drop(bob);
    for missing in [true, false] {
        let mut bad = original.clone();
        bad["version"] = json!(2);
        if missing {
            bad["contacts"][&group]
                .as_object_mut()
                .unwrap()
                .remove("receive_order");
        } else {
            bad["contacts"][&group]["receive_order"] = json!("unknown_policy");
        }
        write_application(&br, &bad);
        let state = persisted_state(&br);
        assert!(
            AppCore::new(
                ProfileStore::open(br.path().join("profile.db"), &KEY).unwrap(),
                DOMAIN
            )
            .is_err()
        );
        assert_eq!(persisted_state(&br), state);
    }
    write_application(&br, &original);
    let bob = core(&br, DOMAIN);
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
}
