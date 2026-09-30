use super::*;

fn addresses(port: u16) -> Vec<String> {
    vec![format!("/ip4/127.0.0.1/tcp/{port}/p2p/peer-B")]
}
fn database(root: &TempDir) -> rusqlite::Connection {
    let db = rusqlite::Connection::open(root.path().join("profile.db")).unwrap();
    db.pragma_update(None, "key", format!("x'{}'", hex::encode(KEY)))
        .unwrap();
    db
}
fn signer(root: &TempDir) -> SigningKey {
    let seed: Vec<u8> = database(root)
        .query_row(
            "SELECT owner_seed FROM profile WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    SigningKey::from_bytes(&seed.try_into().unwrap())
}
// A wire fixture signed by an independent root, without a second routing implementation.
fn fixture(key: &SigningKey, sequence: u64, port: u16, domain: [u8; 32], issued: u64) -> Vec<u8> {
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(4)
        .unwrap()
        .u8(2)
        .unwrap()
        .str("peer-B")
        .unwrap()
        .array(1)
        .unwrap()
        .str(&addresses(port)[0])
        .unwrap()
        .u64(sequence)
        .unwrap();
    SignedDocument::sign(
        DocumentDraft {
            domain,
            kind: DocumentKind::Identity,
            authority_epoch: 0,
            issued_at: issued,
            expires_at: Some(issued + 86400),
            body: e.into_writer(),
            extensions: BTreeMap::new(),
        },
        key,
    )
    .unwrap()
    .to_wire()
}

#[test]
fn publisher_reuses_current_record_and_orders_same_second_changes_across_restart() {
    let root = TempDir::new().unwrap();
    let mut node = profile(&root, "Bob");
    let first = node
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    let parsed = node.verify_node_record(&first, "peer-B", NOW).unwrap();
    assert_eq!(parsed.sequence, 1);
    assert_eq!(parsed.peer_id, "peer-B");
    assert_eq!(parsed.addresses, addresses(4101));
    let before = persisted_state(&root);
    assert_eq!(
        node.publish_node_record("peer-B", addresses(4101), NOW + 1)
            .unwrap(),
        first
    );
    assert_eq!(
        persisted_state(&root),
        before,
        "ordinary sends must not rewrite the record"
    );
    let second = node
        .publish_node_record("peer-B", addresses(4102), NOW)
        .unwrap();
    assert_eq!(
        node.verify_node_record(&second, "peer-B", NOW)
            .unwrap()
            .sequence,
        2
    );
    drop(node);
    let mut node = core(&root, DOMAIN);
    assert_eq!(
        node.publish_node_record("peer-B", addresses(4102), NOW + 2)
            .unwrap(),
        second
    );
    let third = node
        .publish_node_record("peer-B", addresses(4103), NOW + 2)
        .unwrap();
    assert_eq!(
        node.verify_node_record(&third, "peer-B", NOW + 2)
            .unwrap()
            .sequence,
        3
    );
    let renewal = node
        .publish_node_record("peer-B", addresses(4103), NOW + 86402)
        .unwrap();
    let renewed = node
        .verify_node_record(&renewal, "peer-B", NOW + 86402)
        .unwrap();
    assert_eq!(renewed.sequence, 4);
    assert_eq!(renewed.expires_at, NOW + 2 * 86400 + 2);
    assert!(
        node.verify_node_record(&first, "peer-B", NOW + 86400)
            .is_err()
    );
}

#[test]
fn refreshed_contact_routes_preserve_queued_ciphertext_and_reject_rollback_after_restart() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = bob
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    assert!(alice.remember_node_record(&first, "peer-B", NOW).unwrap());
    let queued = alice
        .send_message(
            &group,
            "Already queued before the address changed",
            "route-original",
            NOW,
        )
        .unwrap();
    let wire = alice.outbox(10).unwrap()[0].wire.clone();
    let second = bob
        .publish_node_record("peer-B", addresses(4102), NOW)
        .unwrap();
    assert!(alice.remember_node_record(&second, "peer-B", NOW).unwrap());
    let before = persisted_state(&ar);
    assert!(!alice.remember_node_record(&first, "peer-B", NOW).unwrap());
    assert!(!alice.remember_node_record(&second, "peer-B", NOW).unwrap());
    assert_eq!(persisted_state(&ar), before);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let legacy = bob
        .create_node_record("peer-B", addresses(4199), NOW + 1)
        .unwrap();
    assert!(
        !alice
            .remember_node_record(&legacy, "peer-B", NOW + 1)
            .unwrap()
    );
    assert!(
        !alice
            .remember_node_record(&first, "peer-B", NOW + 1)
            .unwrap()
    );
    let pending = alice.outbox(10).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].message_id, queued.id);
    assert_eq!(pending[0].wire, wire);
    assert_eq!(pending[0].addresses, addresses(4102));
    let receipt = bob
        .receive(&pending[0].wire, NOW + 1)
        .unwrap()
        .reply
        .unwrap();
    alice
        .receive_from(&receipt, &second, "peer-B", NOW + 1)
        .unwrap();
    assert!(alice.outbox(10).unwrap().is_empty());
    assert_eq!(alice.snapshot().unwrap().conversations[0].messages.len(), 1);
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].id,
        queued.id
    );
    assert_eq!(
        alice.snapshot().unwrap().conversations[0].messages[0]
            .delivery
            .phase,
        "delivered"
    );
}

#[test]
fn incoming_application_updates_routes_but_invalid_envelope_does_not_poison_cache() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = bob
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    alice.remember_node_record(&first, "peer-B", NOW).unwrap();
    let second = bob
        .publish_node_record("peer-B", addresses(4102), NOW)
        .unwrap();
    bob.send_message(&group, "My new address", "route-update", NOW)
        .unwrap();
    let wire = bob.outbox(10).unwrap()[0].wire.clone();
    let before = persisted_state(&ar);
    let mut corrupt = wire.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(
        alice
            .receive_from(&corrupt, &second, "peer-B", NOW)
            .is_err()
    );
    assert_eq!(persisted_state(&ar), before);
    let receipt = alice
        .receive_from(&wire, &second, "peer-B", NOW)
        .unwrap()
        .reply
        .unwrap();
    bob.receive(&receipt, NOW).unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    alice
        .send_message(&group, "Reply to the new address", "new-route-reply", NOW)
        .unwrap();
    assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4102));
    // A delayed valid message may carry an older binding; consume the message, retain the new route.
    bob.send_message(&group, "Delayed old route", "old-route-valid-message", NOW)
        .unwrap();
    alice
        .receive_from(&bob.outbox(10).unwrap()[0].wire, &first, "peer-B", NOW)
        .unwrap();
    assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4102));
    assert_eq!(alice.snapshot().unwrap().conversations[0].messages.len(), 3);
}

#[test]
fn signatures_network_peer_expiry_and_equivocation_are_checked_before_route_mutation() {
    let root = TempDir::new().unwrap();
    let mut node = profile(&root, "Alice");
    let signer = runtime_key(91);
    let valid = fixture(&signer, 4, 4104, DOMAIN, NOW);
    assert!(node.remember_node_record(&valid, "peer-B", NOW).unwrap());
    let before = persisted_state(&root);
    let mut tampered = valid.clone();
    *tampered.last_mut().unwrap() ^= 1;
    for (wire, peer, time) in [
        (tampered, "peer-B", NOW),
        (valid.clone(), "peer-E", NOW),
        (valid.clone(), "peer-B", NOW + 86400),
        (fixture(&signer, 5, 4199, [8; 32], NOW), "peer-B", NOW),
        (fixture(&signer, 5, 4199, DOMAIN, NOW + 31), "peer-B", NOW),
        (fixture(&signer, 0, 4199, DOMAIN, NOW), "peer-B", NOW),
    ] {
        assert!(node.remember_node_record(&wire, peer, time).is_err());
        assert_eq!(persisted_state(&root), before);
    }
    assert!(
        !node
            .remember_node_record(&fixture(&signer, 4, 4199, DOMAIN, NOW), "peer-B", NOW)
            .unwrap()
    );
    assert_eq!(persisted_state(&root), before);
    let parsed = VerifiedDocument::decode(&valid, DOMAIN, NOW).unwrap();
    let mut noncanonical = parsed.body().to_vec();
    noncanonical.splice(1..2, [0x18, 0x02]);
    let mut trailing = parsed.body().to_vec();
    trailing.push(0);
    for (expiry, body) in [
        (None, parsed.body().to_vec()),
        (Some(NOW + 86401), parsed.body().to_vec()),
        (Some(NOW + 86400), noncanonical),
        (Some(NOW + 86400), trailing),
    ] {
        let invalid = SignedDocument::sign(
            DocumentDraft {
                domain: DOMAIN,
                kind: DocumentKind::Identity,
                authority_epoch: 0,
                issued_at: NOW,
                expires_at: expiry,
                body,
                extensions: BTreeMap::new(),
            },
            &signer,
        )
        .unwrap()
        .to_wire();
        assert!(node.remember_node_record(&invalid, "peer-B", NOW).is_err());
        assert_eq!(persisted_state(&root), before);
    }
    let records = node.cached_node_records(NOW).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].addresses, addresses(4104));
    assert!(
        node.snapshot().unwrap().conversations.is_empty(),
        "a signed routing hint does not create a trusted chat"
    );
    assert!(node.cached_node_records(NOW + 86400).unwrap().is_empty());
}

#[test]
fn cache_is_bounded_and_eviction_or_expiry_cannot_erase_confirmed_contact_high_water_mark() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let old = bob
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    let latest = bob
        .publish_node_record("peer-B", addresses(4102), NOW)
        .unwrap();
    alice.remember_node_record(&latest, "peer-B", NOW).unwrap();
    alice
        .send_message(&group, "Keep the confirmed root", "bounded-contact", NOW)
        .unwrap();
    let destination = alice.outbox(10).unwrap()[0].destination.clone();
    for n in 1..=70 {
        let record = fixture(&runtime_key(n), 1, 5000 + u16::from(n), DOMAIN, NOW + 1);
        assert!(
            alice
                .remember_node_record(&record, "peer-B", NOW + 1)
                .unwrap()
        );
        assert!(alice.cached_node_records(NOW + 1).unwrap().len() <= 64);
    }
    assert_eq!(alice.cached_node_records(NOW + 1).unwrap().len(), 64);
    assert!(
        alice
            .cached_node_records(NOW + 1)
            .unwrap()
            .iter()
            .all(|r| agentic_protocol::network_id(&r.author) != destination),
        "the test must actually evict Bob's cache entry"
    );
    assert_eq!(alice.snapshot().unwrap().conversations.len(), 1);
    assert_eq!(alice.outbox(10).unwrap()[0].destination, destination);
    assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4102));
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert!(!alice.remember_node_record(&old, "peer-B", NOW + 2).unwrap());
    assert!(alice.cached_node_records(NOW + 86401).unwrap().is_empty());
    let legacy = bob
        .create_node_record("peer-B", addresses(4199), NOW + 86401)
        .unwrap();
    assert!(
        !alice
            .remember_node_record(&legacy, "peer-B", NOW + 86401)
            .unwrap()
    );
    let before = persisted_state(&ar);
    for sequence in [1, 2] {
        let valid_conflict = fixture(&signer(&br), sequence, 4199, DOMAIN, NOW + 86401);
        assert!(
            alice
                .verify_node_record(&valid_conflict, "peer-B", NOW + 86401)
                .is_ok(),
            "the rollback attempt must pass cryptographic expiry checks"
        );
        assert!(
            !alice
                .remember_node_record(&valid_conflict, "peer-B", NOW + 86401)
                .unwrap()
        );
        assert_eq!(persisted_state(&ar), before);
        assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4102));
    }
    assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4102));
    let renewed = bob
        .publish_node_record("peer-B", addresses(4103), NOW + 86401)
        .unwrap();
    assert!(
        alice
            .remember_node_record(&renewed, "peer-B", NOW + 86401)
            .unwrap()
    );
    assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4103));
}

#[test]
fn failed_route_cache_transaction_keeps_previous_cache_and_contact_until_exact_retry() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = bob
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    alice.remember_node_record(&first, "peer-B", NOW).unwrap();
    let sent = alice
        .send_message(&group, "Survive storage failure", "route-disk", NOW)
        .unwrap();
    let second = bob
        .publish_node_record("peer-B", addresses(4102), NOW)
        .unwrap();
    let db = database(&ar);
    db.execute_batch("CREATE TRIGGER fail_route_update BEFORE UPDATE ON states WHEN NEW.namespace='network/peer-records' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    let before = persisted_state(&ar);
    assert!(alice.remember_node_record(&second, "peer-B", NOW).is_err());
    assert_eq!(persisted_state(&ar), before);
    assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4101));
    db.execute_batch("DROP TRIGGER fail_route_update;").unwrap();
    drop(db);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert!(alice.remember_node_record(&second, "peer-B", NOW).unwrap());
    let queued = alice.outbox(10).unwrap();
    assert_eq!(queued[0].message_id, sent.id);
    assert_eq!(queued[0].addresses, addresses(4102));
    assert_eq!(
        alice.cached_node_records(NOW).unwrap()[0].addresses,
        addresses(4102)
    );
}

#[test]
fn failed_publication_does_not_issue_a_record_without_persisting_its_sequence() {
    let root = TempDir::new().unwrap();
    let mut node = profile(&root, "Bob");
    let db = database(&root);
    db.execute_batch("CREATE TRIGGER fail_publish_insert BEFORE INSERT ON states WHEN NEW.namespace='network/own-record' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    let before = persisted_state(&root);
    assert!(
        node.publish_node_record("peer-B", addresses(4101), NOW)
            .is_err()
    );
    assert_eq!(persisted_state(&root), before);
    db.execute_batch("DROP TRIGGER fail_publish_insert;")
        .unwrap();
    let first = node
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    assert_eq!(
        node.verify_node_record(&first, "peer-B", NOW)
            .unwrap()
            .sequence,
        1
    );
    db.execute_batch("CREATE TRIGGER fail_publish_update BEFORE UPDATE ON states WHEN NEW.namespace='network/own-record' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    let before = persisted_state(&root);
    assert!(
        node.publish_node_record("peer-B", addresses(4102), NOW)
            .is_err()
    );
    assert_eq!(persisted_state(&root), before);
    assert_eq!(
        node.publish_node_record("peer-B", addresses(4101), NOW)
            .unwrap(),
        first
    );
    db.execute_batch("DROP TRIGGER fail_publish_update;")
        .unwrap();
    drop(db);
    drop(node);
    let mut node = core(&root, DOMAIN);
    let second = node
        .publish_node_record("peer-B", addresses(4102), NOW)
        .unwrap();
    assert_eq!(
        node.verify_node_record(&second, "peer-B", NOW)
            .unwrap()
            .sequence,
        2
    );
}

#[test]
fn cache_failure_after_incoming_commit_recovers_on_duplicate_without_advancing_mls_twice() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = bob
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    alice.remember_node_record(&first, "peer-B", NOW).unwrap();
    let second = bob
        .publish_node_record("peer-B", addresses(4102), NOW)
        .unwrap();
    let sent = bob
        .send_message(
            &group,
            "Committed before route cache failed",
            "route-partial",
            NOW,
        )
        .unwrap();
    let wire = bob.outbox(10).unwrap()[0].wire.clone();
    let db = database(&ar);
    db.execute_batch("CREATE TRIGGER fail_incoming_route BEFORE UPDATE ON states WHEN NEW.namespace='network/peer-records' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    assert!(alice.receive_from(&wire, &second, "peer-B", NOW).is_err());
    let history = alice.snapshot().unwrap();
    assert_eq!(history.conversations[0].messages.len(), 1);
    assert_eq!(history.conversations[0].messages[0].id, sent.id);
    assert_eq!(
        history.conversations[0].messages[0].text,
        "Committed before route cache failed"
    );
    assert_eq!(
        alice.cached_node_records(NOW).unwrap()[0].addresses,
        addresses(4101)
    );
    let mls = persisted_state(&ar)
        .0
        .into_iter()
        .find(|s| s.0 == "mls")
        .unwrap();
    drop(alice);
    db.execute_batch("DROP TRIGGER fail_incoming_route;")
        .unwrap();
    drop(db);
    let mut alice = core(&ar, DOMAIN);
    let receipt = alice
        .receive_from(&wire, &second, "peer-B", NOW)
        .unwrap()
        .reply
        .unwrap();
    assert_eq!(alice.snapshot().unwrap(), history);
    assert_eq!(
        persisted_state(&ar)
            .0
            .into_iter()
            .find(|s| s.0 == "mls")
            .unwrap(),
        mls
    );
    assert_eq!(
        alice.cached_node_records(NOW).unwrap()[0].addresses,
        addresses(4102)
    );
    bob.receive(&receipt, NOW).unwrap();
    assert!(bob.outbox(10).unwrap().is_empty());
    alice
        .send_message(&group, "Reply after recovery", "route-partial-reply", NOW)
        .unwrap();
    let pending = alice.outbox(10).unwrap();
    assert_eq!(pending[0].addresses, addresses(4102));
    let receipt = bob.receive(&pending[0].wire, NOW).unwrap().reply.unwrap();
    alice.receive(&receipt, NOW).unwrap();
    assert!(alice.outbox(10).unwrap().is_empty());
    assert_eq!(bob.snapshot().unwrap().conversations[0].messages.len(), 2);
}

#[test]
fn temporarily_unadvertised_peer_retains_last_dial_hint_without_rolling_back_its_record() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let first = bob
        .publish_node_record("peer-B", addresses(4101), NOW)
        .unwrap();
    alice.remember_node_record(&first, "peer-B", NOW).unwrap();
    // AutoNAT can temporarily withdraw all confirmed published addresses while the old
    // endpoint still serves an existing contact. An empty record is not identity revocation.
    let withdrawn = bob.publish_node_record("peer-B", vec![], NOW).unwrap();
    bob.send_message(
        &group,
        "External address verification unavailable",
        "route-withdrawal",
        NOW,
    )
    .unwrap();
    let received = alice
        .receive_from(&bob.outbox(10).unwrap()[0].wire, &withdrawn, "peer-B", NOW)
        .unwrap();
    bob.receive(&received.reply.unwrap(), NOW).unwrap();
    let cached = alice.cached_node_records(NOW).unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].sequence, 2);
    assert!(
        cached[0].addresses.is_empty(),
        "discovery must not republish the obsolete record as current"
    );
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let before = persisted_state(&ar);
    assert!(!alice.remember_node_record(&first, "peer-B", NOW).unwrap());
    assert_eq!(persisted_state(&ar), before);
    let sent = alice
        .send_message(
            &group,
            "Attempt the last authenticated hint",
            "route-last-hint",
            NOW,
        )
        .unwrap();
    assert_eq!(sent.delivery.phase, "queued");
    let pending = alice.outbox(10).unwrap();
    assert_eq!(pending[0].addresses, addresses(4101));
    assert_eq!(pending[0].message_id, sent.id);
    let receipt = bob.receive(&pending[0].wire, NOW).unwrap().reply.unwrap();
    alice
        .receive_from(&receipt, &withdrawn, "peer-B", NOW)
        .unwrap();
    assert!(alice.outbox(10).unwrap().is_empty());
    assert_eq!(
        alice.snapshot().unwrap().conversations[0].messages[1]
            .delivery
            .phase,
        "delivered"
    );
    let replacement = bob
        .publish_node_record("peer-B", addresses(4103), NOW)
        .unwrap();
    assert!(
        alice
            .remember_node_record(&replacement, "peer-B", NOW)
            .unwrap()
    );
    alice
        .send_message(
            &group,
            "Use the newly confirmed route",
            "route-after-withdrawal",
            NOW,
        )
        .unwrap();
    assert_eq!(alice.outbox(10).unwrap()[0].addresses, addresses(4103));
    assert_eq!(alice.cached_node_records(NOW).unwrap()[0].sequence, 3);
}
