use super::*;
use serde_json::Value;

#[test]
fn dht_role_legacy_default_and_durable_toggle_preserve_real_pending_mls_and_cas() {
    let root = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    let mut alice = profile(&root, "DHT owner");
    let mut bob = profile(&other, "Recipient");
    let group = connect(&mut alice, &mut bob);
    alice
        .send_message(&group, "Queued across DHT role changes", "dht-pending", NOW)
        .unwrap();
    let before = snapshot(&alice);
    let pending = queue(&alice);
    assert_eq!(pending.len(), 1);
    drop(alice);
    // Exact previous-version data, not serialized through the new preference type.
    let bytes = serde_json::to_vec(&json!({"version":1,"preferences":{"relays":[],"relayOnly":false,"autoNatPeers":[],"bootstrapPeers":[],"lanDiscovery":false}})).unwrap();
    let mut store = ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap();
    store
        .commit_states(vec![agentic_store::StateChange {
            namespace: "network/preferences".into(),
            expected_revision: 0,
            bytes: bytes.clone(),
        }])
        .unwrap();
    let mut core = AppCore::new(store, DOMAIN).unwrap();
    let old = core.network_preferences().unwrap().unwrap();
    let mut value = serde_json::to_value(&old.preferences).unwrap();
    assert_eq!(value["dhtServer"], false);
    assert_eq!(
        core.save_network_preferences(old.preferences.clone(), 0)
            .unwrap(),
        old
    );
    drop(core);
    let store = ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap();
    let state = store.state("network/preferences").unwrap().unwrap();
    assert_eq!(state.revision, 1);
    assert_eq!(state.bytes, bytes);
    let mut core = AppCore::new(store, DOMAIN).unwrap();
    for invalid in [json!("server"), json!(1), Value::Null] {
        value["dhtServer"] = invalid;
        assert!(serde_json::from_value::<agentic_core::NetworkPreferences>(value.clone()).is_err());
        assert_eq!(core.network_preferences().unwrap().unwrap(), old);
    }
    value["dhtServer"] = json!(true);
    let desired: agentic_core::NetworkPreferences = serde_json::from_value(value).unwrap();
    let server = core.save_network_preferences(desired.clone(), 1).unwrap();
    assert_eq!(server.revision, 2);
    assert_eq!(core.save_network_preferences(desired, 1).unwrap(), server);
    assert_eq!(snapshot(&core), before);
    assert_eq!(queue(&core), pending);
    drop(core);
    let mut core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap().unwrap(), server);
    let db = rusqlite::Connection::open(root.path().join("profile.db")).unwrap();
    db.pragma_update(None, "key", format!("x'{}'", hex::encode(KEY)))
        .unwrap();
    db.execute_batch("CREATE TRIGGER fail_dht BEFORE UPDATE ON states WHEN NEW.namespace='network/preferences' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    assert!(
        core.save_network_preferences(old.preferences.clone(), 2)
            .is_err()
    );
    assert_eq!(core.network_preferences().unwrap().unwrap(), server);
    assert_eq!(queue(&core), pending);
    db.execute_batch("DROP TRIGGER fail_dht;").unwrap();
    assert!(matches!(
        core.save_network_preferences(old.preferences.clone(), 1),
        Err(agentic_core::CoreError::Store(
            agentic_store::StoreError::StateConflict
        ))
    ));
    let client = core.save_network_preferences(old.preferences, 2).unwrap();
    assert_eq!(client.revision, 3);
    drop(core);
    let mut core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap().unwrap(), client);
    assert_eq!(snapshot(&core), before);
    assert_eq!(queue(&core), pending);
    let receipt = bob.receive(&pending[0].1, NOW).unwrap().reply.unwrap();
    core.receive(&receipt, NOW).unwrap();
    assert!(core.outbox(100).unwrap().is_empty());
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Queued across DHT role changes"
    );
    assert_eq!(
        core.snapshot().unwrap().conversations[0].messages[0]
            .delivery
            .phase,
        "delivered"
    );
}
