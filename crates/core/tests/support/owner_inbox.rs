//! The owner's inbox (spec/owner-cli-v1.md): a processing cursor per
//! conversation with a lease, apart from the history and every agent's cursor.
use super::agent_inbox::{call_at, incoming, poll};
use super::*;
use agentic_core::{CoreError, OwnerInboxPage};

fn texts(page: &OwnerInboxPage) -> Vec<&str> {
    page.items.iter().map(|item| item.text.as_str()).collect()
}

fn unread(core: &AppCore, now: u64) -> Vec<(String, u64, bool)> {
    let mut found: Vec<_> = core
        .owner_inbox_unread(now)
        .unwrap()
        .into_iter()
        .map(|u| (u.conversation_id, u.unread, u.leased))
        .collect();
    found.sort();
    found
}

fn sorted(mut expected: Vec<(String, u64, bool)>) -> Vec<(String, u64, bool)> {
    expected.sort();
    expected
}

#[test]
fn the_owner_inbox_pages_incoming_messages_and_moves_only_on_ack() {
    let (ar, br, cr) = (
        TempDir::new().unwrap(),
        TempDir::new().unwrap(),
        TempDir::new().unwrap(),
    );
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let group = connect(&mut alice, &mut bob);
    let private = connect(&mut alice, &mut carol);
    let first = incoming(&mut alice, &mut bob, &group, "one", "in-1");
    alice
        .send_message(&group, "own text is not incoming", "own-1", NOW)
        .unwrap();
    incoming(&mut alice, &mut bob, &group, "two", "in-2");
    incoming(&mut alice, &mut bob, &group, "three", "in-3");
    incoming(&mut alice, &mut carol, &private, "hi", "in-4");
    assert_eq!(
        unread(&alice, NOW),
        sorted(vec![(group.clone(), 3, false), (private.clone(), 1, false)])
    );

    let page = alice.owner_inbox_poll(&group, 2, 30, NOW).unwrap();
    assert_eq!(texts(&page), ["one", "two"]);
    assert_eq!(page.items[0].id, first);
    assert!(!page.items[0].author.is_empty());
    assert!(page.has_more);
    assert_eq!(page.expires_at, Some(NOW + 30));
    let lease = page.lease_id.clone().unwrap();
    // The wire shape the CLI prints.
    let wire = serde_json::to_value(&page).unwrap();
    assert_eq!(wire["conversationId"], group);
    assert_eq!(wire["leaseId"], lease);
    assert_eq!(wire["hasMore"], true);
    assert!(wire["items"][0]["createdAt"].is_u64());

    // While the lease is active a poll returns the same page, whatever it asks.
    let again = alice.owner_inbox_poll(&group, 10, 60, NOW + 5).unwrap();
    assert_eq!(
        (again.lease_id.as_deref(), texts(&again)),
        (Some(lease.as_str()), vec!["one", "two"])
    );
    assert_eq!(
        unread(&alice, NOW + 5),
        sorted(vec![(group.clone(), 3, true), (private.clone(), 1, false)])
    );
    alice.owner_inbox_ack(&group, &lease, NOW + 6).unwrap();
    alice.owner_inbox_ack(&group, &lease, NOW + 7).unwrap();
    assert_eq!(
        unread(&alice, NOW + 7),
        sorted(vec![(group.clone(), 1, false), (private.clone(), 1, false)])
    );
    let last = alice.owner_inbox_poll(&group, 10, 30, NOW + 8).unwrap();
    assert_eq!(texts(&last), ["three"]);
    assert!(!last.has_more);
    alice
        .owner_inbox_ack(&group, last.lease_id.as_ref().unwrap(), NOW + 9)
        .unwrap();
    let empty = alice.owner_inbox_poll(&group, 10, 30, NOW + 10).unwrap();
    assert!(empty.items.is_empty() && empty.lease_id.is_none() && !empty.has_more);
    // Own messages after the cursor are passed over, not offered.
    alice
        .send_message(&group, "own again", "own-2", NOW)
        .unwrap();
    assert!(
        alice
            .owner_inbox_poll(&group, 10, 30, NOW + 11)
            .unwrap()
            .items
            .is_empty()
    );
    assert_eq!(unread(&alice, NOW + 11), vec![(private.clone(), 1, false)]);

    // An agent's cursor is its own: it still reads everything, and its ack
    // moves nothing for the owner.
    let key = runtime_key(9);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let agent_page = call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_poll",
        poll(&group, "agent-1", 10, 4096, 30),
        1,
        NOW + 12,
    )
    .unwrap();
    assert_eq!(agent_page["items"].as_array().unwrap().len(), 3);
    call_at(
        &mut alice,
        &key,
        &grant,
        "inbox_ack",
        json!({"conversationId": group, "leaseId": agent_page["leaseId"]}),
        2,
        NOW + 13,
    )
    .unwrap();
    assert_eq!(unread(&alice, NOW + 13), vec![(private.clone(), 1, false)]);

    // The cursor survives a restart.
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    incoming(&mut alice, &mut bob, &group, "four", "in-5");
    let fresh = alice.owner_inbox_poll(&group, 10, 30, NOW + 14).unwrap();
    assert_eq!(texts(&fresh), ["four"]);
}

#[test]
fn an_expired_owner_lease_is_taken_over_by_the_next_poll() {
    let (ar, br) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    incoming(&mut alice, &mut bob, &group, "one", "in-1");
    incoming(&mut alice, &mut bob, &group, "two", "in-2");
    let a = alice.owner_inbox_poll(&group, 1, 30, NOW).unwrap();
    assert_eq!(texts(&a), ["one"]);
    // Expired, the conversation waits for a poll again; until one comes, the
    // old lease is still acknowledged.
    assert_eq!(unread(&alice, NOW + 31), vec![(group.clone(), 2, false)]);
    alice
        .owner_inbox_ack(&group, a.lease_id.as_ref().unwrap(), NOW + 31)
        .unwrap();
    let b = alice.owner_inbox_poll(&group, 1, 30, NOW + 32).unwrap();
    assert_eq!(texts(&b), ["two"]);
    incoming(&mut alice, &mut bob, &group, "three", "in-3");
    // Taken over after expiry: a new lease on a fresh page, the old one refused.
    let c = alice.owner_inbox_poll(&group, 5, 30, NOW + 63).unwrap();
    assert_eq!(texts(&c), ["two", "three"]);
    assert_ne!(c.lease_id, b.lease_id);
    assert!(matches!(
        alice.owner_inbox_ack(&group, b.lease_id.as_ref().unwrap(), NOW + 64),
        Err(CoreError::InboxLeaseExpired)
    ));
    alice
        .owner_inbox_ack(&group, c.lease_id.as_ref().unwrap(), NOW + 64)
        .unwrap();
    assert!(
        alice
            .owner_inbox_poll(&group, 10, 30, NOW + 65)
            .unwrap()
            .items
            .is_empty()
    );
}

#[test]
fn the_owner_inbox_refuses_bad_bounds_unknown_conversations_and_foreign_leases() {
    let (ar, br, cr) = (
        TempDir::new().unwrap(),
        TempDir::new().unwrap(),
        TempDir::new().unwrap(),
    );
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let group = connect(&mut alice, &mut bob);
    let private = connect(&mut alice, &mut carol);
    incoming(&mut alice, &mut bob, &group, "one", "in-1");
    incoming(&mut alice, &mut carol, &private, "hi", "in-2");
    for (limit, seconds) in [(0, 30), (101, 30), (10, 0), (10, 601)] {
        assert!(
            matches!(
                alice.owner_inbox_poll(&group, limit, seconds, NOW),
                Err(CoreError::InvalidInput)
            ),
            "{limit} {seconds}"
        );
    }
    let last = group.chars().last().unwrap();
    let unknown = format!(
        "{}{}",
        &group[..group.len() - 1],
        if last == '0' { '1' } else { '0' }
    );
    assert!(matches!(
        alice.owner_inbox_poll(&unknown, 10, 30, NOW),
        Err(CoreError::UnknownConversation)
    ));
    let page = alice.owner_inbox_poll(&group, 10, 30, NOW).unwrap();
    let lease = page.lease_id.clone().unwrap();
    assert!(matches!(
        alice.owner_inbox_ack(&unknown, &lease, NOW),
        Err(CoreError::UnknownConversation)
    ));
    // A lease belongs to its conversation.
    assert!(matches!(
        alice.owner_inbox_ack(&private, &lease, NOW),
        Err(CoreError::InboxLeaseExpired)
    ));
    assert!(matches!(
        alice.owner_inbox_ack(&group, "not-a-lease", NOW),
        Err(CoreError::InvalidInput)
    ));
    assert!(matches!(
        alice.owner_inbox_ack(&group, &"ab".repeat(32), NOW),
        Err(CoreError::InboxLeaseExpired)
    ));
    // Nothing moved: the same lease, and the other conversation untouched.
    assert_eq!(
        alice
            .owner_inbox_poll(&group, 10, 30, NOW + 1)
            .unwrap()
            .lease_id,
        Some(lease)
    );
    assert_eq!(
        texts(&alice.owner_inbox_poll(&private, 10, 30, NOW + 1).unwrap()),
        ["hi"]
    );
}
