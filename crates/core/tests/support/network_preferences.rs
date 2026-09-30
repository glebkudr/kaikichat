use super::*;
use agentic_core::NetworkPreferences;
#[path = "dht_preferences.rs"]
mod dht_roles;

fn preferences() -> NetworkPreferences {
    NetworkPreferences {
        lan_discovery: true,
        dht_server: false,
        relays: vec![
            "/ip4/198.18.0.2/tcp/4001/p2p/12D3KooWCdx98FFYHgspi6EYoT4NPm5uZ5FLPMcLEh64DLFSbUJP"
                .into(),
        ],
        relay_only: true,
        auto_nat_peers: vec![
            "/ip4/198.18.0.3/tcp/4001/p2p/12D3KooWFEdt8F5ncpmnTzgDhSRJDN9xvHT3DckF9zzRAU74Cmn1"
                .into(),
        ],
        bootstrap_peers: Some(vec![
            "/ip4/198.18.0.4/tcp/4001/p2p/12D3KooWFEdt8F5ncpmnTzgDhSRJDN9xvHT3DckF9zzRAU74Cmn1"
                .into(),
        ]),
    }
}
pub(super) fn snapshot(core: &AppCore) -> serde_json::Value {
    serde_json::to_value(core.snapshot().unwrap()).unwrap()
}
pub(super) fn queue(core: &AppCore) -> Vec<(String, Vec<u8>)> {
    core.outbox(100)
        .unwrap()
        .into_iter()
        .map(|item| (item.message_id, item.wire))
        .collect()
}

#[test]
fn network_preferences_survive_reopen_without_changing_identity_history_or_pending_mls() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    alice
        .send_message(
            &group,
            "Queued across network change",
            "network-pending",
            NOW,
        )
        .unwrap();
    let before = snapshot(&alice);
    let pending = queue(&alice);
    assert_eq!(pending.len(), 1);
    assert!(alice.network_preferences().unwrap().is_none());
    let saved = alice.save_network_preferences(preferences(), 0).unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.preferences, preferences());
    assert_eq!(
        alice.save_network_preferences(preferences(), 0).unwrap(),
        saved,
        "lost response retry must not write another revision"
    );
    assert_eq!(snapshot(&alice), before);
    assert_eq!(queue(&alice), pending);
    drop(alice);
    let mut reopened = AppCore::new(
        ProfileStore::open(ar.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(reopened.network_preferences().unwrap(), Some(saved));
    assert_eq!(snapshot(&reopened), before);
    assert_eq!(queue(&reopened), pending);
    let receipt = bob.receive(&pending[0].1, NOW).unwrap().reply.unwrap();
    reopened.receive(&receipt, NOW).unwrap();
    assert!(reopened.outbox(100).unwrap().is_empty());
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Queued across network change"
    );
    assert_eq!(
        reopened.snapshot().unwrap().conversations[0].messages[0]
            .delivery
            .phase,
        "delivered"
    );
}

#[test]
fn stale_network_form_and_scoped_agent_cannot_replace_owner_preferences() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let key = runtime_key(87);
    let grant = grant_runtime(
        &mut alice,
        &key,
        &group,
        &[Action::ReadInbox, Action::SendMessage],
    );
    let first = alice.save_network_preferences(preferences(), 0).unwrap();
    let direct = NetworkPreferences {
        lan_discovery: false,
        dht_server: false,
        relays: vec![],
        relay_only: false,
        auto_nat_peers: vec![],
        bootstrap_peers: Some(vec![]),
    };
    assert!(matches!(
        alice.save_network_preferences(direct.clone(), 0),
        Err(agentic_core::CoreError::Store(
            agentic_store::StoreError::StateConflict
        ))
    ));
    assert_eq!(alice.network_preferences().unwrap(), Some(first));
    let before = snapshot(&alice);
    for (nonce, method, request) in [
        (81, "network_settings", json!({})),
        (
            82,
            "configure_network",
            json!({"expectedRevision":1,"preferences":direct}),
        ),
    ] {
        assert!(
            alice
                .agent_call(&agent_proof(&key, &grant, method, request, nonce), NOW)
                .is_err()
        );
    }
    assert_eq!(
        alice.network_preferences().unwrap().unwrap().preferences,
        preferences()
    );
    assert_eq!(snapshot(&alice), before);
    let saved = alice.save_network_preferences(direct.clone(), 1).unwrap();
    assert_eq!(saved.revision, 2);
    assert_eq!(saved.preferences, direct);
    assert_eq!(snapshot(&alice), before);
}

#[test]
fn network_preference_storage_failure_keeps_saved_config_and_original_message_retryable() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    alice
        .send_message(
            &group,
            "Do not lose this on settings failure",
            "settings-disk-failure",
            NOW,
        )
        .unwrap();
    let before = snapshot(&alice);
    let pending = queue(&alice);
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_preferences BEFORE INSERT ON states BEGIN SELECT RAISE(ABORT,'disk full'); END; CREATE TRIGGER fail_preferences_update BEFORE UPDATE ON states BEGIN SELECT RAISE(ABORT,'disk full'); END;",hex::encode(KEY))).unwrap();
    assert!(alice.save_network_preferences(preferences(), 0).is_err());
    assert!(alice.network_preferences().unwrap().is_none());
    assert_eq!(snapshot(&alice), before);
    assert_eq!(queue(&alice), pending);
    db.execute_batch("DROP TRIGGER fail_preferences; DROP TRIGGER fail_preferences_update;")
        .unwrap();
    let saved = alice.save_network_preferences(preferences(), 0).unwrap();
    assert_eq!(saved.revision, 1);
    db.execute_batch("CREATE TRIGGER fail_preferences_update BEFORE UPDATE ON states BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    let changed = NetworkPreferences {
        lan_discovery: false,
        relay_only: false,
        bootstrap_peers: Some(vec![]),
        ..preferences()
    };
    assert!(alice.save_network_preferences(changed.clone(), 1).is_err());
    assert_eq!(alice.network_preferences().unwrap(), Some(saved));
    assert_eq!(snapshot(&alice), before);
    assert_eq!(queue(&alice), pending);
    db.execute_batch("DROP TRIGGER fail_preferences_update;")
        .unwrap();
    assert_eq!(
        alice.save_network_preferences(changed, 1).unwrap().revision,
        2
    );
    let receipt = bob.receive(&pending[0].1, NOW).unwrap().reply.unwrap();
    alice.receive(&receipt, NOW).unwrap();
    assert!(alice.outbox(100).unwrap().is_empty());
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Do not lose this on settings failure"
    );
}

#[test]
fn legacy_network_preferences_load_without_rewrite_and_can_add_bootstrap_hints() {
    let root = TempDir::new().unwrap();
    let core = profile(&root, "Existing owner");
    let before = snapshot(&core);
    drop(core);
    let old = json!({"version":1,"preferences":{"relays":[],"relayOnly":false,"autoNatPeers":[]}});
    let mut store = ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap();
    let bytes = serde_json::to_vec(&old).unwrap();
    store
        .commit_states(vec![agentic_store::StateChange {
            namespace: "network/preferences".into(),
            expected_revision: 0,
            bytes: bytes.clone(),
        }])
        .unwrap();
    let mut core = AppCore::new(store, DOMAIN).unwrap();
    let saved = core.network_preferences().unwrap().unwrap();
    assert_eq!(saved.revision, 1);
    // Saved before bootstrap routes were a preference: the daemon's flags give them.
    assert_eq!(saved.preferences.bootstrap_peers, None);
    assert!(!saved.preferences.lan_discovery);
    assert_eq!(
        serde_json::to_value(&saved.preferences).unwrap(),
        json!({"relays":[],"relayOnly":false,"autoNatPeers":[],"lanDiscovery":false,"dhtServer":false})
    );
    assert_eq!(
        core.save_network_preferences(saved.preferences.clone(), 0)
            .unwrap(),
        saved
    );
    assert_eq!(snapshot(&core), before);
    drop(core);
    let store = ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap();
    let state = store.state("network/preferences").unwrap().unwrap();
    assert_eq!(state.revision, 1);
    assert_eq!(
        state.bytes, bytes,
        "loading or exact retry cannot rewrite an older profile"
    );
    let mut core = AppCore::new(store, DOMAIN).unwrap();
    let mut changed = saved.preferences;
    changed.bootstrap_peers = preferences().bootstrap_peers;
    let upgraded = core.save_network_preferences(changed.clone(), 1).unwrap();
    assert_eq!(upgraded.revision, 2);
    drop(core);
    let core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap().unwrap(), upgraded);
    assert_eq!(
        core.network_preferences().unwrap().unwrap().preferences,
        changed
    );
    assert_eq!(snapshot(&core), before);
}

#[test]
fn bootstrap_preferences_reject_excess_or_unbounded_input_before_mutating_profile() {
    let root = TempDir::new().unwrap();
    let mut core = profile(&root, "Owner");
    let first = core.save_network_preferences(preferences(), 0).unwrap();
    let before = snapshot(&core);
    let valid = preferences().bootstrap_peers.unwrap()[0].clone();
    for addresses in [
        vec![valid; 5],
        vec![String::new()],
        vec!["x".repeat(257)],
        vec!["/ip4/127.0.0.1\n/tcp/4001".into()],
    ] {
        let mut bad = preferences();
        bad.bootstrap_peers = Some(addresses);
        assert!(matches!(
            core.save_network_preferences(bad, 1),
            Err(agentic_core::CoreError::InvalidInput)
        ));
        assert_eq!(core.network_preferences().unwrap(), Some(first.clone()));
        assert_eq!(snapshot(&core), before);
    }
    drop(core);
    let core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap(), Some(first));
}

#[test]
fn lan_discovery_opt_in_alone_is_persistent_idempotent_and_can_be_disabled() {
    let root = TempDir::new().unwrap();
    let mut core = profile(&root, "LAN owner");
    let before = snapshot(&core);
    let mut wanted = preferences();
    wanted.relay_only = false;
    wanted.relays.clear();
    wanted.auto_nat_peers.clear();
    wanted.bootstrap_peers = None;
    wanted.lan_discovery = false;
    let initial = core.save_network_preferences(wanted.clone(), 0).unwrap();
    wanted.lan_discovery = true;
    let enabled = core
        .save_network_preferences(wanted.clone(), initial.revision)
        .unwrap();
    assert_eq!(enabled.revision, 2);
    assert_eq!(enabled.preferences, wanted);
    assert_eq!(
        core.save_network_preferences(wanted.clone(), 1).unwrap(),
        enabled
    );
    drop(core);
    let mut core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap().unwrap(), enabled);
    let db = rusqlite::Connection::open(root.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_lan BEFORE UPDATE ON states WHEN NEW.namespace='network/preferences' BEGIN SELECT RAISE(ABORT,'disk full'); END;", hex::encode(KEY))).unwrap();
    wanted.lan_discovery = false;
    assert!(core.save_network_preferences(wanted.clone(), 2).is_err());
    assert_eq!(core.network_preferences().unwrap().unwrap(), enabled);
    db.execute_batch("DROP TRIGGER fail_lan;").unwrap();
    assert!(matches!(
        core.save_network_preferences(wanted.clone(), 1),
        Err(agentic_core::CoreError::Store(
            agentic_store::StoreError::StateConflict
        ))
    ));
    let disabled = core.save_network_preferences(wanted.clone(), 2).unwrap();
    assert_eq!(disabled.revision, 3);
    assert_eq!(disabled.preferences, wanted);
    assert_eq!(snapshot(&core), before);
    drop(core);
    let core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap().unwrap(), disabled);
    assert_eq!(snapshot(&core), before);
}

/// Bootstrap routes the owner never named follow the daemon's `--bootstrap`
/// flags (the network preset's): such preferences are stored without the
/// key, which an older daemon on the same profile still reads, while routes
/// the owner named, none included, are kept as named.
#[test]
fn unnamed_bootstrap_routes_are_stored_apart_from_routes_the_owner_named() {
    let root = TempDir::new().unwrap();
    let mut core = profile(&root, "Owner");
    let before = snapshot(&core);
    let stored = |root: &TempDir| -> serde_json::Value {
        let store = ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap();
        serde_json::from_slice(&store.state("network/preferences").unwrap().unwrap().bytes).unwrap()
    };
    let mut network = preferences();
    network.relay_only = false;
    network.bootstrap_peers = None;
    let following = core.save_network_preferences(network.clone(), 0).unwrap();
    assert_eq!(following.preferences, network);
    drop(core);
    let saved = stored(&root);
    assert_eq!(saved["version"], 1);
    assert!(
        saved["preferences"].get("bootstrapPeers").is_none(),
        "{saved}"
    );
    let mut core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap(), Some(following.clone()));

    // No routes, named: a different preference that survives reopening.
    let mut none = network.clone();
    none.bootstrap_peers = Some(vec![]);
    let named = core.save_network_preferences(none.clone(), 1).unwrap();
    assert_eq!(named.revision, 2);
    drop(core);
    assert_eq!(stored(&root)["preferences"]["bootstrapPeers"], json!([]));
    let mut core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        DOMAIN,
    )
    .unwrap();
    assert_eq!(core.network_preferences().unwrap(), Some(named));

    // Cleared again: back to the flags.
    let cleared = core.save_network_preferences(network.clone(), 2).unwrap();
    assert_eq!((cleared.revision, &cleared.preferences), (3, &network));
    assert_eq!(snapshot(&core), before);
    drop(core);
    assert!(stored(&root)["preferences"].get("bootstrapPeers").is_none());
}
