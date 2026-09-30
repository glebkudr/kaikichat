use super::agent_inbox::{call_at, outbox_state};
use super::*;

fn status(group: &str, operation: &str, id: &serde_json::Value, phase: &str) -> serde_json::Value {
    json!({"conversationId":group,"operationId":operation,"messageId":id,"delivery":{"phase":phase,"replicas":0,"target":10}})
}

fn business_state(root: &TempDir) -> (Vec<(String, i64, Vec<u8>)>, i64) {
    let (mut states, operations) = persisted_state(root);
    states.retain(|s| {
        !matches!(
            s.0.as_str(),
            "authorization/registry" | "authorization/nonces"
        )
    });
    (states, operations)
}

#[test]
fn agent_delivery_status_is_bound_to_runtime_operation_and_tracks_actual_receipt_across_reopen() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let group = connect(&mut alice, &mut bob);
    let other = connect(&mut alice, &mut carol);
    let k1 = runtime_key(111);
    let k2 = runtime_key(112);
    let reader = runtime_key(113);
    let mut request = runtime_request(&k1, &group, &[Action::SendMessage]);
    request.conversation_ids.push(other.clone());
    let g1 = alice.grant_runtime(request, NOW).unwrap();
    let g2 = grant_runtime(&mut alice, &k2, &group, &[Action::SendMessage]);
    let gr = grant_runtime(&mut alice, &reader, &group, &[Action::ReadInbox]);
    alice
        .send_message(&other, "Owner-only outgoing body", "owner-op", NOW)
        .unwrap();
    let first=alice.agent_call(&agent_proof(&k1,&g1,"send_message",json!({"conversationId":group,"text":"Runtime one private body","operationId":"tracked"}),1),NOW).unwrap();
    let second=alice.agent_call(&agent_proof(&k2,&g2,"send_message",json!({"conversationId":group,"text":"Runtime two private body","operationId":"tracked"}),1),NOW).unwrap();
    assert_ne!(first["id"], second["id"]);
    let input = json!({"conversationId":group,"operationId":"tracked"});
    let owner = alice.snapshot().unwrap();
    let outbox = outbox_state(&alice);
    let business_before = business_state(&ar);
    let wire = agent_proof(&k1, &g1, "delivery_get", input.clone(), 2);
    assert_eq!(
        alice.agent_call(&wire, NOW).unwrap(),
        status(&group, "tracked", &first["id"], "queued")
    );
    assert_eq!(
        alice
            .agent_call(
                &agent_proof(&k2, &g2, "delivery_get", input.clone(), 2),
                NOW
            )
            .unwrap(),
        status(&group, "tracked", &second["id"], "queued")
    );
    assert_eq!(
        business_state(&ar),
        business_before,
        "status reads cannot debit or advance MLS/application state"
    );
    let before = persisted_state(&ar);
    for bad in [
        wire,
        agent_proof(&reader, &gr, "delivery_get", input.clone(), 3),
        agent_proof(&runtime_key(114), &g1, "delivery_get", input.clone(), 3),
        agent_proof(
            &k1,
            &g1,
            "delivery_get",
            json!({"conversationId":other,"operationId":"tracked"}),
            4,
        ),
        agent_proof(
            &k1,
            &g1,
            "delivery_get",
            json!({"conversationId":other,"operationId":"owner-op"}),
            5,
        ),
        agent_proof(
            &k1,
            &g1,
            "delivery_get",
            json!({"conversationId":group,"operationId":"unknown"}),
            6,
        ),
        agent_proof(
            &k1,
            &g1,
            "delivery_get",
            json!({"conversationId":group,"operationId":"tracked","principal":"owner"}),
            7,
        ),
    ] {
        assert!(alice.agent_call(&bad, NOW).is_err());
        assert_eq!(persisted_state(&ar), before);
    }
    assert_eq!(alice.snapshot().unwrap(), owner);
    assert_eq!(outbox_state(&alice), outbox);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(
        call_at(
            &mut alice,
            &k1,
            &g1,
            "delivery_get",
            input.clone(),
            8,
            NOW + 1
        )
        .unwrap(),
        status(&group, "tracked", &first["id"], "queued")
    );
    let work = alice
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|w| w.message_id == first["id"].as_str().unwrap())
        .unwrap();
    let ack = bob.receive(&work.wire, NOW + 1).unwrap().reply.unwrap();
    alice.receive(&ack, NOW + 1).unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Runtime one private body"
    );
    assert_eq!(
        call_at(
            &mut alice,
            &k1,
            &g1,
            "delivery_get",
            input.clone(),
            9,
            NOW + 1
        )
        .unwrap(),
        status(&group, "tracked", &first["id"], "delivered")
    );
    assert_eq!(
        call_at(
            &mut alice,
            &k2,
            &g2,
            "delivery_get",
            input.clone(),
            9,
            NOW + 1
        )
        .unwrap(),
        status(&group, "tracked", &second["id"], "queued")
    );
    alice.revoke_runtime(g1.grant_id, NOW + 1).unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let before = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &k1,
            &g1,
            "delivery_get",
            input.clone(),
            10,
            NOW + 2
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    let downgraded = alice
        .grant_runtime(runtime_request(&k1, &group, &[Action::ReadInbox]), NOW + 2)
        .unwrap();
    let context = call_at(
        &mut alice,
        &k1,
        &downgraded,
        "runtime_context",
        json!({}),
        11,
        NOW + 2,
    )
    .unwrap();
    assert_eq!(context["actions"], json!(["read_inbox"]));
    assert_eq!(
        context["principal"],
        hex::encode(k1.verifying_key().to_bytes())
    );
    // The operation exists for this principal, but its current live grant no longer permits send.
    let before = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &k1,
            &downgraded,
            "delivery_get",
            input,
            12,
            NOW + 2
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
}

#[test]
fn agent_delivery_lookup_commits_nonce_before_result_and_storage_failure_keeps_proof_retryable() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let key = runtime_key(115);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::SendMessage]);
    let sent=alice.agent_call(&agent_proof(&key,&grant,"send_message",json!({"conversationId":group,"text":"Pending before failed status read","operationId":"tracked"}),1),NOW).unwrap();
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_delivery_read BEFORE UPDATE ON states WHEN NEW.namespace='authorization/nonces' BEGIN SELECT RAISE(ABORT,'disk full'); END;",hex::encode(KEY))).unwrap();
    let before = persisted_state(&ar);
    let outbox = outbox_state(&alice);
    let owner = alice.snapshot().unwrap();
    let wire = agent_proof(
        &key,
        &grant,
        "delivery_get",
        json!({"conversationId":group,"operationId":"tracked"}),
        2,
    );
    assert!(matches!(
        alice.agent_call(&wire, NOW),
        Err(agentic_core::CoreError::Store(_))
    ));
    assert_eq!(persisted_state(&ar), before);
    assert_eq!(alice.snapshot().unwrap(), owner);
    assert_eq!(outbox_state(&alice), outbox);
    db.execute_batch("DROP TRIGGER fail_delivery_read").unwrap();
    assert_eq!(
        alice.agent_call(&wire, NOW).unwrap(),
        status(&group, "tracked", &sent["id"], "queued")
    );
    assert!(alice.agent_call(&wire, NOW).is_err());
}
