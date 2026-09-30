//! Closed channels (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts 8
//! and 10c): posts are sealed under the channel key, the root of a key tree
//! whose paths its subscribers hold; each subscriber gets its keys alone,
//! without a batch; a removal moves the channel to new keys only the rest
//! can derive, and a reseed gives the rest keys of a new seed.
use super::*;
use agentic_core::DoorEntry;
use agentic_protocol::group::Retention;

/// A channel of Alice, closed as `access` says, with Bob in its team.
fn closed(alice: &mut Person, bob: &mut Person, access: Access) -> String {
    let team = vec![invitee(bob)];
    let c = alice
        .core
        .create_channel("Club", &team, access, "ch-1", NOW)
        .unwrap()
        .id;
    deliver_invites(alice, &mut [bob]);
    c
}

/// `admin` gives `who` the channel's keys, which reach it through its intro
/// mailbox; the conversation `who` reads the channel in.
fn subscribe(admin: &mut Person, c: &str, who: &mut Person, op: &str) -> String {
    let card = invitee(who);
    admin.core.channel_subscribe(c, &card, op, NOW).unwrap();
    let outcomes = deliver_invites(admin, &mut [who]);
    let follow = hex::encode(reference(admin, c));
    assert!(
        matches!(outcomes.as_slice(), [IntroOutcome::Subscribed(id)] if *id == follow),
        "{outcomes:?}"
    );
    follow
}

fn unsubscribe(p: &Person) -> GroupChange {
    GroupChange {
        unsubscribe: vec![p.id.clone()],
        ..GroupChange::default()
    }
}

/// `admin`'s documents of new keys to publish, each read back from the
/// mailbox as its node does once it stored them there.
fn key_docs(admin: &mut Person, c: &str) -> Vec<SwarmDelivery> {
    let docs = admin.core.channel_key_docs(c, NOW).unwrap();
    for doc in &docs {
        admin
            .core
            .receive_public_entry(c, &doc.envelope, NOW, NOW)
            .unwrap();
    }
    assert!(admin.core.channel_key_docs(c, NOW).unwrap().is_empty());
    docs
}

fn take(p: &mut Person, follow: &str, d: &SwarmDelivery) -> PublicEntry {
    p.core
        .receive_public_entry(follow, &d.envelope, NOW, NOW)
        .unwrap()
}

fn holds(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

fn used(p: &Person) -> u32 {
    p.core.mailbox_books().unwrap()[0].used
}

#[test]
fn a_closed_channels_subscriber_reads_it_with_its_keys_and_nobody_else_can() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let c = closed(&mut alice, &mut bob, Access::Private);
    let gref = reference(&alice, &c);
    // A closed channel keeps no history, and stays closed.
    for (op, change) in [
        (
            "x-1",
            GroupChange {
                retention: Some(Retention::Days(90)),
                ..GroupChange::default()
            },
        ),
        ("x-2", access(Access::Public)),
    ] {
        assert!(
            matches!(
                alice.core.change_group(&c, change, op, NOW),
                Err(CoreError::InvalidInput)
            ),
            "{op}"
        );
    }
    // Written before Dave's keys: not his to read.
    let earlier = say(&mut bob, &c, "before subscribing", "p-0");
    // His keys go to him alone; the team is told where they hang: a stamp
    // each.
    let before = used(&alice);
    let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
    pending_in(&mut alice, &c);
    assert_eq!(used(&alice) - before, 2);
    let followed = dave.core.follows().unwrap();
    assert_eq!(
        (followed[0].id.as_str(), followed[0].kind.as_str()),
        (follow.as_str(), "channel")
    );
    let post = say(&mut bob, &c, "members only", "p-1");
    assert_eq!(
        dave.core.public_mailbox(&follow, NOW).unwrap(),
        Some(post.mailbox)
    );
    assert_ne!(
        post.mailbox,
        public_group_mailbox_id(&DOMAIN, &gref, period(NOW))
    );
    assert!(!holds(&post.envelope, "members only"));
    assert_eq!(take(&mut dave, &follow, &post), PublicEntry::Post);
    assert_eq!(
        dave.core
            .receive_public_entry(&follow, &earlier.envelope, NOW - 60, NOW)
            .unwrap(),
        PublicEntry::Ignored
    );
    assert_eq!(
        texts(&dave, &follow),
        [(bob.id.clone(), "members only".to_owned())]
    );
    // The team reads it too.
    assert_eq!(take(&mut alice, &c, &post), PublicEntry::Post);
    // Following it by its reference alone reads nothing of it.
    let outside = erin
        .core
        .follow_group(gref, &alice.id, "Club", NOW)
        .unwrap();
    assert_eq!(take(&mut erin, &outside.id, &post), PublicEntry::Ignored);
}

#[test]
fn an_applicant_to_a_channel_by_request_gets_its_keys_on_an_admins_decision_without_a_batch() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut erin = person("Erin");
    let c = closed(&mut alice, &mut bob, Access::Request);
    let gref = reference(&alice, &c);
    // Erin lets no stranger in by herself; a channel she knocked on is
    // another matter.
    door::manual(&mut erin);
    let card = door::door_card(&mut alice, &c);
    let application = door::knock(&mut erin, &gref, &card, "let me read");
    let DoorEntry::Waiting(request) = alice
        .core
        .receive_door_entry(&c, application.period, &application.envelope, NOW)
        .unwrap()
    else {
        panic!("an application waits for a decision");
    };
    assert_eq!(request.network_id, erin.id);
    alice
        .core
        .decide_door_request(&c, &request.request_id, true, NOW)
        .unwrap();
    // Nobody joins the team: the keys go at once, alone.
    assert!(alice.core.door_batch(&c, NOW).unwrap().is_none());
    let outcomes = deliver_invites(&mut alice, &mut [&mut erin]);
    let follow = hex::encode(gref);
    assert!(
        matches!(outcomes.as_slice(), [IntroOutcome::Subscribed(id)] if *id == follow),
        "{outcomes:?}"
    );
    assert_eq!(alice.core.group(&c).unwrap().members.len(), 2);
    let post = say(&mut alice, &c, "welcome", "p-1");
    assert_eq!(take(&mut erin, &follow, &post), PublicEntry::Post);
}

#[test]
fn keys_of_a_private_channel_wait_for_a_subscriber_who_lets_no_stranger_in() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut grace = person("Grace");
    let c = closed(&mut alice, &mut bob, Access::Private);
    door::manual(&mut grace);
    let card = invitee(&mut grace);
    let earlier = say(&mut alice, &c, "before the keys", "p-0");
    alice.core.channel_subscribe(&c, &card, "s-1", NOW).unwrap();
    let outcomes = deliver_invites(&mut alice, &mut [&mut grace]);
    let [IntroOutcome::Pending(request)] = outcomes.as_slice() else {
        panic!("{outcomes:?}");
    };
    assert!(grace.core.follows().unwrap().is_empty());
    let post = say(&mut alice, &c, "good to see you", "p-1");
    // An hour on she lets the keys in: what was written since they were
    // given is hers, what came before is not.
    let later = NOW + 3_600;
    grace
        .core
        .accept_intro_request(&request.request_id, later)
        .unwrap();
    let follow = hex::encode(reference(&alice, &c));
    let take_at = |grace: &mut Person, d: &SwarmDelivery, stored_at: u64| {
        grace
            .core
            .receive_public_entry(&follow, &d.envelope, stored_at, later)
            .unwrap()
    };
    assert_eq!(take_at(&mut grace, &post, NOW + 10), PublicEntry::Post);
    assert_eq!(
        take_at(&mut grace, &earlier, NOW - 60),
        PublicEntry::Ignored
    );
}

#[test]
fn removed_subscribers_read_nothing_new_and_the_rest_follow_each_new_key() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let mut frank = person("Frank");
    let c = closed(&mut alice, &mut bob, Access::Private);
    let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
    // Frank hangs next to Dave, in Alice's branch; Bob lets Erin in, and
    // his note tells Alice where her keys hang.
    subscribe(&mut alice, &c, &mut frank, "s-2");
    subscribe(&mut bob, &c, &mut erin, "s-3");
    let notice = pending_in(&mut bob, &c);
    read(&mut alice, &c, &notice).unwrap();
    let key0 = dave.core.public_mailbox(&follow, NOW).unwrap().unwrap();
    let before = say(&mut alice, &c, "while we are all together", "p-0");
    let removal = unsubscribe(&erin);
    change(&mut alice, &mut [&mut bob], &c, removal, "u-1");
    // Only the committer publishes; the same document, under the same
    // stamp, until its copy is read back.
    assert!(bob.core.channel_key_docs(&c, NOW).unwrap().is_empty());
    let spent = used(&alice);
    let first = alice.core.channel_key_docs(&c, NOW).unwrap();
    assert_eq!(
        alice.core.channel_key_docs(&c, NOW).unwrap(),
        first,
        "handed out again"
    );
    assert_eq!(used(&alice), spent + 1);
    let docs = key_docs(&mut alice, &c);
    assert_eq!(docs, first);
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].mailbox, key0, "in the mailbox of the key before");
    assert!(
        docs[0].envelope.len() <= 4 * 1024,
        "{} B",
        docs[0].envelope.len()
    );
    for (p, outcome) in [
        (&mut dave, PublicEntry::Rekeyed),
        (&mut frank, PublicEntry::Rekeyed),
        (&mut erin, PublicEntry::Ignored),
    ] {
        assert_eq!(take(p, &follow, &docs[0]), outcome);
    }
    let key1 = dave.core.public_mailbox(&follow, NOW).unwrap().unwrap();
    assert_ne!(key1, key0);
    let after = say(&mut bob, &c, "already without Erin", "p-1");
    assert_eq!(after.mailbox, key1);
    assert_eq!(take(&mut dave, &follow, &after), PublicEntry::Post);
    assert_eq!(take(&mut erin, &follow, &after), PublicEntry::Ignored);
    assert_eq!(erin.core.public_mailbox(&follow, NOW).unwrap(), Some(key0));
    // What was sealed before still opens for those who had its key.
    for p in [&mut dave, &mut erin] {
        assert_eq!(take(p, &follow, &before), PublicEntry::Post);
    }
    // Frank, Dave's neighbour, goes next: the keys they shared move again.
    let removal = unsubscribe(&frank);
    change(&mut alice, &mut [&mut bob], &c, removal, "u-2");
    let docs = key_docs(&mut alice, &c);
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].mailbox, key1);
    assert_eq!(take(&mut dave, &follow, &docs[0]), PublicEntry::Rekeyed);
    assert_eq!(take(&mut frank, &follow, &docs[0]), PublicEntry::Ignored);
    // Bob, who only read the commit, derives the new key himself.
    let last = say(&mut bob, &c, "already without Frank", "p-2");
    assert_ne!(last.mailbox, key1);
    assert_eq!(take(&mut dave, &follow, &last), PublicEntry::Post);
    assert_eq!(take(&mut frank, &follow, &last), PublicEntry::Ignored);
}

#[test]
fn a_ban_with_a_reseed_gives_the_rest_new_keys_and_leaves_the_banned_out() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let mut frank = person("Frank");
    let c = closed(&mut alice, &mut bob, Access::Private);
    let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
    subscribe(&mut alice, &c, &mut erin, "s-2");
    subscribe(&mut alice, &c, &mut frank, "s-3");
    let old = dave.core.public_mailbox(&follow, NOW).unwrap().unwrap();
    let reseed = GroupChange {
        ban: vec![frank.id.clone()],
        reseed: true,
        ..GroupChange::default()
    };
    change(&mut alice, &mut [&mut bob], &c, reseed, "r-1");
    let docs = key_docs(&mut alice, &c);
    assert!(!docs.is_empty());
    for doc in &docs {
        assert_eq!(doc.mailbox, old);
        assert_eq!(take(&mut frank, &follow, doc), PublicEntry::Ignored);
    }
    for p in [&mut dave, &mut erin] {
        let taken: Vec<PublicEntry> = docs.iter().map(|doc| take(p, &follow, doc)).collect();
        assert!(taken.contains(&PublicEntry::Rekeyed), "{taken:?}");
    }
    let post = say(&mut bob, &c, "after the reseed", "p-1");
    assert_ne!(post.mailbox, old);
    for p in [&mut dave, &mut erin] {
        assert_eq!(
            p.core.public_mailbox(&follow, NOW).unwrap(),
            Some(post.mailbox)
        );
        assert_eq!(take(p, &follow, &post), PublicEntry::Post);
    }
    assert_eq!(take(&mut frank, &follow, &post), PublicEntry::Ignored);
    // Banned, nobody gives him keys again.
    assert!(matches!(
        alice
            .core
            .channel_subscribe(&c, &invitee(&mut frank), "s-4", NOW),
        Err(CoreError::Banned)
    ));
}

#[test]
fn an_admin_removed_from_a_closed_channels_team_writes_for_nobody() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut dave = person("Dave");
    let c = closed(&mut alice, &mut bob, Access::Private);
    let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
    // Bob writes, and is removed before his post reaches Dave.
    let held = say(&mut bob, &c, "I am still here", "p-1");
    let without_bob = remove(&bob);
    change(&mut alice, &mut [&mut bob], &c, without_bob, "c-1");
    // The roster reaches subscribers sealed, like a post.
    let roster = public_roster(&mut alice, &c);
    assert_eq!(
        Some(roster.mailbox),
        dave.core.public_mailbox(&follow, NOW).unwrap()
    );
    assert_eq!(take(&mut dave, &follow, &roster), PublicEntry::Roster);
    assert_eq!(take(&mut dave, &follow, &held), PublicEntry::Ignored);
    let from_alice = say(&mut alice, &c, "Bob does not write anymore", "p-2");
    assert_eq!(take(&mut dave, &follow, &from_alice), PublicEntry::Post);
}

/// A post queued by a writer removed before it went out goes nowhere: a
/// closed channel's posts are never sent in the clear.
#[test]
fn a_closed_channels_post_goes_nowhere_once_its_writer_is_out() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let c = closed(&mut alice, &mut bob, Access::Private);
    bob.core
        .send_message(&c, "in the queue", "p-1", NOW)
        .unwrap();
    let queued = bob
        .core
        .swarm_outbox(8)
        .unwrap()
        .into_iter()
        .rfind(|item| item.conversation_id == c)
        .unwrap();
    let without_bob = remove(&bob);
    change(&mut alice, &mut [&mut bob], &c, without_bob, "c-1");
    assert!(
        bob.core
            .prepare_swarm_delivery(&queued.message_id, NOW)
            .is_err()
    );
}

/// Keys sent again after they moved on — a retried delivery, a replay —
/// do not turn a subscriber back to them.
#[test]
fn old_keys_sent_again_do_not_turn_a_subscriber_back() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let c = closed(&mut alice, &mut bob, Access::Private);
    let card = invitee(&mut dave);
    alice.core.channel_subscribe(&c, &card, "s-1", NOW).unwrap();
    let keys = alice
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .rfind(|w| w.destination == dave.id)
        .unwrap()
        .wire;
    deliver_invites(&mut alice, &mut [&mut dave]);
    let follow = subscribe(&mut alice, &c, &mut erin, "s-2");
    let removal = unsubscribe(&erin);
    change(&mut alice, &mut [&mut bob], &c, removal, "u-1");
    for doc in key_docs(&mut alice, &c) {
        take(&mut dave, &follow, &doc);
    }
    let moved = dave.core.public_mailbox(&follow, NOW).unwrap();
    dave.core.receive(&keys, NOW).unwrap();
    assert_eq!(dave.core.public_mailbox(&follow, NOW).unwrap(), moved);
    let post = say(&mut alice, &c, "after the key change", "p-1");
    assert_eq!(take(&mut dave, &follow, &post), PublicEntry::Post);
}

/// `p`'s document of its own key for the channel it follows as `follow`,
/// read back from the mailbox, as its node does once it stored it there:
/// the delivery.
fn own_key_published(p: &mut Person, follow: &str) -> SwarmDelivery {
    let hellos = p.core.channel_hellos(NOW).unwrap();
    let [hello] = hellos.as_slice() else {
        panic!("one key of its own to publish: {}", hellos.len());
    };
    // The same, under the same stamp, until read back.
    assert_eq!(p.core.channel_hellos(NOW).unwrap(), hellos);
    assert_eq!(hello.conversation_id, follow);
    assert_eq!(
        p.core.public_mailbox(follow, NOW).unwrap(),
        Some(hello.mailbox)
    );
    take(p, follow, hello);
    assert!(p.core.channel_hellos(NOW).unwrap().is_empty());
    hello.clone()
}

/// Deliver `committer`'s newest pending commit in `c` to `readers`.
fn deliver_commit(committer: &mut Person, readers: &mut [&mut Person], c: &str) {
    let d = pending_in(committer, c);
    for p in readers.iter_mut() {
        let _ = read(p, c, &d);
    }
}

/// The owner's reseed onto subscribers' own keys, read and chosen by
/// `readers`, and shown to `outsiders`, who are no longer in; its commit.
fn hard_reseed(
    owner: &mut Person,
    readers: &mut [&mut Person],
    outsiders: &mut [&mut Person],
    c: &str,
) -> String {
    let e = epoch(owner, c);
    let commit = owner
        .core
        .channel_hard_reseed(c, NOW)
        .unwrap()
        .expect("a reseed is due")
        .commit;
    let d = pending_in(owner, c);
    for p in readers.iter_mut() {
        read(p, c, &d).unwrap();
    }
    for p in outsiders.iter_mut() {
        let _ = read(p, c, &d);
    }
    owner.core.decide_group_commit(c, e, &commit, NOW).unwrap();
    decide_everywhere(readers, c, e, &commit);
    commit
}

/// The key tree `p`'s MLS state holds for group `c`.
fn tree_at(p: &Person, c: &str) -> agentic_protocol::group::KeyTree {
    let group: [u8; 32] = hex::decode(c).unwrap().try_into().unwrap();
    let data = super::super::profile_db::mls(&p._dir)
        .group_data(group)
        .unwrap()
        .unwrap();
    Bans::decode(&data).unwrap().tree.unwrap()
}

/// A channel of Alice with Bob and Carol in its team.
fn team_of_three(alice: &mut Person, bob: &mut Person, carol: &mut Person) -> String {
    let team = vec![invitee(bob), invitee(carol)];
    let c = alice
        .core
        .create_channel("Club", &team, Access::Private, "ch-1", NOW)
        .unwrap()
        .id;
    deliver_invites(alice, &mut [bob, carol]);
    c
}

#[test]
fn taking_an_admin_off_the_team_moves_the_channel_to_keys_the_admin_never_learns() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let mut frank = person("Frank");
    let mut grace = person("Grace");
    let c = team_of_three(&mut alice, &mut bob, &mut carol);
    // Dave's keys come from Alice; each subscriber publishes a key of its
    // own, paid by itself, sealed in the channel's mailbox.
    let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
    let before = used(&dave);
    let dave_key = own_key_published(&mut dave, &follow);
    assert_eq!(used(&dave), before + 1);
    // Erin's come from Carol: Alice reads Erin's key before Carol's note of
    // where Erin hangs.
    subscribe(&mut carol, &c, &mut erin, "s-2");
    let erin_key = own_key_published(&mut erin, &follow);
    for key in [&dave_key, &erin_key] {
        take(&mut alice, &c, key);
    }
    let notice = pending_in(&mut carol, &c);
    for p in [&mut alice, &mut bob] {
        read(p, &c, &notice).unwrap();
    }
    // Grace gave her key too, and was taken out since.
    subscribe(&mut alice, &c, &mut grace, "s-3");
    let grace_key = own_key_published(&mut grace, &follow);
    take(&mut alice, &c, &grace_key);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &c,
        unsubscribe(&grace),
        "u-1",
    );
    for doc in key_docs(&mut alice, &c) {
        for p in [&mut dave, &mut erin] {
            take(p, &follow, &doc);
        }
    }
    // Frank lets no stranger in: Carol's keys wait for him, and so does his
    // own key.
    door::manual(&mut frank);
    carol
        .core
        .channel_subscribe(&c, &invitee(&mut frank), "s-4", NOW)
        .unwrap();
    let outcomes = deliver_invites(&mut carol, &mut [&mut frank]);
    assert!(
        matches!(outcomes.as_slice(), [IntroOutcome::Pending(_)]),
        "{outcomes:?}"
    );
    let notice = pending_in(&mut carol, &c);
    read(&mut alice, &c, &notice).unwrap();
    let old = dave.core.public_mailbox(&follow, NOW).unwrap().unwrap();
    let old_tree = tree_at(&alice, &c);
    // A subscriber taken out is no reason for new keys all round.
    assert!(alice.core.channel_hard_reseed(&c, NOW).unwrap().is_none());

    // Carol leaves the team. The commit that took her out carries the seed
    // she knew.
    let without_carol = remove(&carol);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &c,
        without_carol,
        "c-1",
    );
    assert_eq!(tree_at(&alice, &c).seed, old_tree.seed);
    // Nothing is due at Bob's, who is not the owner.
    assert!(bob.core.channel_hard_reseed(&c, NOW).unwrap().is_none());
    // Alice's reseed waits for the notary: asked again, nothing more. Bob's
    // commit of the same epoch wins; hers is due still, and made again on
    // top of his.
    let e = epoch(&alice, &c);
    alice
        .core
        .channel_hard_reseed(&c, NOW)
        .unwrap()
        .expect("due");
    assert!(alice.core.channel_hard_reseed(&c, NOW).unwrap().is_none());
    deliver_commit(&mut alice, &mut [&mut bob], &c);
    let bobs = bob
        .core
        .change_group(&c, unsubscribe(&erin), "u-2", NOW)
        .unwrap()
        .commit;
    deliver_commit(&mut bob, &mut [&mut alice], &c);
    decide_everywhere(&mut [&mut alice, &mut bob], &c, e, &bobs);
    for doc in key_docs(&mut bob, &c) {
        take(&mut dave, &follow, &doc);
    }
    hard_reseed(&mut alice, &mut [&mut bob], &mut [&mut carol], &c);
    assert!(alice.core.channel_hard_reseed(&c, NOW).unwrap().is_none());
    let new_tree = tree_at(&alice, &c);
    assert_eq!(new_tree.generation, old_tree.generation + 1);
    assert_ne!(new_tree.seed, old_tree.seed);
    assert_eq!(
        tree_at(&bob, &c).seed,
        new_tree.seed,
        "Bob derives the new keys"
    );
    // Carol, shown that commit, still holds the seed she knew.
    assert_eq!(tree_at(&carol, &c).seed, old_tree.seed);

    // Its keys go in the mailbox of the key before; Dave moves on, Erin —
    // taken out by Bob — and Grace — taken out before — do not, though
    // Alice had their keys.
    let docs = key_docs(&mut alice, &c);
    assert!(!docs.is_empty());
    for doc in &docs {
        assert_ne!(doc.mailbox, old, "the key before is Bob's commit's");
    }
    let moved: Vec<PublicEntry> = docs
        .iter()
        .map(|doc| take(&mut dave, &follow, doc))
        .collect();
    assert!(moved.contains(&PublicEntry::Rekeyed), "{moved:?}");
    for p in [&mut erin, &mut grace] {
        let moved: Vec<PublicEntry> = docs.iter().map(|doc| take(p, &follow, doc)).collect();
        assert!(!moved.contains(&PublicEntry::Rekeyed), "{moved:?}");
    }
    // The key of his own counted: nothing more to publish.
    assert!(dave.core.channel_hellos(NOW).unwrap().is_empty());
    let post = say(&mut bob, &c, "already without Carol", "p-1");
    assert_eq!(
        dave.core.public_mailbox(&follow, NOW).unwrap(),
        Some(post.mailbox)
    );
    assert_eq!(take(&mut dave, &follow, &post), PublicEntry::Post);
    for p in [&mut erin, &mut grace] {
        assert_eq!(take(p, &follow, &post), PublicEntry::Ignored);
    }

    // Frank gave no key of his own: his keys are given again, of the new
    // seed; he takes them and reads on.
    assert_eq!(
        alice.core.channel_reissues(&c).unwrap(),
        vec![frank.id.clone()]
    );
    alice
        .core
        .channel_subscribe(&c, &invitee(&mut frank), "again-1", NOW)
        .unwrap();
    assert!(alice.core.channel_reissues(&c).unwrap().is_empty());
    let outcomes = deliver_invites(&mut alice, &mut [&mut frank]);
    let [IntroOutcome::Pending(request)] = outcomes.as_slice() else {
        panic!("{outcomes:?}");
    };
    frank
        .core
        .accept_intro_request(&request.request_id, NOW)
        .unwrap();
    assert_eq!(
        frank.core.public_mailbox(&follow, NOW).unwrap(),
        Some(post.mailbox)
    );
    assert_eq!(take(&mut frank, &follow, &post), PublicEntry::Post);
}

/// What makes new keys onto subscribers' own due at the owner: a team
/// member out. Not a commit that takes the member out and reseeds at once,
/// nor a reseed under the old keys after it, whose seed the former member
/// cannot read but whose keys it opens: both leave it due.
#[test]
fn only_a_commit_after_the_admins_removal_and_onto_subscribers_keys_ends_what_is_due() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let c = team_of_three(&mut alice, &mut bob, &mut carol);
    let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
    // Bob learns where Dave hangs: his reseed gives Dave keys too.
    let notice = pending_in(&mut alice, &c);
    read(&mut bob, &c, &notice).unwrap();
    let key = own_key_published(&mut dave, &follow);
    take(&mut alice, &c, &key);
    let out_and_reseeded = GroupChange {
        remove: vec![carol.id.clone()],
        reseed: true,
        ..GroupChange::default()
    };
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &c,
        out_and_reseeded,
        "c-1",
    );
    for doc in key_docs(&mut alice, &c) {
        take(&mut dave, &follow, &doc);
    }
    // A reseed under the old keys moves the mailbox the team reads: Dave
    // publishes his key again, once, under the new keys.
    let again = own_key_published(&mut dave, &follow);
    take(&mut alice, &c, &again);
    // Bob reseeds under the old keys: still due.
    let reseed = GroupChange {
        reseed: true,
        ..GroupChange::default()
    };
    change(&mut bob, &mut [&mut alice], &c, reseed, "r-1");
    for doc in key_docs(&mut bob, &c) {
        take(&mut dave, &follow, &doc);
    }
    let again = own_key_published(&mut dave, &follow);
    take(&mut alice, &c, &again);
    let generation = tree_at(&alice, &c).generation;
    hard_reseed(&mut alice, &mut [&mut bob], &mut [&mut carol], &c);
    assert_eq!(tree_at(&alice, &c).generation, generation + 1);
    let moved: Vec<PublicEntry> = key_docs(&mut alice, &c)
        .iter()
        .map(|doc| take(&mut dave, &follow, doc))
        .collect();
    assert!(moved.contains(&PublicEntry::Rekeyed), "{moved:?}");
    assert!(alice.core.channel_hard_reseed(&c, NOW).unwrap().is_none());
}

/// A subscriber whose key of its own the owner never read gets nothing
/// from the new keys — though it holds every key of its path the former
/// admin knows, and a neighbour's keys moved on — and has its keys given
/// again, of the new seed, once; it then publishes its key anew.
#[test]
fn a_subscriber_whose_key_the_owner_never_read_gets_its_keys_again() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let c = team_of_three(&mut alice, &mut bob, &mut carol);
    let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
    subscribe(&mut alice, &c, &mut erin, "s-2");
    // Dave's key never reaches Alice; Erin's, his neighbour's, does.
    own_key_published(&mut dave, &follow);
    let erin_key = own_key_published(&mut erin, &follow);
    take(&mut alice, &c, &erin_key);
    let without_carol = remove(&carol);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &c,
        without_carol,
        "c-1",
    );
    hard_reseed(&mut alice, &mut [&mut bob], &mut [&mut carol], &c);
    let docs = key_docs(&mut alice, &c);
    let moved: Vec<PublicEntry> = docs
        .iter()
        .map(|doc| take(&mut erin, &follow, doc))
        .collect();
    assert!(moved.contains(&PublicEntry::Rekeyed), "{moved:?}");
    let moved: Vec<PublicEntry> = docs
        .iter()
        .map(|doc| take(&mut dave, &follow, doc))
        .collect();
    assert!(!moved.contains(&PublicEntry::Rekeyed), "{moved:?}");
    let post = say(&mut bob, &c, "after the new keys", "p-1");
    assert_eq!(take(&mut dave, &follow, &post), PublicEntry::Ignored);
    assert_eq!(take(&mut erin, &follow, &post), PublicEntry::Post);
    // Given again: Dave follows the channel already and takes them.
    assert_eq!(
        alice.core.channel_reissues(&c).unwrap(),
        vec![dave.id.clone()]
    );
    let outcomes = {
        alice
            .core
            .channel_subscribe(&c, &invitee(&mut dave), "again-1", NOW)
            .unwrap();
        deliver_invites(&mut alice, &mut [&mut dave])
    };
    assert!(
        matches!(outcomes.as_slice(), [IntroOutcome::Subscribed(id)] if *id == follow),
        "{outcomes:?}"
    );
    assert_eq!(take(&mut dave, &follow, &post), PublicEntry::Post);
    // The reseed's documents read late move him nowhere.
    let root = dave.core.public_mailbox(&follow, NOW).unwrap();
    for doc in &docs {
        assert_eq!(take(&mut dave, &follow, doc), PublicEntry::Ignored);
    }
    assert_eq!(dave.core.public_mailbox(&follow, NOW).unwrap(), root);
    // His key goes out again, once, under the new keys.
    let hello = own_key_published(&mut dave, &follow);
    assert_eq!(Some(hello.mailbox), root);
}

// --- the channel's crypto, written here apart from the core's ------------

fn hkdf(salt: &[u8], ikm: &[u8], info: &[u8]) -> [u8; 32] {
    let mut out = [0; 32];
    hkdf::Hkdf::<sha2::Sha256>::new(Some(salt), ikm)
        .expand(info, &mut out)
        .unwrap();
    out
}

/// The key of `node` of `generation` at `version` from `seed`.
fn node_key(
    seed: &[u8; 32],
    group: &[u8; 32],
    generation: u32,
    node: u64,
    version: u32,
) -> [u8; 32] {
    let info = [
        &group[..],
        &generation.to_be_bytes(),
        &node.to_be_bytes(),
        &version.to_be_bytes(),
    ]
    .concat();
    hkdf(b"AIN_CHANNEL_KEY_V1", seed, &info)
}

/// The channel key of `generation` at `version` from `seed`.
fn channel_root(seed: &[u8; 32], group: &[u8; 32], generation: u32, version: u32) -> [u8; 32] {
    node_key(seed, group, generation, 0, version)
}

/// A subscriber's key document as it goes in the channel's mailbox: `0x06
/// ‖ leaf ‖ generation ‖ nonce ‖ AEAD` under a key from the leaf's key, so
/// that only the team and the leaf's holder read who signed it.
fn hello(seed: &[u8; 32], group: &[u8; 32], generation: u32, leaf: u32, doc: &[u8]) -> Vec<u8> {
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    use sha2::Digest;
    let leaf_key = node_key(seed, group, generation, (32u64 << 32) | u64::from(leaf), 0);
    let key = hkdf(
        b"AIN_CHANNEL_V1",
        &leaf_key,
        &[&b"hello"[..], group].concat(),
    );
    let nonce: [u8; 32] = sha2::Sha256::digest([&key[..], doc].concat()).into();
    let aad = [
        &b"AIN_CHANNEL_HELLO_V1"[..],
        group,
        &generation.to_be_bytes(),
        &leaf.to_be_bytes(),
    ]
    .concat();
    let sealed = chacha20poly1305::ChaCha20Poly1305::new_from_slice(&key)
        .unwrap()
        .encrypt(
            chacha20poly1305::Nonce::from_slice(&nonce[..12]),
            Payload {
                msg: doc,
                aad: &aad,
            },
        )
        .unwrap();
    [
        &[6][..],
        &leaf.to_be_bytes(),
        &generation.to_be_bytes(),
        &nonce[..12],
        &sealed,
    ]
    .concat()
}

/// `inner` sealed under channel key `root`: `0x04 ‖ sealed-v1`.
fn seal(root: &[u8; 32], group: &[u8; 32], generation: u32, version: u32, inner: &[u8]) -> Vec<u8> {
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    use sha2::Digest;
    let key = hkdf(b"AIN_CHANNEL_V1", root, &[&b"entries"[..], group].concat());
    let nonce: [u8; 32] = sha2::Sha256::digest([&key[..], inner].concat()).into();
    let aad = [
        &b"AIN_CHANNEL_ENTRY_V1"[..],
        group,
        &generation.to_be_bytes(),
        &version.to_be_bytes(),
    ]
    .concat();
    let sealed = chacha20poly1305::ChaCha20Poly1305::new_from_slice(&key)
        .unwrap()
        .encrypt(
            chacha20poly1305::Nonce::from_slice(&nonce[..12]),
            Payload {
                msg: inner,
                aad: &aad,
            },
        )
        .unwrap();
    let mut e = minicbor::Encoder::new(vec![4]);
    e.array(6).unwrap();
    e.str("sealed-v1").unwrap();
    e.bytes(group).unwrap();
    e.u32(generation).unwrap();
    e.u32(version).unwrap();
    e.bytes(&nonce[..12]).unwrap();
    e.bytes(&sealed).unwrap();
    e.into_writer()
}

/// `p`'s root signing key, as its database keeps it.
fn signing_key(p: &Person) -> SigningKey {
    let seed: Vec<u8> = super::super::profile_db::encrypted_db(&p._dir)
        .query_row(
            "SELECT owner_seed FROM profile WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    SigningKey::from_bytes(&seed.try_into().unwrap())
}

/// A subscriber's key of its own counts for its own leaf alone: signed by
/// one subscriber for another's leaf, it is not used, and that other
/// subscriber's keys are given again; a key that is no key (all zeros) is
/// passed over without holding up the rest. (The same document for its own
/// leaf, sealed the same way, is taken: the crypto here is the core's.)
#[test]
fn a_subscribers_key_counts_for_its_own_leaf_alone() {
    use agentic_protocol::group::SubscriberKey;
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let mut frank = person("Frank");
    let c = team_of_three(&mut alice, &mut bob, &mut carol);
    subscribe(&mut alice, &c, &mut dave, "s-1");
    subscribe(&mut alice, &c, &mut erin, "s-2");
    subscribe(&mut alice, &c, &mut frank, "s-3");
    let gref = reference(&alice, &c);
    let tree = tree_at(&alice, &c);
    let root = channel_root(&tree.seed, &gref, tree.generation, 0);
    // Alice gives from branch 0: Dave hangs on leaf 0, Erin on 1, Frank on 2.
    let claim = |signer: &Person, leaf: u32, key: [u8; 32]| {
        let body = SubscriberKey {
            group: gref,
            leaf,
            key,
        }
        .encode();
        let doc = signed(DocumentKind::ChannelSubscriber, body, &signing_key(signer));
        let inner = hello(&tree.seed, &gref, tree.generation, leaf, &doc);
        seal(&root, &gref, tree.generation, 0, &inner)
    };
    let outcomes: Vec<PublicEntry> = [
        claim(&erin, 1, [51; 32]),
        claim(&erin, 0, [50; 32]),
        claim(&frank, 2, [0; 32]),
    ]
    .iter()
    .map(|entry| {
        alice
            .core
            .receive_public_entry(&c, entry, NOW, NOW)
            .unwrap()
    })
    .collect();
    assert_eq!(outcomes[0], PublicEntry::SubscriberKey);
    assert_eq!(outcomes[2], PublicEntry::Ignored, "no key at all");
    let without_carol = remove(&carol);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &c,
        without_carol,
        "c-1",
    );
    hard_reseed(&mut alice, &mut [&mut bob], &mut [&mut carol], &c);
    assert!(!key_docs(&mut alice, &c).is_empty());
    // Erin's own leaf had a key; Dave's none of his, Frank's none at all.
    let mut again = alice.core.channel_reissues(&c).unwrap();
    again.sort();
    let mut expected = vec![dave.id.clone(), frank.id.clone()];
    expected.sort();
    assert_eq!(again, expected);
}

/// An admin taken off the team that also held keys as a subscriber reads
/// nothing new: its own leaves are neither sealed to — whether the owner
/// had its key or not — nor given again, at that reseed or any after it.
#[test]
fn an_admin_taken_off_the_team_loses_the_keys_it_held_as_a_subscriber_too() {
    for with_key in [false, true] {
        let mut alice = person("Alice");
        let mut bob = person("Bob");
        let mut carol = person("Carol");
        let mut dave = person("Dave");
        let c = team_of_three(&mut alice, &mut bob, &mut carol);
        let follow = subscribe(&mut alice, &c, &mut dave, "s-1");
        subscribe(&mut alice, &c, &mut carol, "s-2");
        let key = own_key_published(&mut dave, &follow);
        take(&mut alice, &c, &key);
        if with_key {
            let key = own_key_published(&mut carol, &follow);
            take(&mut alice, &c, &key);
        }
        let without_carol = remove(&carol);
        change(
            &mut alice,
            &mut [&mut bob, &mut carol],
            &c,
            without_carol,
            "c-1",
        );
        hard_reseed(&mut alice, &mut [&mut bob], &mut [&mut carol], &c);
        let docs = key_docs(&mut alice, &c);
        let moved: Vec<PublicEntry> = docs
            .iter()
            .map(|doc| take(&mut carol, &follow, doc))
            .collect();
        assert!(
            !moved.contains(&PublicEntry::Rekeyed),
            "{with_key}: {moved:?}"
        );
        let moved: Vec<PublicEntry> = docs
            .iter()
            .map(|doc| take(&mut dave, &follow, doc))
            .collect();
        assert!(moved.contains(&PublicEntry::Rekeyed), "{moved:?}");
        assert!(
            alice.core.channel_reissues(&c).unwrap().is_empty(),
            "{with_key}"
        );
        let post = say(&mut bob, &c, "without Carol", "p-1");
        assert_eq!(take(&mut carol, &follow, &post), PublicEntry::Ignored);
        assert_eq!(take(&mut dave, &follow, &post), PublicEntry::Post);
        // Bob leaves too: the next reseed gives Carol nothing either.
        let without_bob = remove(&bob);
        change(&mut alice, &mut [&mut bob], &c, without_bob, "c-2");
        hard_reseed(&mut alice, &mut [], &mut [&mut bob, &mut carol], &c);
        let docs = key_docs(&mut alice, &c);
        let moved: Vec<PublicEntry> = docs
            .iter()
            .map(|doc| take(&mut carol, &follow, doc))
            .collect();
        assert!(
            !moved.contains(&PublicEntry::Rekeyed),
            "{with_key}: {moved:?}"
        );
        let moved: Vec<PublicEntry> = docs
            .iter()
            .map(|doc| take(&mut dave, &follow, doc))
            .collect();
        assert!(moved.contains(&PublicEntry::Rekeyed), "{moved:?}");
        assert!(
            alice.core.channel_reissues(&c).unwrap().is_empty(),
            "{with_key}"
        );
    }
}
