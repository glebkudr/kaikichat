use super::*;
use agentic_core::ProvisionRuntimeRequest;

fn intent(group: &str, operation: &str) -> ProvisionRuntimeRequest {
    ProvisionRuntimeRequest {
        operation_id: operation.into(),
        name: "Writing assistant".into(),
        agent_id: [71; 32],
        service_id: [72; 32],
        conversation_ids: vec![group.into()],
        actions: BTreeSet::from([Action::ReadInbox, Action::SendMessage]),
        expires_at: NOW + 3600,
        max_data_bytes: 4096,
    }
}

#[test]
fn provisioned_runtime_signs_actual_scoped_send_and_exactly_resumes_after_reopen() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let group = connect(&mut alice, &mut bob);
    let private = connect(&mut alice, &mut carol);
    let request = intent(&group, "setup-writing");
    let setup = alice.provision_runtime(request.clone(), NOW).unwrap();
    let key = SigningKey::from_bytes(&setup.signing_seed);
    let listed = serde_json::to_value(alice.list_runtimes(NOW).unwrap()).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(
        listed[0]["principal"],
        json!(key.verifying_key().to_bytes())
    );
    assert_eq!(listed[0]["grantId"], json!(setup.grant.grant_id));
    assert_eq!(listed[0]["name"], request.name);
    assert_eq!(listed[0]["agentId"], json!(request.agent_id));
    assert_eq!(listed[0]["serviceId"], json!(request.service_id));
    assert_eq!(listed[0]["conversationIds"], json!([group]));
    assert_eq!(listed[0]["actions"], json!(["read_inbox", "send_message"]));
    assert_eq!(listed[0]["expiresAt"], NOW + 3600);
    assert_eq!(listed[0]["maxDataBytes"], 4096);
    assert_eq!(listed[0]["status"], "active");
    assert_eq!(listed[0].as_object().unwrap().len(), 10);
    let before = persisted_state(&ar);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let retried = alice.provision_runtime(request, NOW + 10).unwrap();
    assert_eq!(*retried.signing_seed, *setup.signing_seed);
    assert_eq!(retried.grant.wire, setup.grant.wire);
    assert_eq!(retried.ownership_epoch, setup.ownership_epoch);
    assert_eq!(persisted_state(&ar), before);
    assert!(alice.agent_call(&agent_proof(&key, &setup.grant, "send_message", json!({"conversationId":private,"text":"No permission","operationId":"forbidden"}), 1), NOW).is_err());
    let sent = alice.agent_call(&agent_proof(&key, &setup.grant, "send_message", json!({"conversationId":group,"text":"Provisioned key works","operationId":"actual-send"}), 2), NOW).unwrap();
    let work = alice
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|w| w.message_id == sent["id"].as_str().unwrap())
        .unwrap();
    bob.receive(&work.wire, NOW).unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Provisioned key works"
    );
    assert!(
        carol.snapshot().unwrap().conversations[0]
            .messages
            .is_empty()
    );
    let seed = &*setup.signing_seed;
    for name in ["profile.db", "profile.db-wal"] {
        if let Ok(bytes) = std::fs::read(ar.path().join(name)) {
            assert!(!bytes.windows(32).any(|w| w == seed));
            assert!(!bytes.windows(64).any(|w| w == hex::encode(seed).as_bytes()));
        }
    }
}

#[test]
fn provisioning_rejects_changed_intent_and_cannot_resurrect_revoked_or_expired_keys() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let request = intent(&group, "first-runtime");
    let first = alice.provision_runtime(request.clone(), NOW).unwrap();
    let mut changed = request.clone();
    changed.actions.remove(&Action::SendMessage);
    let before = persisted_state(&ar);
    assert!(alice.provision_runtime(changed, NOW).is_err());
    assert_eq!(persisted_state(&ar), before);
    let second_request = intent(&group, "second-runtime");
    let second = alice
        .provision_runtime(second_request.clone(), NOW)
        .unwrap();
    assert_ne!(*first.signing_seed, *second.signing_seed);
    assert_ne!(first.grant.grant_id, second.grant.grant_id);
    alice.revoke_runtime(first.grant.grant_id, NOW).unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let before = persisted_state(&ar);
    assert!(alice.provision_runtime(request, NOW + 1).is_err());
    assert!(alice.provision_runtime(second_request, NOW + 3600).is_err());
    assert_eq!(persisted_state(&ar), before);
    let listed = serde_json::to_value(alice.list_runtimes(NOW + 3600).unwrap()).unwrap();
    let first_info = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["grantId"] == json!(first.grant.grant_id))
        .unwrap();
    let second_info = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["grantId"] == json!(second.grant.grant_id))
        .unwrap();
    assert_eq!(first_info["status"], "revoked");
    assert_eq!(second_info["status"], "expired");
    let key = SigningKey::from_bytes(&first.signing_seed);
    assert!(
        alice
            .agent_call(
                &agent_proof(&key, &first.grant, "snapshot", json!({}), 5),
                NOW
            )
            .is_err()
    );
    assert!(
        alice
            .agent_call(
                &agent_proof(
                    &SigningKey::from_bytes(&second.signing_seed),
                    &second.grant,
                    "list_runtimes",
                    json!({}),
                    6
                ),
                NOW
            )
            .is_err()
    );
}

#[test]
fn failed_secret_record_write_rolls_back_grant_and_retry_provisions_one_usable_key() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_provision BEFORE INSERT ON states WHEN NEW.namespace LIKE 'authorization/provision/%' BEGIN SELECT RAISE(ABORT,'disk full'); END;",hex::encode(KEY))).unwrap();
    let before = persisted_state(&ar);
    assert!(
        alice
            .provision_runtime(intent(&group, "disk-retry"), NOW)
            .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    assert!(alice.list_runtimes(NOW).unwrap().is_empty());
    db.execute_batch("DROP TRIGGER fail_provision").unwrap();
    drop(db);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let setup = alice
        .provision_runtime(intent(&group, "disk-retry"), NOW)
        .unwrap();
    assert_eq!(alice.list_runtimes(NOW).unwrap().len(), 1);
    let key = SigningKey::from_bytes(&setup.signing_seed);
    let snapshot = alice
        .agent_call(
            &agent_proof(&key, &setup.grant, "snapshot", json!({}), 1),
            NOW,
        )
        .unwrap();
    assert_eq!(snapshot["conversations"].as_array().unwrap().len(), 1);
    let before = persisted_state(&ar);
    for invalid in [
        {
            let mut r = intent(&group, "invalid-action");
            r.actions.insert(Action::SpendPostage);
            r
        },
        intent("00", "unknown-conversation"),
        intent(&group, ""),
    ] {
        assert!(alice.provision_runtime(invalid, NOW).is_err());
        assert_eq!(persisted_state(&ar), before);
    }
}
