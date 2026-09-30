use super::*;

#[test]
fn background_conversation_scan_is_bounded_and_resumes_after_restart_without_changing_messages() {
    let br = TempDir::new().unwrap();
    let mut bob = profile(&br, "Bob");
    let mut expected = Vec::new();
    for name in ["Alice", "Carol", "Dave"] {
        let root = TempDir::new().unwrap();
        let mut peer = profile(&root, name);
        let id = connect(&mut peer, &mut bob);
        bob.send_message(&id, "Unacknowledged original message", name, NOW)
            .unwrap();
        expected.push(id);
    }
    expected.sort();
    let saved = persisted_state(&br);
    let snapshot = bob.snapshot().unwrap();
    let outbox = profile_db::message_queue(&bob);
    assert_eq!(bob.conversation_ids(None, 2).unwrap(), expected[..2]);
    let mut missing = hex::decode(&expected[0]).unwrap();
    for byte in missing.iter_mut().rev() {
        let (next, overflow) = byte.overflowing_add(1);
        *byte = next;
        if !overflow {
            break;
        }
    }
    let missing = hex::encode(missing);
    assert!(expected[0] < missing && missing < expected[1]);
    assert_eq!(
        bob.conversation_ids(Some(&missing), 2).unwrap(),
        expected[1..]
    );
    assert_eq!(
        bob.conversation_ids(Some(&expected[1]), 2).unwrap(),
        expected[2..]
    );
    assert!(
        bob.conversation_ids(Some(&expected[2]), 2)
            .unwrap()
            .is_empty()
    );
    assert_eq!(bob.snapshot().unwrap(), snapshot);
    assert_eq!(profile_db::message_queue(&bob), outbox);
    assert_eq!(persisted_state(&br), saved);
    drop(bob);
    let bob = core(&br, DOMAIN);
    assert_eq!(bob.conversation_ids(None, 32).unwrap(), expected);
    assert_eq!(
        bob.conversation_ids(Some(&expected[1]), 1).unwrap(),
        expected[2..]
    );
    assert_eq!(bob.snapshot().unwrap(), snapshot);
    assert_eq!(profile_db::message_queue(&bob), outbox);
    assert_eq!(persisted_state(&br), saved);
}

#[test]
fn background_scan_refuses_unbounded_or_malformed_queries_and_handles_an_empty_profile() {
    let root = TempDir::new().unwrap();
    let c = core(&root, DOMAIN);
    let before = persisted_state(&root);
    assert!(c.conversation_ids(None, 32).unwrap().is_empty());
    for size in [0, 33, usize::MAX] {
        assert!(c.conversation_ids(None, size).is_err());
    }
    for cursor in ["", "broken", "../../../private"] {
        assert!(c.conversation_ids(Some(cursor), 2).is_err());
    }
    // A valid cursor need not name an extant conversation: paging must resume
    // after its removal, without requiring an old conversation to be recreated.
    assert!(
        c.conversation_ids(Some(&hex::encode([255; 32])), 2)
            .unwrap()
            .is_empty()
    );
    assert_eq!(persisted_state(&root), before);
}
