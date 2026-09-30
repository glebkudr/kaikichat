use super::agent_inbox::{call_at, incoming, outbox_state, poll};
use super::*;

#[test]
fn runtime_context_and_legacy_snapshot_expose_only_scoped_metadata_never_message_history() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let group = connect(&mut alice, &mut bob);
    let private = connect(&mut alice, &mut carol);
    incoming(
        &mut alice,
        &mut bob,
        &group,
        &"Secret message contents ".repeat(100),
        "large-message",
    );
    incoming(
        &mut alice,
        &mut carol,
        &private,
        "Private conversation message",
        "private-message",
    );
    let key = runtime_key(81);
    let mut request = runtime_request(&key, &group, &[Action::ReadInbox, Action::SendMessage]);
    request.max_data_bytes = 64;
    let grant = alice.grant_runtime(request, NOW).unwrap();
    let other = grant_runtime(&mut alice, &runtime_key(82), &private, &[Action::ReadInbox]);
    let owner = alice.snapshot().unwrap();
    let outbox = outbox_state(&alice);
    let expected = json!({"version":1,"networkId":owner.identity.as_ref().unwrap().network_id,"grantId":hex::encode(grant.grant_id),"agentId":hex::encode([61;32]),"serviceId":hex::encode([62;32]),"principal":hex::encode(key.verifying_key().to_bytes()),"expiresAt":NOW+3600,"maxDataBytes":64,"actions":["read_inbox","send_message"],"conversations":[{"id":group,"title":"Bob","networkId":bob.snapshot().unwrap().identity.unwrap().network_id}]});
    for (nonce, method) in [(1, "runtime_context"), (2, "snapshot")] {
        let result = alice
            .agent_call(&agent_proof(&key, &grant, method, json!({}), nonce), NOW)
            .unwrap();
        assert_eq!(
            result, expected,
            "only the documented metadata allowlist is public"
        );
        let bytes = serde_json::to_vec(&result).unwrap();
        assert!(bytes.len() <= 16 * 1024);
        assert!(!result.to_string().contains(&private));
        assert!(
            !result
                .to_string()
                .contains(&carol.snapshot().unwrap().identity.unwrap().network_id)
        );
        assert!(!result.to_string().contains(&hex::encode(other.grant_id)));
    }
    // Discovery must not consume a lease or silently bypass a grant's text bound.
    let before = persisted_state(&ar);
    assert!(matches!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "too-large", 10, 64, 30),
            3,
            NOW
        ),
        Err(agentic_core::CoreError::InboxItemTooLarge {
            required_bytes: 2400
        })
    ));
    assert_eq!(persisted_state(&ar), before);
    assert_eq!(alice.snapshot().unwrap(), owner);
    assert_eq!(outbox_state(&alice), outbox);
    assert!(
        !before
            .0
            .iter()
            .any(|s| s.0.starts_with("authorization/inbox/"))
    );
}

#[test]
fn runtime_context_supports_send_only_discovery_but_rechecks_proof_scope_expiry_and_revocation() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    incoming(
        &mut alice,
        &mut bob,
        &group,
        "Must not expose to send-only agent",
        "incoming",
    );
    let key = runtime_key(83);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::SendMessage]);
    let valid = agent_proof(&key, &grant, "runtime_context", json!({}), 1);
    let context = alice.agent_call(&valid, NOW).unwrap();
    assert_eq!(context["actions"], json!(["send_message"]));
    assert_eq!(
        context["conversations"],
        json!([{"id":group,"title":"Bob","networkId":bob.snapshot().unwrap().identity.unwrap().network_id}])
    );
    let before = persisted_state(&ar);
    for proof in [
        valid,
        agent_proof(&runtime_key(84), &grant, "runtime_context", json!({}), 2),
        agent_proof(
            &key,
            &grant,
            "runtime_context",
            json!({"principal":"owner"}),
            3,
        ),
        agent_proof(&key, &grant, "snapshot", json!({}), 4),
        agent_proof(
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "denied-read", 10, 4096, 30),
            5,
        ),
    ] {
        assert!(alice.agent_call(&proof, NOW).is_err());
        assert_eq!(persisted_state(&ar), before);
    }
    // Fresh signed proof at expiry, not merely an expired request envelope.
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "runtime_context",
            json!({}),
            6,
            NOW + 3600
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "runtime_context",
            json!({}),
            7,
            NOW + 1
        )
        .unwrap(),
        context
    );
    alice.revoke_runtime(grant.grant_id, NOW + 1).unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let before = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "runtime_context",
            json!({}),
            8,
            NOW + 2
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    assert_eq!(alice.snapshot().unwrap().conversations[0].messages.len(), 1);
}

#[test]
fn runtime_context_returns_no_result_when_nonce_commit_fails_and_same_proof_can_retry() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let key = runtime_key(85);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let first = alice
        .agent_call(
            &agent_proof(&key, &grant, "runtime_context", json!({}), 1),
            NOW,
        )
        .unwrap();
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_context BEFORE UPDATE ON states WHEN NEW.namespace='authorization/nonces' BEGIN SELECT RAISE(ABORT,'disk full'); END;",hex::encode(KEY))).unwrap();
    let before = persisted_state(&ar);
    let owner = alice.snapshot().unwrap();
    let wire = agent_proof(&key, &grant, "runtime_context", json!({}), 2);
    assert!(matches!(
        alice.agent_call(&wire, NOW),
        Err(agentic_core::CoreError::Store(_))
    ));
    assert_eq!(persisted_state(&ar), before);
    assert_eq!(alice.snapshot().unwrap(), owner);
    db.execute_batch("DROP TRIGGER fail_context").unwrap();
    assert_eq!(alice.agent_call(&wire, NOW).unwrap(), first);
    assert!(alice.agent_call(&wire, NOW).is_err());
}
