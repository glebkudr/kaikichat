//! Big groups at the core (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
//! parts 1–3): the group mailbox stays when people join and changes when
//! someone is removed; a commit or a ratchet tree too big for one envelope
//! goes in parts, each paid, and is stored only when every part is; a
//! newcomer to a big group joins once it has put the tree together from its
//! own mailbox.
use super::*;
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::receipt::{HolderKey, Receipt};

fn mailboxes(p: &Person, group: &str) -> Vec<[u8; 32]> {
    p.core.group_mailboxes(group, NOW).unwrap()
}

/// `from`'s pending swarm deliveries in `group` into `mailbox`, oldest
/// first; invitations, which also go directly, are left alone.
fn pending_into(from: &mut Person, group: &str, mailbox: &[u8; 32]) -> Vec<SwarmDelivery> {
    let invitations: BTreeSet<String> = from
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .map(|w| w.message_id)
        .collect();
    let items: Vec<_> = from
        .core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .filter(|p| p.conversation_id == group && !invitations.contains(&p.message_id))
        .collect();
    items
        .iter()
        .map(|p| {
            from.core
                .prepare_swarm_delivery(&p.message_id, NOW)
                .unwrap()
        })
        .filter(|d| &d.mailbox == mailbox)
        .collect()
}

/// The ids in `from`'s swarm outbox, in order.
fn swarm_ids(from: &Person) -> Vec<String> {
    from.core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .map(|p| p.message_id)
        .collect()
}

/// Deliver `from`'s invitation of `to` through `to`'s intro mailbox only;
/// `to` is among the first hundred invited.
fn invite_one(from: &mut Person, to: &mut Person) -> IntroOutcome {
    let id = from
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|w| w.destination == to.id)
        .expect("an invitation")
        .message_id;
    let d = from.core.prepare_swarm_delivery(&id, NOW).unwrap();
    assert_eq!(d.mailbox, to.core.own_intro_mailbox(NOW).unwrap());
    // Its Welcome is its own, not the whole batch's: small however many
    // were added with it.
    assert!(d.envelope.len() <= 8 * 1024, "{} B", d.envelope.len());
    to.core.receive_intro_envelope(&d.envelope, NOW).unwrap()
}

fn holder(n: u8) -> HolderKey {
    HolderKey::from_bytes(&[n; 32]).unwrap()
}

/// The ten holders of every swarm here.
fn swarm() -> Vec<Account> {
    (1..=10).map(|n| holder(n).account()).collect()
}

/// A quorum of holders' receipts for one delivery.
fn receipts_for(d: &SwarmDelivery) -> Vec<Receipt> {
    (1..=7)
        .map(|n| {
            Receipt::sign(
                &DOMAIN,
                d.mailbox,
                d.stamp.operation,
                d.stamp.ticket_id(&DOMAIN),
                [n; 32],
                NOW + 1,
                &holder(n),
            )
        })
        .collect()
}

/// A delivery stored by a quorum of holders.
fn stored(from: &mut Person, d: &SwarmDelivery) {
    from.core
        .complete_swarm_delivery(&d.message_id, &swarm(), &receipts_for(d))
        .unwrap();
}

#[test]
fn people_joining_keep_the_groups_mailbox_and_a_removal_changes_it() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = team(&mut alice, &mut bob, &mut carol);
    let first = mailboxes(&alice, &g);
    assert_eq!(first.len(), 1);
    let early = say(&mut bob, &g, "before Dave", "m-1");
    read(&mut alice, &g, &early).unwrap();

    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        add(&mut dave),
        "c-1",
    );
    // A small group's tree rides in the invitation: Dave joins at once.
    let outcomes = deliver_invites(&mut alice, &mut [&mut dave]);
    assert!(matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g));
    for p in [&alice, &bob, &carol, &dave] {
        assert_eq!(mailboxes(p, &g), first, "{}", p.id);
    }
    // What was written before him lies in the same mailbox; he passes it
    // over quietly, unread.
    read(&mut dave, &g, &early).unwrap();
    assert!(texts(&dave, &g).is_empty());
    let hi = say(&mut dave, &g, "hi, I am Dave", "m-2");
    assert_eq!(hi.mailbox, first[0]);
    read(&mut bob, &g, &hi).unwrap();
    assert_eq!(texts(&bob, &g).last().unwrap().1, "hi, I am Dave");

    // A removal moves everyone left to a mailbox the removed one cannot find.
    let out = remove(&carol);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol, &mut dave],
        &g,
        out,
        "c-2",
    );
    let now = mailboxes(&alice, &g);
    assert_ne!(now[0], first[0]);
    assert_eq!(
        now[1], first[0],
        "the old one is still read for late messages"
    );
    assert_eq!(mailboxes(&bob, &g), now);
    assert_eq!(mailboxes(&dave, &g), now);
    let after = say(&mut alice, &g, "without Carol", "m-3");
    assert_eq!(after.mailbox, now[0]);
    read(&mut dave, &g, &after).unwrap();
    assert!(carol.core.group_mailboxes(&g, NOW).is_err());
    assert!(read(&mut carol, &g, &after).is_err());
    assert!(!texts(&carol, &g).iter().any(|(_, t)| t == "without Carol"));
}

#[test]
fn two_hundred_eighty_join_at_once_and_the_commit_and_the_tree_go_in_parts() {
    let mut owner = person("Owner");
    let mut alice = person("Alice");
    let g = owner
        .core
        .create_group("Many", &[invitee(&mut alice)], "g-1", NOW)
        .unwrap()
        .id;
    deliver_invites(&mut owner, &mut [&mut alice]);
    let group_mailbox = mailboxes(&alice, &g);
    let mut newcomer = person("Newcomer");
    let mut add = vec![invitee(&mut newcomer)];
    add.extend((1..280).map(|i| card_of(&format!("P{i}"))));
    let e = epoch(&owner, &g);
    let made = owner
        .core
        .change_group(
            &g,
            GroupChange {
                add,
                ..GroupChange::default()
            },
            "c-1",
            NOW,
        )
        .unwrap();

    // The commit (about 100 KB) goes into the group mailbox in parts, each
    // its own delivery and stamp.
    let used = owner.core.mailbox_books().unwrap()[0].used;
    let parts = pending_into(&mut owner, &g, &group_mailbox[0]);
    assert!(parts.len() >= 2, "{} parts", parts.len());
    assert_eq!(
        owner.core.mailbox_books().unwrap()[0].used,
        used + parts.len() as u32
    );
    // Stored only when every part is: one part at a quorum is not enough.
    stored(&mut owner, &parts[0]);
    assert!(
        owner
            .core
            .swarm_receipts(&made.message_id)
            .unwrap()
            .is_none()
    );
    let ids = swarm_ids(&owner);
    assert!(!ids.contains(&parts[0].message_id));
    assert!(ids.contains(&parts[1].message_id));
    for d in &parts[1..] {
        stored(&mut owner, d);
    }
    assert!(
        owner
            .core
            .swarm_receipts(&made.message_id)
            .unwrap()
            .is_some()
    );
    let ids = swarm_ids(&owner);
    assert!(parts.iter().all(|d| !ids.contains(&d.message_id)));

    // Alice reads the parts in any order; until the last one there is no
    // commit to apply.
    for d in parts.iter().skip(1).rev() {
        read(&mut alice, &g, d).unwrap();
    }
    read(&mut alice, &g, &parts[1]).unwrap();
    assert_eq!(
        alice
            .core
            .decide_group_commit(&g, e, &made.commit, NOW)
            .unwrap(),
        CommitDecision::Waiting
    );
    read(&mut alice, &g, &parts[0]).unwrap();
    owner
        .core
        .decide_group_commit(&g, e, &made.commit, NOW)
        .unwrap();
    assert_eq!(
        alice
            .core
            .decide_group_commit(&g, e, &made.commit, NOW)
            .unwrap(),
        CommitDecision::Applied
    );
    assert_eq!(alice.core.group(&g).unwrap().members.len(), 282);
    assert_eq!(mailboxes(&alice, &g), group_mailbox);

    // The newcomer's invitation names the tree; it waits for it, also
    // across a restart.
    let outcome = invite_one(&mut owner, &mut newcomer);
    assert!(
        matches!(&outcome, IntroOutcome::AwaitingTree(id) if *id == g),
        "{outcome:?}"
    );
    assert!(newcomer.core.group(&g).is_err());
    let Person {
        _dir,
        core,
        id,
        root,
    } = newcomer;
    drop(core);
    let mut newcomer = Person {
        core: crate::core(&_dir, DOMAIN),
        _dir,
        id,
        root,
    };
    let wanted = newcomer.core.group_tree_mailboxes(NOW).unwrap();
    let [(id, tree_mailbox)] = wanted.as_slice() else {
        panic!("{wanted:?}")
    };
    assert_eq!(id, &g);
    // Members never download the tree: it lies in a mailbox of its own.
    assert!(!group_mailbox.contains(tree_mailbox));
    assert!(alice.core.group_tree_mailboxes(NOW).unwrap().is_empty());

    // The tree was queued before the invitations, so it is on its way
    // before anyone waits for it; it takes more than one part.
    let used = owner.core.mailbox_books().unwrap()[0].used;
    let tree = pending_into(&mut owner, &g, tree_mailbox);
    assert!(tree.len() >= 2, "the tree of 282 does not fit one envelope");
    assert_eq!(
        owner.core.mailbox_books().unwrap()[0].used,
        used + tree.len() as u32
    );
    let order = swarm_ids(&owner);
    let direct: BTreeSet<String> = owner
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .map(|w| w.message_id)
        .collect();
    let invitation = order.iter().position(|i| direct.contains(i)).unwrap();
    for d in &tree {
        assert!(order.iter().position(|i| *i == d.message_id).unwrap() < invitation);
    }
    let mut joined = None;
    for d in tree.iter().rev() {
        assert!(joined.is_none(), "joined before the tree was whole");
        joined = newcomer
            .core
            .receive_group_tree_envelope(&g, d.period, &d.envelope, NOW)
            .unwrap();
    }
    assert_eq!(joined.unwrap().id, g);
    assert!(newcomer.core.group_tree_mailboxes(NOW).unwrap().is_empty());
    let info = newcomer.core.group(&g).unwrap();
    assert_eq!((info.epoch, info.members.len()), (e + 1, 282));
    assert_eq!(mailboxes(&newcomer, &g), group_mailbox);
    // The commit that added it lies in the same mailbox: passed over.
    for d in &parts {
        read(&mut newcomer, &g, d).unwrap();
    }
    assert_eq!(epoch(&newcomer, &g), e + 1);

    // The owner's queue still holds invitations: take its message by id.
    let sent = owner
        .core
        .send_message(&g, "to all two hundred eighty", "m-1", NOW)
        .unwrap();
    let hello = owner.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    for p in [&mut alice, &mut newcomer] {
        read(p, &g, &hello).unwrap();
        assert_eq!(texts(p, &g).last().unwrap().1, "to all two hundred eighty");
    }
    let reply = say(&mut newcomer, &g, "I am new here", "m-2");
    assert_eq!(reply.mailbox, group_mailbox[0]);
    for p in [&mut alice, &mut owner] {
        read(p, &g, &reply).unwrap();
        assert_eq!(texts(p, &g).last().unwrap().1, "I am new here");
    }
}

// --- messages that outlived three epochs in their sender's outbox ----------

/// When `p` shows `text` in `group`.
fn shown_at(p: &Person, group: &str, text: &str) -> Vec<u64> {
    p.core
        .snapshot()
        .unwrap()
        .conversations
        .into_iter()
        .find(|c| c.id == group)
        .map(|c| {
            c.messages
                .into_iter()
                .filter(|m| m.text == text)
                .map(|m| m.created_at)
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_message_three_epochs_old_is_sealed_again_under_its_id_and_read_once() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let erin = person("Erin").id;
    let g = team(&mut alice, &mut bob, &mut carol);
    // Written while Bob reached no holder: pinned, never stored.
    let sent = bob
        .core
        .send_message(&g, "while you were away", "late", NOW)
        .unwrap();
    let old = bob.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    let first = mailboxes(&alice, &g);

    // Two epochs on, everyone still holds its keys: left as it is.
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 2);
    assert!(bob.core.reseal_stale_group_sends().unwrap().is_empty());
    let same = bob.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    assert_eq!(same.envelope, old.envelope);
    // Three: one more and the members drop them. Sealed again, once, a few
    // minutes later.
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 1);
    let later = NOW + 300;
    assert_eq!(
        bob.core.reseal_stale_group_sends().unwrap(),
        vec![sent.id.clone()]
    );
    assert!(bob.core.reseal_stale_group_sends().unwrap().is_empty());
    // Alice's commits wait in her outbox as long: a commit is never sealed
    // again.
    assert!(alice.core.reseal_stale_group_sends().unwrap().is_empty());
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 1);
    assert_eq!(mailboxes(&alice, &g), first);

    // What was sealed first is lost to the members now.
    let _ = read(&mut carol, &g, &old);
    assert!(texts(&carol, &g).is_empty());
    // Still one message waiting on Bob's side, under its id; receipts for
    // the first envelope no longer store it.
    assert_eq!(swarm_ids(&bob), vec![sent.id.clone()]);
    assert!(
        bob.core
            .complete_swarm_delivery(&sent.id, &swarm(), &receipts_for(&old))
            .is_err()
    );
    let again = bob.core.prepare_swarm_delivery(&sent.id, later).unwrap();
    assert!(
        bob.core
            .complete_swarm_delivery(&sent.id, &swarm(), &receipts_for(&old))
            .is_err()
    );
    assert_eq!(swarm_ids(&bob), vec![sent.id.clone()]);
    assert_eq!(again.mailbox, first[0]);
    assert_ne!(again.envelope, old.envelope);
    for p in [&mut alice, &mut carol] {
        read(p, &g, &again).unwrap();
        // Read twice, from two holders: shown once, when it was written.
        read(p, &g, &again).unwrap();
        assert_eq!(
            texts(p, &g),
            vec![(bob.id.clone(), "while you were away".into())]
        );
        assert_eq!(shown_at(p, &g, "while you were away"), vec![NOW]);
    }
    // Bob reads the group's mailbox too: his own message, shown once.
    read(&mut bob, &g, &again).unwrap();
    stored(&mut bob, &again);
    assert!(swarm_ids(&bob).is_empty());
    assert_eq!(
        texts(&bob, &g),
        vec![(bob.id.clone(), "while you were away".into())]
    );
}

#[test]
fn a_message_is_not_sealed_again_while_its_sender_has_a_commit_waiting() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let erin = person("Erin").id;
    let g = team(&mut alice, &mut bob, &mut carol);
    let sent = alice
        .core
        .send_message(&g, "from the queue", "late", NOW)
        .unwrap();
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 3);
    // A commit of hers waits for the notary: the group is busy, and the
    // message waits with it.
    let e = epoch(&alice, &g);
    let what = toggle(&alice, &g, &erin);
    let commit = alice
        .core
        .change_group(&g, what, "busy", NOW)
        .unwrap()
        .commit;
    assert!(alice.core.reseal_stale_group_sends().unwrap().is_empty());
    let d = pending_in(&mut alice, &g);
    for p in [&mut bob, &mut carol] {
        read(p, &g, &d).unwrap();
    }
    decide_everywhere(&mut [&mut alice, &mut bob, &mut carol], &g, e, &commit);
    assert_eq!(
        alice.core.reseal_stale_group_sends().unwrap(),
        vec![sent.id.clone()]
    );
    let again = alice.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    for p in [&mut bob, &mut carol] {
        read(p, &g, &again).unwrap();
        assert_eq!(
            texts(p, &g),
            vec![(alice.id.clone(), "from the queue".into())]
        );
    }
}

#[test]
fn a_message_whose_mailbox_nobody_reads_any_more_is_sealed_again_for_todays() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = team(&mut alice, &mut bob, &mut carol);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        add(&mut dave),
        "dave",
    );
    deliver_invites(&mut alice, &mut [&mut dave]);
    let sent = bob
        .core
        .send_message(&g, "before two removals", "late", NOW)
        .unwrap();
    let old = bob.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    // One removal: the members still read the mailbox it waits for.
    change(
        &mut alice,
        &mut [&mut bob, &mut dave],
        &g,
        remove(&carol),
        "no-carol",
    );
    assert!(bob.core.reseal_stale_group_sends().unwrap().is_empty());
    assert!(mailboxes(&alice, &g).contains(&old.mailbox));
    // Two: they read only the two newest; two epochs on, sealed again.
    change(&mut alice, &mut [&mut bob], &g, remove(&dave), "no-dave");
    assert!(!mailboxes(&alice, &g).contains(&old.mailbox));
    assert_eq!(
        bob.core.reseal_stale_group_sends().unwrap(),
        vec![sent.id.clone()]
    );
    let again = bob.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    assert_eq!(again.mailbox, mailboxes(&alice, &g)[0]);
    read(&mut alice, &g, &again).unwrap();
    assert_eq!(
        texts(&alice, &g),
        vec![(bob.id.clone(), "before two removals".into())]
    );
    // Carol, removed before it was sealed again, reads nothing of it.
    let _ = read(&mut carol, &g, &again);
    assert!(texts(&carol, &g).is_empty());
}

#[test]
fn a_senders_stale_messages_are_sealed_again_in_their_order() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let erin = person("Erin").id;
    let g = team(&mut alice, &mut bob, &mut carol);
    let first = bob.core.send_message(&g, "one", "one", NOW).unwrap();
    let second = bob.core.send_message(&g, "two", "two", NOW).unwrap();
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 3);
    assert_eq!(
        bob.core.reseal_stale_group_sends().unwrap(),
        vec![first.id.clone(), second.id.clone()]
    );
    let one = bob.core.prepare_swarm_delivery(&first.id, NOW).unwrap();
    let two = bob.core.prepare_swarm_delivery(&second.id, NOW).unwrap();
    // The second, read first, waits for the first.
    assert!(matches!(
        read(&mut alice, &g, &two),
        Err(CoreError::Crypto(agentic_crypto::CryptoError::ReceiveGap))
    ));
    read(&mut alice, &g, &one).unwrap();
    read(&mut alice, &g, &two).unwrap();
    assert_eq!(
        texts(&alice, &g),
        vec![
            (bob.id.clone(), "one".into()),
            (bob.id.clone(), "two".into())
        ]
    );
}

/// Accepted: a later message of the same epoch already stored stays behind
/// the gap its sealed-again predecessor left (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 1).
#[test]
fn a_later_message_of_the_same_epoch_already_stored_stays_behind_the_gap() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let erin = person("Erin").id;
    let g = team(&mut alice, &mut bob, &mut carol);
    let first = bob.core.send_message(&g, "one", "one", NOW).unwrap();
    let second = bob.core.send_message(&g, "two", "two", NOW).unwrap();
    let two = bob.core.prepare_swarm_delivery(&second.id, NOW).unwrap();
    stored(&mut bob, &two);
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 3);
    assert_eq!(
        bob.core.reseal_stale_group_sends().unwrap(),
        vec![first.id.clone()]
    );
    let one = bob.core.prepare_swarm_delivery(&first.id, NOW).unwrap();
    read(&mut alice, &g, &one).unwrap();
    assert!(matches!(
        read(&mut alice, &g, &two),
        Err(CoreError::Crypto(agentic_crypto::CryptoError::ReceiveGap))
    ));
    assert_eq!(texts(&alice, &g), vec![(bob.id.clone(), "one".into())]);
}

#[test]
fn an_open_groups_post_is_never_sealed_again() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let erin = person("Erin").id;
    let g = opened(&mut alice, &mut bob, &mut carol);
    let sent = bob
        .core
        .send_message(&g, "for everyone", "post", NOW)
        .unwrap();
    let before = bob.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 3);
    assert!(bob.core.reseal_stale_group_sends().unwrap().is_empty());
    let after = bob.core.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    assert_eq!(
        (after.mailbox, after.envelope),
        (before.mailbox, before.envelope)
    );
}

#[test]
fn a_member_of_two_groups_works_in_one_whatever_the_other_holds() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let erin = person("Erin").id;
    let first = team(&mut alice, &mut bob, &mut carol);
    let invitees = vec![invitee(&mut bob), invitee(&mut dave)];
    let second = alice
        .core
        .create_group("Second", &invitees, "g-2", NOW)
        .unwrap()
        .id;
    deliver_invites(&mut alice, &mut [&mut bob, &mut dave]);
    // Bob waits with a message in each while both move on: both sealed again.
    let one = bob
        .core
        .send_message(&first, "in the first", "one", NOW)
        .unwrap();
    let two = bob
        .core
        .send_message(&second, "in the second", "two", NOW)
        .unwrap();
    move_on(&mut alice, &mut [&mut bob, &mut carol], &first, &erin, 3);
    move_on(&mut alice, &mut [&mut bob, &mut dave], &second, &erin, 3);
    assert_eq!(
        bob.core.reseal_stale_group_sends().unwrap(),
        vec![one.id.clone(), two.id.clone()]
    );
    whole_mls(&bob);

    // The second group's own records at Bob's go bad: the first group works.
    let group: [u8; 32] = hex::decode(&second).unwrap().try_into().unwrap();
    let common = agentic_crypto::record_scope(None);
    let own: Vec<Vec<u8>> = agentic_crypto::record_scope(Some(group))
        .into_iter()
        .filter(|prefix| !common.contains(prefix))
        .collect();
    let (_, rows) = mls_rows(&bob);
    let spoiled: Vec<_> = rows
        .into_iter()
        .filter(|(key, _)| own.iter().any(|prefix| key.starts_with(prefix)))
        .collect();
    assert!(spoiled.len() > 3);
    let db = super::super::profile_db::encrypted_db(&bob._dir);
    for (key, _) in &spoiled {
        db.execute(
            "UPDATE state_records SET bytes=x'824040' WHERE namespace='mls' AND record_key=?1",
            [key],
        )
        .unwrap();
    }
    let again = bob.core.prepare_swarm_delivery(&one.id, NOW).unwrap();
    read(&mut alice, &first, &again).unwrap();
    move_on(&mut alice, &mut [&mut bob, &mut carol], &first, &erin, 1);
    let hi = say(&mut alice, &first, "to everyone in the first", "hi");
    read(&mut bob, &first, &hi).unwrap();
    assert_eq!(
        texts(&bob, &first).last().unwrap().1,
        "to everyone in the first"
    );
    assert!(
        bob.core
            .send_message(&second, "no way", "bad", NOW)
            .is_err()
    );

    // Put back, the whole profile holds, and the second group works again.
    for (key, bytes) in &spoiled {
        db.execute(
            "UPDATE state_records SET bytes=?2 WHERE namespace='mls' AND record_key=?1",
            rusqlite::params![key, bytes],
        )
        .unwrap();
    }
    whole_mls(&bob);
    let waited = bob.core.prepare_swarm_delivery(&two.id, NOW).unwrap();
    read(&mut dave, &second, &waited).unwrap();
    let later = say(&mut bob, &second, "in the second again", "later");
    read(&mut dave, &second, &later).unwrap();
    assert_eq!(
        texts(&dave, &second),
        vec![
            (bob.id.clone(), "in the second".into()),
            (bob.id.clone(), "in the second again".into())
        ]
    );
}
