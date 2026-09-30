use super::*;
use agent_inbox::{incoming, outbox_state};

fn ids(messages: &[agentic_core::Message]) -> Vec<String> {
    messages.iter().map(|m| m.id.clone()).collect()
}

#[test]
fn desktop_reads_latest_after_thousand_and_pages_all_original_mls_messages_across_restart() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let mut expected = Vec::new();
    for i in 0..1051 {
        let text = format!("Message {i:04} 🧭");
        let op = format!("history-{i}");
        let id = if i % 2 == 0 {
            incoming(&mut alice, &mut bob, &group, &text, &op)
        } else {
            incoming(&mut bob, &mut alice, &group, &text, &op)
        };
        expected.push((id, text, i % 2 != 0));
    }
    let queued = alice
        .send_message(&group, "Still in the queue", "queued", NOW)
        .unwrap();
    expected.push((queued.id.clone(), queued.text.clone(), true));
    let before = persisted_state(&ar);
    let outbox = outbox_state(&alice);
    let revision = alice.desktop_revision();
    let overview = alice.desktop_overview(None).unwrap();
    assert_eq!(overview.conversations.len(), 1);
    assert_eq!(overview.conversations[0].unread, 526);
    assert_eq!(
        ids(&overview.conversations[0].messages),
        vec![queued.id.clone()]
    );
    assert_eq!(
        overview.conversations[0].messages[0].delivery.phase,
        "queued"
    );
    let first = alice.conversation_history(&group, None).unwrap();
    assert_eq!(first.conversation_id, group);
    assert_eq!(first.messages.len(), 50);
    assert_eq!(
        ids(&first.messages),
        expected[1002..]
            .iter()
            .map(|e| e.0.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(first.messages.last().unwrap().delivery.phase, "queued");
    assert_eq!(persisted_state(&ar), before);
    assert_eq!(outbox_state(&alice), outbox);
    assert_eq!(alice.desktop_revision(), revision);
    let cursor = first.next_before.clone().expect("older messages remain");
    let live = incoming(
        &mut alice,
        &mut bob,
        &group,
        "Arrived while reading",
        "live",
    );
    assert!(alice.desktop_revision() > revision);
    drop(alice);
    let alice = core(&ar, DOMAIN);
    assert_eq!(
        alice
            .conversation_history(&group, None)
            .unwrap()
            .messages
            .last()
            .unwrap()
            .id,
        live
    );
    assert_eq!(
        alice.desktop_overview(None).unwrap().conversations[0].unread,
        527
    );
    let mut collected = first.messages;
    let mut next = Some(cursor);
    let mut pages = 1;
    while let Some(cursor) = next {
        let page = alice.conversation_history(&group, Some(&cursor)).unwrap();
        assert_eq!(page.conversation_id, group);
        assert!(!page.messages.is_empty());
        assert!(page.messages.len() <= 50);
        let mut older = page.messages;
        older.extend(collected);
        collected = older;
        next = page.next_before;
        pages += 1;
        assert!(pages <= 22, "cursor must make progress");
    }
    assert_eq!(collected.len(), expected.len());
    let alice_id = alice.snapshot().unwrap().identity.unwrap().network_id;
    let bob_id = bob.snapshot().unwrap().identity.unwrap().network_id;
    for (actual, (id, text, own)) in collected.iter().zip(&expected) {
        assert_eq!((&actual.id, &actual.text, actual.own), (id, text, *own));
        assert_eq!(&actual.author, if *own { &alice_id } else { &bob_id });
    }
    assert_eq!(outbox_state(&alice), outbox);
}

#[test]
fn desktop_pages_large_unicode_bodies_with_bounded_wire_size_and_full_content() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let mut expected = Vec::new();
    for i in 0..33 {
        let text = format!("{i:02} {}", "🧭".repeat(11000));
        let id = incoming(&mut alice, &mut bob, &group, &text, &format!("large-{i}"));
        expected.push((id, text));
    }
    let overview = alice.desktop_overview(None).unwrap();
    assert!(serde_json::to_vec(&overview).unwrap().len() < 4096);
    assert_eq!(overview.conversations[0].messages[0].id, expected[32].0);
    assert_eq!(
        overview.conversations[0].messages[0].text,
        format!("{}…", expected[32].1.chars().take(256).collect::<String>())
    );
    let mut all = Vec::new();
    let mut cursor = None;
    let mut pages = 0;
    loop {
        let page = alice
            .conversation_history(&group, cursor.as_deref())
            .unwrap();
        assert!(serde_json::to_vec(&page).unwrap().len() <= 512 * 1024);
        assert!(!page.messages.is_empty());
        let mut batch = page.messages;
        batch.extend(all);
        all = batch;
        pages += 1;
        assert!(pages <= 4);
        cursor = page.next_before;
        if cursor.is_none() {
            break;
        }
    }
    assert!(pages >= 3, "byte bound must split this real corpus");
    assert_eq!(
        all.iter()
            .map(|m| (m.id.clone(), m.text.clone()))
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn desktop_cursors_are_scoped_and_reads_do_not_acquire_agent_inbox_or_ack_messages() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let group = connect(&mut alice, &mut bob);
    let private = connect(&mut alice, &mut carol);
    assert!(
        alice
            .conversation_history(&group, None)
            .unwrap()
            .messages
            .is_empty()
    );
    assert!(
        alice
            .conversation_history(&group, None)
            .unwrap()
            .next_before
            .is_none()
    );
    let private_id = incoming(
        &mut alice,
        &mut carol,
        &private,
        "A private conversation",
        "private",
    );
    let id = incoming(&mut alice, &mut bob, &group, "For the agent", "allowed");
    let key = runtime_key(41);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let before = persisted_state(&ar);
    for cursor in [&private_id, "broken", &hex::encode([255; 32])] {
        assert!(alice.conversation_history(&group, Some(cursor)).is_err());
    }
    assert!(
        alice
            .conversation_history(&hex::encode([254; 32]), None)
            .is_err()
    );
    assert!(alice.desktop_overview(Some("broken")).is_err());
    assert!(
        alice
            .conversation_history(&group, Some(&id))
            .unwrap()
            .messages
            .is_empty()
    );
    assert_eq!(persisted_state(&ar), before);
    for (nonce, (method, request)) in [
        ("desktop_overview", json!({})),
        (
            "conversation_history",
            json!({"conversationId":group,"before":null}),
        ),
        ("desktop_revision", json!({})),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            alice
                .agent_call(
                    &agent_proof(&key, &grant, method, request, nonce as u8 + 1),
                    NOW
                )
                .is_err()
        );
        assert_eq!(persisted_state(&ar), before);
    }
    let page = alice
        .agent_call(
            &agent_proof(
                &key,
                &grant,
                "inbox_poll",
                agent_inbox::poll(&group, "after-desktop", 10, 4096, 30),
                9,
            ),
            NOW,
        )
        .unwrap();
    assert_eq!(page["items"][0]["id"], id);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
}

#[test]
fn desktop_overview_paginates_all_contacts_and_updates_after_delivery_without_read_side_effects() {
    let ar = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut expected = Vec::new();
    for i in 0..33 {
        let root = TempDir::new().unwrap();
        let mut peer = profile(&root, &format!("Peer {i}"));
        let group = connect(&mut alice, &mut peer);
        let revision = alice.desktop_revision();
        let message = alice
            .send_message(
                &group,
                &format!("Preview {i}"),
                &format!("preview-{i}"),
                NOW,
            )
            .unwrap();
        assert!(alice.desktop_revision() > revision);
        let revision = alice.desktop_revision();
        let work = alice
            .outbox(100)
            .unwrap()
            .into_iter()
            .find(|w| w.message_id == message.id)
            .unwrap();
        let ack = peer.receive(&work.wire, NOW).unwrap().reply.unwrap();
        alice.receive(&ack, NOW).unwrap();
        assert!(alice.desktop_revision() > revision);
        expected.push((group, format!("Preview {i}")));
    }
    expected.sort();
    let revision = alice.desktop_revision();
    let saved = persisted_state(&ar);
    let first = alice.desktop_overview(None).unwrap();
    assert_eq!(first.conversations.len(), 32);
    let second = alice.desktop_overview(first.next_after.as_deref()).unwrap();
    assert_eq!(second.conversations.len(), 1);
    assert!(second.next_after.is_none());
    let all = first
        .conversations
        .into_iter()
        .chain(second.conversations)
        .collect::<Vec<_>>();
    assert_eq!(
        all.iter()
            .map(|c| (c.id.clone(), c.messages[0].text.clone()))
            .collect::<Vec<_>>(),
        expected
    );
    for c in all {
        assert_eq!(c.messages.len(), 1);
        assert_eq!(c.messages[0].delivery.phase, "delivered");
        assert_eq!(c.unread, 0);
    }
    assert_eq!(alice.desktop_revision(), revision);
    assert_eq!(persisted_state(&ar), saved);
}
