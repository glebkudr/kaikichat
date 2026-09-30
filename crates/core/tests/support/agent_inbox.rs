use super::*;

pub(super) fn incoming(
    alice: &mut AppCore,
    bob: &mut AppCore,
    group: &str,
    text: &str,
    op: &str,
) -> String {
    let sent = bob.send_message(group, text, op, NOW).unwrap();
    let work = bob
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|w| w.message_id == sent.id)
        .unwrap();
    let receipt = alice.receive(&work.wire, NOW).unwrap().reply.unwrap();
    bob.receive(&receipt, NOW).unwrap();
    sent.id
}
pub(super) fn call_at(
    alice: &mut AppCore,
    key: &SigningKey,
    grant: &RuntimeGrant,
    method: &str,
    request: serde_json::Value,
    nonce: u8,
    now: u64,
) -> Result<serde_json::Value, agentic_core::CoreError> {
    let draft = AgentCall {
        grant_id: grant.grant_id,
        method: method.into(),
        request,
        nonce: [nonce; 32],
    }
    .draft(DOMAIN, 0, now)
    .unwrap();
    alice.agent_call(&SignedDocument::sign(draft, key).unwrap().to_wire(), now)
}
pub(super) fn poll(
    group: &str,
    operation: &str,
    limit: u64,
    bytes: u64,
    seconds: u64,
) -> serde_json::Value {
    json!({"conversationId":group,"operationId":operation,"limit":limit,"maxBytes":bytes,"leaseSeconds":seconds})
}
fn ack(group: &str, page: &serde_json::Value) -> serde_json::Value {
    json!({"conversationId":group,"leaseId":page["leaseId"]})
}
fn item_ids(page: &serde_json::Value) -> Vec<&str> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect()
}
fn inbox_states(root: &TempDir) -> Vec<(String, i64, Vec<u8>)> {
    persisted_state(root)
        .0
        .into_iter()
        .filter(|s| s.0.starts_with("authorization/inbox/"))
        .collect()
}
pub(super) fn outbox_state(core: &AppCore) -> Vec<(String, String, Vec<String>, Vec<u8>)> {
    core.outbox(1000)
        .unwrap()
        .into_iter()
        .map(|m| (m.message_id, m.destination, m.addresses, m.wire))
        .collect()
}

#[test]
fn paginated_inbox_uses_real_incoming_messages_persists_lease_and_advances_only_after_ack() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let group = connect(&mut alice, &mut bob);
    let private = connect(&mut alice, &mut carol);
    let private_id = incoming(
        &mut alice,
        &mut carol,
        &private,
        "Private other dialog",
        "private-inbox",
    );
    let first = incoming(&mut alice, &mut bob, &group, "The first", "in-1");
    alice
        .send_message(
            &group,
            "Own messages are not incoming work",
            "owner-text",
            NOW,
        )
        .unwrap();
    let second = incoming(&mut alice, &mut bob, &group, "The second", "in-2");
    let third = incoming(&mut alice, &mut bob, &group, "The third", "in-3");
    let key = runtime_key(71);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let send_key = runtime_key(77);
    let send_grant = grant_runtime(&mut alice, &send_key, &group, &[Action::SendMessage]);
    let before_denied = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&private, "private-read", 2, 4096, 30),
            20,
            NOW
        )
        .is_err()
    );
    assert!(
        call_at(
            &mut alice,
            &send_key,
            &send_grant,
            "inbox_poll",
            poll(&group, "send-only-read", 2, 4096, 30),
            1,
            NOW
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before_denied);
    let owner_before = alice.snapshot().unwrap();
    let outbox_before = outbox_state(&alice);
    let mls_before = persisted_state(&ar)
        .0
        .into_iter()
        .find(|s| s.0 == "mls")
        .unwrap();
    let request = poll(&group, "page-1", 2, 4096, 30);
    let page = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        request.clone(),
        1,
        NOW,
    )
    .unwrap();
    assert_eq!(item_ids(&page), vec![first.as_str(), second.as_str()]);
    assert_eq!(page["conversationId"], group);
    assert_eq!(page["expiresAt"], NOW + 30);
    assert_eq!(page["hasMore"], true);
    assert_eq!(page["items"][0]["text"], "The first");
    let before_foreign_ack = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_ack",
            ack(&private, &page),
            21,
            NOW
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before_foreign_ack);
    assert!(!page.to_string().contains(&private_id));
    assert!(
        !page
            .to_string()
            .contains("Own messages are not incoming work")
    );
    assert!(page["cursor"].as_u64().unwrap() > 0);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let retry = call_at(&mut alice, &key, &grant, "inbox_poll", request, 2, NOW + 1).unwrap();
    assert_eq!(
        retry, page,
        "lost poll response returns the original finite lease"
    );
    let before_busy = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "different-poll", 2, 4096, 30),
            3,
            NOW + 1
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before_busy);
    let acknowledged = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page),
        4,
        NOW + 1,
    )
    .unwrap();
    assert_eq!(acknowledged["cursor"], page["cursor"]);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let page2 = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "page-2", 2, 4096, 30),
        5,
        NOW + 2,
    )
    .unwrap();
    assert_eq!(item_ids(&page2), vec![third.as_str()]);
    assert_eq!(page2["hasMore"], false);
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page2),
        6,
        NOW + 2,
    )
    .unwrap();
    let empty = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "empty", 2, 4096, 30),
        7,
        NOW + 2,
    )
    .unwrap();
    assert!(item_ids(&empty).is_empty());
    assert!(empty["leaseId"].is_null());
    assert_eq!(empty["cursor"], page2["cursor"]);
    assert_eq!(alice.snapshot().unwrap(), owner_before);
    assert_eq!(outbox_state(&alice), outbox_before);
    assert_eq!(
        persisted_state(&ar)
            .0
            .into_iter()
            .find(|s| s.0 == "mls")
            .unwrap(),
        mls_before
    );
}

#[test]
fn sibling_runtime_cannot_take_or_ack_live_lease_but_recovers_same_messages_after_expiry() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = incoming(&mut alice, &mut bob, &group, "First work item", "work-1");
    let second = incoming(&mut alice, &mut bob, &group, "Second work item", "work-2");
    let k1 = runtime_key(72);
    let k2 = runtime_key(73);
    let g1 = grant_runtime(&mut alice, &k1, &group, &[Action::ReadInbox]);
    let g2 = grant_runtime(&mut alice, &k2, &group, &[Action::ReadInbox]);
    let page = call_at(
        &mut alice,
        &k1,
        &g1,
        "inbox_poll",
        poll(&group, "r1", 1, 4096, 10),
        1,
        NOW,
    )
    .unwrap();
    assert_eq!(item_ids(&page), vec![first.as_str()]);
    let before = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &k2,
            &g2,
            "inbox_poll",
            poll(&group, "r2", 1, 4096, 10),
            1,
            NOW + 1
        )
        .is_err()
    );
    assert!(
        call_at(
            &mut alice,
            &k2,
            &g2,
            "inbox_ack",
            ack(&group, &page),
            2,
            NOW + 1
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    assert!(
        call_at(
            &mut alice,
            &k1,
            &g1,
            "inbox_ack",
            ack(&group, &page),
            2,
            NOW + 10
        )
        .is_err(),
        "expiry applies even before another runtime replaces this lease"
    );
    assert_eq!(persisted_state(&ar), before);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let replacement = call_at(
        &mut alice,
        &k2,
        &g2,
        "inbox_poll",
        poll(&group, "r2", 1, 4096, 10),
        1,
        NOW + 11,
    )
    .unwrap();
    assert_eq!(item_ids(&replacement), vec![first.as_str()]);
    assert_ne!(replacement["leaseId"], page["leaseId"]);
    assert_eq!(replacement["expiresAt"], NOW + 21);
    let before_stale = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &k1,
            &g1,
            "inbox_ack",
            ack(&group, &page),
            2,
            NOW + 12
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before_stale);
    call_at(
        &mut alice,
        &k2,
        &g2,
        "inbox_ack",
        ack(&group, &replacement),
        2,
        NOW + 12,
    )
    .unwrap();
    let next = call_at(
        &mut alice,
        &k1,
        &g1,
        "inbox_poll",
        poll(&group, "r1-next", 1, 4096, 10),
        3,
        NOW + 12,
    )
    .unwrap();
    assert_eq!(
        item_ids(&next),
        vec![second.as_str()],
        "both runtimes share the agent/service cursor"
    );
    alice.revoke_runtime(g1.grant_id, NOW + 12).unwrap();
    let revoked = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &k1,
            &g1,
            "inbox_ack",
            ack(&group, &next),
            4,
            NOW + 12
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), revoked);
}

#[test]
fn inbox_byte_limit_counts_utf8_without_truncation_and_failed_small_page_consumes_nothing() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = incoming(&mut alice, &mut bob, &group, "äöü", "utf8-1");
    let second = incoming(&mut alice, &mut bob, &group, "öäü", "utf8-2");
    let key = runtime_key(74);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let before = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "byte-page", 10, 5, 30),
            1,
            NOW
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    let page = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "byte-page", 10, 7, 30),
        1,
        NOW,
    )
    .unwrap();
    assert_eq!(item_ids(&page), vec![first.as_str()]);
    assert_eq!(page["items"][0]["text"], "äöü");
    assert_eq!(page["hasMore"], true);
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page),
        2,
        NOW,
    )
    .unwrap();
    let page2 = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        // A thinking agent may hold a page for up to ten minutes.
        poll(&group, "byte-page-2", 10, 7, 600),
        3,
        NOW,
    )
    .unwrap();
    assert_eq!(page2["expiresAt"], NOW + 600);
    assert_eq!(item_ids(&page2), vec![second.as_str()]);
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page2),
        4,
        NOW,
    )
    .unwrap();
    let invalids = [
        poll(&group, "invalid", 0, 7, 30),
        poll(&group, "invalid", 101, 7, 30),
        poll(&group, "invalid", 1, 0, 30),
        poll(&group, "invalid", 1, 4097, 30),
        poll(&group, "invalid", 1, 7, 601),
        json!({"conversationId":group,"operationId":"invalid","limit":1,"maxBytes":7,"leaseSeconds":30,"cursor":9999}),
    ];
    let before_invalid = persisted_state(&ar);
    for (i, input) in invalids.into_iter().enumerate() {
        assert!(
            call_at(
                &mut alice,
                &key,
                &grant,
                "inbox_poll",
                input,
                10 + i as u8,
                NOW
            )
            .is_err()
        );
    }
    assert_eq!(persisted_state(&ar), before_invalid);
    let empty = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "valid-after-invalid", 1, 7, 30),
        20,
        NOW,
    )
    .unwrap();
    assert!(item_ids(&empty).is_empty());
}

#[test]
fn poll_and_ack_sql_failures_roll_back_cursor_lease_operation_and_nonce_across_reopen() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = incoming(&mut alice, &mut bob, &group, "First", "fault-1");
    let second = incoming(&mut alice, &mut bob, &group, "Second", "fault-2");
    let key = runtime_key(75);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\";", hex::encode(KEY)))
        .unwrap();
    let trigger = "CREATE TRIGGER fail_inbox BEFORE INSERT ON states WHEN NEW.namespace LIKE 'authorization/inbox/%' BEGIN SELECT RAISE(ABORT,'injected inbox failure'); END; CREATE TRIGGER fail_inbox_update BEFORE UPDATE ON states WHEN NEW.namespace LIKE 'authorization/inbox/%' BEGIN SELECT RAISE(ABORT,'injected inbox failure'); END;";
    db.execute_batch(trigger).unwrap();
    let request = poll(&group, "fault-poll", 1, 4096, 30);
    let before = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            request.clone(),
            1,
            NOW
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(persisted_state(&ar), before);
    db.execute_batch("DROP TRIGGER fail_inbox; DROP TRIGGER fail_inbox_update;")
        .unwrap();
    let page = call_at(&mut alice, &key, &grant, "inbox_poll", request, 1, NOW).unwrap();
    assert_eq!(item_ids(&page), vec![first.as_str()]);
    db.execute_batch(trigger).unwrap();
    let before_ack = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_ack",
            ack(&group, &page),
            2,
            NOW
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before_ack);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(persisted_state(&ar), before_ack);
    db.execute_batch("DROP TRIGGER fail_inbox; DROP TRIGGER fail_inbox_update;")
        .unwrap();
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page),
        2,
        NOW,
    )
    .unwrap();
    let next = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "after-fault", 1, 4096, 30),
        3,
        NOW,
    )
    .unwrap();
    assert_eq!(item_ids(&next), vec![second.as_str()]);
}

#[test]
fn changed_poll_id_is_rejected_and_duplicate_old_ack_cannot_clear_a_newer_lease() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    incoming(&mut alice, &mut bob, &group, "One", "id-1");
    let second = incoming(&mut alice, &mut bob, &group, "Two", "id-2");
    let key = runtime_key(76);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let request = poll(&group, "stable-poll-id", 1, 4096, 30);
    let page = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        request.clone(),
        1,
        NOW,
    )
    .unwrap();
    let before = persisted_state(&ar);
    assert!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "stable-poll-id", 2, 4096, 30),
            2,
            NOW
        )
        .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    let acked = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page),
        3,
        NOW,
    )
    .unwrap();
    let before_conflict = persisted_state(&ar);
    assert!(matches!(
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "stable-poll-id", 2, 4096, 30),
            2,
            NOW
        ),
        Err(agentic_core::CoreError::Store(
            agentic_store::StoreError::IdempotencyConflict
        ))
    ));
    assert_eq!(persisted_state(&ar), before_conflict);
    let next = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "new-page", 1, 4096, 30),
        4,
        NOW,
    )
    .unwrap();
    assert_eq!(item_ids(&next), vec![second.as_str()]);
    let before_duplicate = inbox_states(&ar);
    let duplicate = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page),
        5,
        NOW,
    )
    .unwrap();
    assert_eq!(duplicate, acked);
    assert_eq!(inbox_states(&ar), before_duplicate);
    let old = call_at(&mut alice, &key, &grant, "inbox_poll", request, 6, NOW).unwrap();
    assert_eq!(old, page, "historical retry must not create another lease");
    assert_eq!(inbox_states(&ar), before_duplicate);
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &next),
        7,
        NOW,
    )
    .unwrap();
    let after_final_ack = inbox_states(&ar);
    let late_duplicate = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page),
        8,
        NOW + 31,
    )
    .unwrap();
    assert_eq!(
        late_duplicate, acked,
        "a committed ack remains idempotent after expiry"
    );
    assert_eq!(inbox_states(&ar), after_final_ack);
}

#[test]
fn scan_only_page_advances_past_a_full_batch_of_owner_history_without_losing_incoming_work() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    for i in 0..1000 {
        alice
            .send_message(
                &group,
                "Sent by owner, not incoming work",
                &format!("owner-history-{i}"),
                NOW,
            )
            .unwrap();
    }
    let incoming_id = incoming(
        &mut alice,
        &mut bob,
        &group,
        "Beyond the first scanned batch",
        "after-scan",
    );
    let key = runtime_key(78);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let page = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "scan-1", 10, 4096, 30),
        1,
        NOW,
    )
    .unwrap();
    assert!(item_ids(&page).is_empty());
    assert!(page["leaseId"].as_str().is_some());
    assert_eq!(page["expiresAt"], NOW + 30);
    assert!(page["cursor"].as_u64().unwrap() >= 1000);
    assert_eq!(page["hasMore"], true);
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &page),
        2,
        NOW,
    )
    .unwrap();
    let next = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "scan-2", 10, 4096, 30),
        3,
        NOW,
    )
    .unwrap();
    assert_eq!(item_ids(&next), vec![incoming_id.as_str()]);
    assert!(next["cursor"].as_u64().unwrap() > page["cursor"].as_u64().unwrap());
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        ack(&group, &next),
        4,
        NOW,
    )
    .unwrap();
    let empty = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "scan-3", 10, 4096, 30),
        5,
        NOW,
    )
    .unwrap();
    assert!(item_ids(&empty).is_empty());
    assert!(empty["leaseId"].is_null());
}

#[test]
fn different_agents_and_services_have_independent_leases_and_acknowledged_cursors() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = incoming(
        &mut alice,
        &mut bob,
        &group,
        "Shared input",
        "independent-1",
    );
    let second = incoming(
        &mut alice,
        &mut bob,
        &group,
        "Second input",
        "independent-2",
    );
    let mut leases = BTreeSet::new();
    for (n, agent, service) in [(79, 61, 62), (80, 81, 62), (81, 61, 82)] {
        let key = runtime_key(n);
        let mut request = runtime_request(&key, &group, &[Action::ReadInbox]);
        request.agent_id = [agent; 32];
        request.service_id = [service; 32];
        let grant = alice.grant_runtime(request, NOW).unwrap();
        let page = call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "first-poll", 1, 4096, 30),
            1,
            NOW,
        )
        .unwrap();
        assert_eq!(item_ids(&page), vec![first.as_str()]);
        assert!(leases.insert(page["leaseId"].as_str().unwrap().to_owned()));
        call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_ack",
            ack(&group, &page),
            2,
            NOW,
        )
        .unwrap();
        let next = call_at(
            &mut alice,
            &key,
            &grant,
            "inbox_poll",
            poll(&group, "second-poll", 1, 4096, 30),
            3,
            NOW,
        )
        .unwrap();
        assert_eq!(item_ids(&next), vec![second.as_str()]);
        // Keep this lease live while another agent/service independently reads its first item.
    }
}
