//! A group's door (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 5): a
//! public group or one by request publishes a door card; anyone who knows
//! the group knocks with a sealed, paid application; the owner and admins
//! let applicants in with a batch commit — at once in a public group, after
//! a decision in one by request. An applicant takes the invitation of a
//! group it knocked on whatever its own contact policy.
use super::*;
use agentic_core::{DoorEntry, DoorRequest};

/// A group of Alice (owner), Bob and Carol, opened to `to`.
fn door_group(alice: &mut Person, bob: &mut Person, carol: &mut Person, to: Access) -> String {
    let g = team(alice, bob, carol);
    change(alice, &mut [bob, carol], &g, access(to), "door");
    g
}

/// `admin`'s door card for the current period, as it publishes it.
pub(super) fn door_card(admin: &mut Person, group: &str) -> SwarmDelivery {
    admin
        .core
        .door_card(group, NOW)
        .unwrap()
        .expect("an owner or admin of a group with a door publishes its card")
}

/// `who` knocks at the door of the group of `card`: its paid application,
/// as delivered into the door mailbox.
pub(super) fn knock(
    who: &mut Person,
    gref: &[u8; 32],
    card: &SwarmDelivery,
    note: &str,
) -> SwarmDelivery {
    let used = who.core.mailbox_books().unwrap()[0].used;
    let applied = who
        .core
        .apply_to_group(gref, &card.envelope, note, &format!("knock-{note}"), NOW)
        .unwrap();
    let d = who
        .core
        .prepare_swarm_delivery(&applied.message_id, NOW)
        .unwrap();
    assert_eq!(d.mailbox, card.mailbox, "an application goes to the door");
    assert_eq!(who.core.mailbox_books().unwrap()[0].used, used + 1);
    d
}

fn requests(p: &Person, group: &str) -> Vec<DoorRequest> {
    p.core.door_requests(group).unwrap()
}

/// The commit a batch made, read and chosen by `readers` too.
pub(super) fn batch_in(
    admin: &mut Person,
    readers: &mut [&mut Person],
    group: &str,
    at: u64,
) -> String {
    let e = epoch(admin, group);
    let made = admin
        .core
        .door_batch(group, at)
        .unwrap()
        .expect("a batch of those let in");
    let d = admin
        .core
        .prepare_swarm_delivery(&made.message_id, NOW)
        .unwrap();
    for p in readers.iter_mut() {
        read(p, group, &d).unwrap();
    }
    admin
        .core
        .decide_group_commit(group, e, &made.commit, NOW)
        .unwrap();
    decide_everywhere(readers, group, e, &made.commit);
    made.commit
}

#[test]
fn anyone_knocking_at_a_public_groups_door_is_let_in_by_the_next_batch_whatever_its_policy() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = door_group(&mut alice, &mut bob, &mut carol, Access::Public);
    let gref = reference(&alice, &g);
    // A plain member has no door to publish.
    assert!(bob.core.door_card(&g, NOW).unwrap().is_none());
    let card = door_card(&mut alice, &g);
    assert_eq!(card.mailbox, dave.core.door_mailbox(&gref, NOW));
    let door = dave
        .core
        .open_door_card(&gref, &card.envelope, NOW)
        .unwrap();
    assert_eq!(
        (
            door.group_id.as_str(),
            door.name.as_str(),
            door.access.as_str()
        ),
        (g.as_str(), "Team", "public")
    );
    assert_eq!(door.owner, alice.id);

    // Dave lets no stranger in by himself; a group he knocked on is another
    // matter.
    dave.core
        .set_intro_policy(IntroPolicy {
            mode: IntroMode::Manual,
            daily_limit: 20,
            allowed: vec![],
        })
        .unwrap();
    let application = knock(&mut dave, &gref, &card, "I want to join you");
    assert_eq!(
        alice
            .core
            .receive_door_entry(&g, application.period, &application.envelope, NOW)
            .unwrap(),
        DoorEntry::Admitted(dave.id.clone())
    );
    // Read again from another holder: nothing new.
    assert_eq!(
        alice
            .core
            .receive_door_entry(&g, application.period, &application.envelope, NOW)
            .unwrap(),
        DoorEntry::Ignored
    );
    assert!(requests(&alice, &g).is_empty());
    let e = epoch(&alice, &g);
    let made = alice.core.door_batch(&g, NOW).unwrap().unwrap();
    // Asked again within the minute, the batch is the same commit.
    assert_eq!(
        alice.core.door_batch(&g, NOW).unwrap().unwrap().commit,
        made.commit
    );
    let d = alice
        .core
        .prepare_swarm_delivery(&made.message_id, NOW)
        .unwrap();
    for p in [&mut bob, &mut carol] {
        read(p, &g, &d).unwrap();
    }
    alice
        .core
        .decide_group_commit(&g, e, &made.commit, NOW)
        .unwrap();
    decide_everywhere(&mut [&mut bob, &mut carol], &g, e, &made.commit);
    let outcomes = deliver_invites(&mut alice, &mut [&mut dave]);
    assert!(
        matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g),
        "{outcomes:?}"
    );
    assert!(dave.core.intro_requests().unwrap().is_empty());
    assert_eq!(dave.core.group(&g).unwrap().members.len(), 4);
    // Everyone let in is in: the next batch has nobody to add.
    assert!(alice.core.door_batch(&g, NOW + 60).unwrap().is_none());
}

#[test]
fn applicants_to_a_group_by_request_wait_for_an_admins_decision_and_its_talk_stays_closed() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let g = door_group(&mut alice, &mut bob, &mut carol, Access::Request);
    let promote = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        promote,
        "bob-admin",
    );
    let gref = reference(&alice, &g);
    // Its talk stays sealed in the group mailbox, its members unpublished.
    for p in [&alice, &bob, &carol] {
        assert_eq!(p.core.group(&g).unwrap().access, "request");
    }
    let hello = say(&mut carol, &g, "members only", "m-1");
    assert_eq!(hello.mailbox, bob.core.group_mailboxes(&g, NOW).unwrap()[0]);
    assert_ne!(
        hello.mailbox,
        public_group_mailbox_id(&DOMAIN, &gref, period(NOW))
    );
    assert!(alice.core.public_roster(&g, NOW).unwrap().is_none());

    // The admin publishes the door, reads it, decides and lets in.
    let card = door_card(&mut bob, &g);
    let door = dave
        .core
        .open_door_card(&gref, &card.envelope, NOW)
        .unwrap();
    assert_eq!(door.access, "request");
    let from_dave = knock(&mut dave, &gref, &card, "I am Dave");
    let from_erin = knock(&mut erin, &gref, &card, "I am Erin");
    for application in [&from_dave, &from_erin] {
        let entry = bob
            .core
            .receive_door_entry(&g, application.period, &application.envelope, NOW)
            .unwrap();
        assert!(matches!(entry, DoorEntry::Waiting(_)), "{entry:?}");
    }
    let waiting = requests(&bob, &g);
    let mut listed: Vec<(String, String, bool)> = waiting
        .iter()
        .map(|r| (r.network_id.clone(), r.note.clone(), r.rejoin))
        .collect();
    listed.sort();
    let mut expected = vec![
        (dave.id.clone(), "I am Dave".to_owned(), false),
        (erin.id.clone(), "I am Erin".to_owned(), false),
    ];
    expected.sort();
    assert_eq!(listed, expected);
    // Nobody is let in before a decision.
    assert!(bob.core.door_batch(&g, NOW).unwrap().is_none());
    let request_of = |id: &str| {
        waiting
            .iter()
            .find(|r| r.network_id == id)
            .unwrap()
            .request_id
            .clone()
    };
    bob.core
        .decide_door_request(&g, &request_of(&erin.id), false, NOW)
        .unwrap();
    bob.core
        .decide_door_request(&g, &request_of(&dave.id), true, NOW)
        .unwrap();
    assert!(requests(&bob, &g).is_empty());
    // Erin's application read again stays refused.
    assert_eq!(
        bob.core
            .receive_door_entry(&g, from_erin.period, &from_erin.envelope, NOW)
            .unwrap(),
        DoorEntry::Ignored
    );
    batch_in(&mut bob, &mut [&mut alice, &mut carol], &g, NOW);
    let outcomes = deliver_invites(&mut bob, &mut [&mut dave, &mut erin]);
    assert!(
        matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g),
        "{outcomes:?}"
    );
    let members = alice.core.group(&g).unwrap().members;
    assert!(members.contains(&dave.id) && !members.contains(&erin.id));

    // A card of another group does not open as this one's door.
    let other = alice
        .core
        .create_group("Another one", &[invitee(&mut carol)], "g-2", NOW)
        .unwrap()
        .id;
    change(
        &mut alice,
        &mut [],
        &other,
        access(Access::Public),
        "open-2",
    );
    let foreign = door_card(&mut alice, &other);
    assert!(
        erin.core
            .open_door_card(&gref, &foreign.envelope, NOW)
            .is_err()
    );
}

#[test]
fn two_admins_batching_in_one_minute_follow_the_notary_and_the_loser_adds_the_rest_later() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let g = door_group(&mut alice, &mut bob, &mut carol, Access::Public);
    let promote = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        promote,
        "bob-admin",
    );
    let gref = reference(&alice, &g);
    let card = door_card(&mut alice, &g);
    let from_dave = knock(&mut dave, &gref, &card, "Dave");
    let from_erin = knock(&mut erin, &gref, &card, "Erin");
    // Alice read both, Bob only Dave's.
    for application in [&from_dave, &from_erin] {
        alice
            .core
            .receive_door_entry(&g, application.period, &application.envelope, NOW)
            .unwrap();
    }
    bob.core
        .receive_door_entry(&g, from_dave.period, &from_dave.envelope, NOW)
        .unwrap();
    let e = epoch(&alice, &g);
    let mine = alice.core.door_batch(&g, NOW).unwrap().unwrap();
    let theirs = bob.core.door_batch(&g, NOW).unwrap().unwrap();
    let from_alice = alice
        .core
        .prepare_swarm_delivery(&mine.message_id, NOW)
        .unwrap();
    let from_bob = bob
        .core
        .prepare_swarm_delivery(&theirs.message_id, NOW)
        .unwrap();
    read(&mut alice, &g, &from_bob).unwrap();
    read(&mut bob, &g, &from_alice).unwrap();
    read(&mut carol, &g, &from_bob).unwrap();
    // The notary names Bob's batch: Alice's is lost.
    assert_eq!(
        alice
            .core
            .decide_group_commit(&g, e, &theirs.commit, NOW)
            .unwrap(),
        CommitDecision::Lost
    );
    decide_everywhere(&mut [&mut bob, &mut carol], &g, e, &theirs.commit);
    deliver_invites(&mut bob, &mut [&mut dave]);
    assert_eq!(dave.core.group(&g).unwrap().epoch, e + 1);
    // At her next pass Alice adds whom Bob's batch left out, and only her:
    // a lost batch leaves its minute free for another.
    let e = e + 1;
    let later = alice.core.door_batch(&g, NOW).unwrap().unwrap();
    assert_ne!(later.commit, mine.commit);
    let d = alice
        .core
        .prepare_swarm_delivery(&later.message_id, NOW)
        .unwrap();
    for p in [&mut bob, &mut carol, &mut dave] {
        read(p, &g, &d).unwrap();
    }
    alice
        .core
        .decide_group_commit(&g, e, &later.commit, NOW)
        .unwrap();
    decide_everywhere(&mut [&mut bob, &mut carol, &mut dave], &g, e, &later.commit);
    let outcomes = deliver_invites(&mut alice, &mut [&mut erin]);
    assert!(
        matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g),
        "{outcomes:?}"
    );
    assert_eq!(alice.core.group(&g).unwrap().members.len(), 5);
    // Erin's application, first read by Bob after she is in: nothing to do.
    assert_eq!(
        bob.core
            .receive_door_entry(&g, from_erin.period, &from_erin.envelope, NOW)
            .unwrap(),
        DoorEntry::Ignored
    );
}

#[test]
fn closing_the_door_drops_who_was_let_in_and_a_banned_id_is_never_let_in() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let mut frank = person("Frank");
    let g = team(&mut alice, &mut bob, &mut carol);
    assert!(alice.core.door_card(&g, NOW).unwrap().is_none());
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        access(Access::Public),
        "open",
    );
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        ban(&[&frank.id]),
        "ban-frank",
    );
    let gref = reference(&alice, &g);
    let card = door_card(&mut alice, &g);
    let from_frank = knock(&mut frank, &gref, &card, "let me in");
    assert_eq!(
        alice
            .core
            .receive_door_entry(&g, from_frank.period, &from_frank.envelope, NOW)
            .unwrap(),
        DoorEntry::Ignored
    );
    // Dave is let in; before the next batch the owner closes the group.
    let from_dave = knock(&mut dave, &gref, &card, "made it");
    assert_eq!(
        alice
            .core
            .receive_door_entry(&g, from_dave.period, &from_dave.envelope, NOW)
            .unwrap(),
        DoorEntry::Admitted(dave.id.clone())
    );
    let from_erin = knock(&mut erin, &gref, &card, "too late");
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        access(Access::Private),
        "close",
    );
    assert!(alice.core.door_card(&g, NOW).unwrap().is_none());
    assert!(alice.core.door_batch(&g, NOW + 60).unwrap().is_none());
    // A knock at the old door read after the closing is not heard.
    assert_eq!(
        alice
            .core
            .receive_door_entry(&g, from_erin.period, &from_erin.envelope, NOW)
            .unwrap(),
        DoorEntry::Ignored
    );
    assert_eq!(alice.core.group(&g).unwrap().members.len(), 3);
    // Those let in were dropped, not kept for a reopening.
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        access(Access::Public),
        "reopen",
    );
    assert!(alice.core.door_batch(&g, NOW + 120).unwrap().is_none());
}

// --- refusals shared, and members back after a long absence ---------------

/// Forty days on: past the thirty the mailboxes keep.
const LATER: u64 = NOW + 40 * DAY;

fn read_at(to: &mut Person, group: &str, d: &SwarmDelivery, at: u64) {
    to.core
        .receive_swarm_envelope(group, d.period, &d.envelope, at)
        .unwrap();
}

/// `from`'s invitation of `to`, read at `at` from `to`'s intro mailbox.
fn invite_at(from: &mut Person, to: &mut Person, at: u64) -> IntroOutcome {
    let id = from
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .rfind(|w| w.destination == to.id)
        .expect("an invitation")
        .message_id;
    let d = from.core.prepare_swarm_delivery(&id, at).unwrap();
    assert_eq!(d.mailbox, to.core.own_intro_mailbox(at).unwrap());
    to.core.receive_intro_envelope(&d.envelope, at).unwrap()
}

/// `admin`'s batch at `at`, applied by `readers` too.
fn batch_at(admin: &mut Person, readers: &mut [&mut Person], group: &str, at: u64) {
    let e = epoch(admin, group);
    let made = admin.core.door_batch(group, at).unwrap().expect("a batch");
    let d = admin
        .core
        .prepare_swarm_delivery(&made.message_id, at)
        .unwrap();
    for p in readers.iter_mut() {
        read_at(p, group, &d, at);
    }
    admin
        .core
        .decide_group_commit(group, e, &made.commit, at)
        .unwrap();
    for p in readers.iter_mut() {
        p.core
            .decide_group_commit(group, e, &made.commit, at)
            .unwrap();
    }
}

pub(super) fn manual(p: &mut Person) {
    p.core
        .set_intro_policy(IntroPolicy {
            mode: IntroMode::Manual,
            daily_limit: 20,
            allowed: vec![],
        })
        .unwrap();
}

#[test]
fn a_refusal_by_one_admin_reaches_the_others_and_no_member_sees_it() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = door_group(&mut alice, &mut bob, &mut carol, Access::Request);
    let promote = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        promote,
        "bob-admin",
    );
    let gref = reference(&alice, &g);
    let card = door_card(&mut bob, &g);
    let from_dave = knock(&mut dave, &gref, &card, "Dave");
    for admin in [&mut alice, &mut bob] {
        admin
            .core
            .receive_door_entry(&g, from_dave.period, &from_dave.envelope, NOW)
            .unwrap();
    }
    let (theirs, hers) = (texts(&carol, &g), texts(&alice, &g));
    let queued: BTreeSet<String> = alice
        .core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .map(|p| p.message_id)
        .collect();
    let request = requests(&alice, &g)[0].request_id.clone();
    alice
        .core
        .decide_door_request(&g, &request, false, NOW)
        .unwrap();
    // Her refusal goes into the group, sealed like its talk.
    let new: Vec<_> = alice
        .core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .filter(|p| p.conversation_id == g && !queued.contains(&p.message_id))
        .collect();
    let [notice] = new.as_slice() else {
        panic!("one notice of the refusal: {new:?}")
    };
    let notice = alice
        .core
        .prepare_swarm_delivery(&notice.message_id, NOW)
        .unwrap();
    assert_eq!(
        notice.mailbox,
        alice.core.group_mailboxes(&g, NOW).unwrap()[0]
    );
    read(&mut bob, &g, &notice).unwrap();
    read(&mut carol, &g, &notice).unwrap();
    // Read again from another holder: nothing more.
    read(&mut bob, &g, &notice).unwrap();
    assert!(requests(&bob, &g).is_empty());
    // And Dave's application, read again at the door, stays refused at Bob's.
    assert_eq!(
        bob.core
            .receive_door_entry(&g, from_dave.period, &from_dave.envelope, NOW)
            .unwrap(),
        DoorEntry::Ignored
    );
    assert_eq!(texts(&carol, &g), theirs, "no member sees it as talk");
    assert_eq!(texts(&alice, &g), hers, "nor its sender");
}

#[test]
fn a_refusal_that_waited_three_epochs_is_sealed_again_and_still_reaches_the_admins() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let erin = person("Erin").id;
    let g = door_group(&mut alice, &mut bob, &mut carol, Access::Request);
    let promote = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        promote,
        "bob-admin",
    );
    let gref = reference(&alice, &g);
    let card = door_card(&mut bob, &g);
    let from_dave = knock(&mut dave, &gref, &card, "Dave");
    for admin in [&mut alice, &mut bob] {
        admin
            .core
            .receive_door_entry(&g, from_dave.period, &from_dave.envelope, NOW)
            .unwrap();
    }
    let queued: BTreeSet<String> = alice
        .core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .map(|p| p.message_id)
        .collect();
    let request = requests(&alice, &g)[0].request_id.clone();
    alice
        .core
        .decide_door_request(&g, &request, false, NOW)
        .unwrap();
    let notice = alice
        .core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .find(|p| p.conversation_id == g && !queued.contains(&p.message_id))
        .expect("a notice of the refusal")
        .message_id;
    // The notice waits while the group moves on: two epochs, left as it is;
    // three, sealed again.
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 2);
    assert!(alice.core.reseal_stale_group_sends().unwrap().is_empty());
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 1);
    assert_eq!(
        alice.core.reseal_stale_group_sends().unwrap(),
        vec![notice.clone()]
    );
    // Dave's application waits in his outbox too: not a group's talk.
    assert!(dave.core.reseal_stale_group_sends().unwrap().is_empty());
    move_on(&mut alice, &mut [&mut bob, &mut carol], &g, &erin, 1);
    let d = alice.core.prepare_swarm_delivery(&notice, NOW).unwrap();
    let theirs = texts(&carol, &g);
    assert_eq!(requests(&bob, &g).len(), 1, "Bob does not know yet");
    read(&mut bob, &g, &d).unwrap();
    read(&mut carol, &g, &d).unwrap();
    assert!(requests(&bob, &g).is_empty(), "Bob learns of the refusal");
    assert_eq!(
        bob.core
            .receive_door_entry(&g, from_dave.period, &from_dave.envelope, NOW)
            .unwrap(),
        DoorEntry::Ignored
    );
    assert_eq!(texts(&carol, &g), theirs, "no member sees it as talk");
}

#[test]
fn a_member_back_after_a_long_absence_knocks_and_takes_its_place_by_the_same_mailbox() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    // By request: a member is let back in without anyone's decision.
    let g = door_group(&mut alice, &mut bob, &mut carol, Access::Request);
    let gref = reference(&alice, &g);
    manual(&mut carol);
    // Carol read the group through today, then was away forty days.
    carol
        .core
        .set_swarm_read_through(&g, period(NOW), NOW)
        .unwrap();
    assert!(carol.core.stale_groups(NOW + DAY).unwrap().is_empty());
    let stale = carol.core.stale_groups(LATER).unwrap();
    assert_eq!(
        stale
            .iter()
            .map(|s| (s.group_id.as_str(), s.group_ref.as_str()))
            .collect::<Vec<_>>(),
        [(g.as_str(), hex::encode(gref).as_str())]
    );
    let mailbox = bob.core.group_mailboxes(&g, LATER).unwrap();
    let card = alice.core.door_card(&g, LATER).unwrap().unwrap();
    let back = carol
        .core
        .rejoin_group(&gref, &card.envelope, "rejoin-1", LATER)
        .unwrap();
    let d = carol
        .core
        .prepare_swarm_delivery(&back.message_id, LATER)
        .unwrap();
    assert_eq!(d.mailbox, card.mailbox);
    // A member asking for its place again is let in, no question asked.
    assert_eq!(
        alice
            .core
            .receive_door_entry(&g, d.period, &d.envelope, LATER)
            .unwrap(),
        DoorEntry::Admitted(carol.id.clone())
    );
    assert!(requests(&alice, &g).is_empty());
    let e = epoch(&alice, &g);
    batch_at(&mut alice, &mut [&mut bob], &g, LATER);
    let members = bob.core.group(&g).unwrap().members;
    assert_eq!(members.len(), 3);
    assert_eq!(members.iter().filter(|m| **m == carol.id).count(), 1);
    // Her place is taken again, not a removal: the mailbox stays.
    assert_eq!(bob.core.group_mailboxes(&g, LATER).unwrap(), mailbox);
    let outcome = invite_at(&mut alice, &mut carol, LATER);
    assert!(
        matches!(&outcome, IntroOutcome::Joined(c) if c.id == g),
        "{outcome:?}"
    );
    assert_eq!(epoch(&carol, &g), e + 1);
    assert_eq!(carol.core.group_mailboxes(&g, LATER).unwrap(), mailbox);
    assert!(carol.core.stale_groups(LATER).unwrap().is_empty());
    let hello = say_at(&mut bob, &g, "welcome back", "m-back", LATER);
    read_at(&mut carol, &g, &hello, LATER);
    assert_eq!(texts(&carol, &g).last().unwrap().1, "welcome back");
    // Forgotten and joined again, each loading the group alone: every
    // profile's whole MLS state still holds together.
    for p in [&alice, &bob, &carol] {
        whole_mls(p);
    }
}

#[test]
fn a_private_groups_member_back_after_a_long_absence_asks_the_owner_and_a_removed_one_does_not_return()
 {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let g = team(&mut alice, &mut bob, &mut carol);
    // Both read through today and went away; meanwhile Alice removed Bob,
    // which neither read.
    for p in [&mut bob, &mut carol] {
        p.core.set_swarm_read_through(&g, period(NOW), NOW).unwrap();
    }
    let e = epoch(&alice, &g);
    let out = alice
        .core
        .change_group(&g, remove(&bob), "bob-out", NOW)
        .unwrap();
    alice
        .core
        .decide_group_commit(&g, e, &out.commit, NOW)
        .unwrap();
    assert_eq!(carol.core.stale_groups(LATER).unwrap().len(), 1);
    assert_eq!(bob.core.stale_groups(LATER).unwrap().len(), 1);
    // No door: each asks the owner, through the owner's intro mailbox.
    let owner_card = alice.core.intro_card(vec![], LATER).unwrap().envelope;
    for (p, op) in [(&mut carol, "rejoin-carol"), (&mut bob, "rejoin-bob")] {
        let back = p.core.rejoin_by_owner(&g, &owner_card, op, LATER).unwrap();
        let d = p
            .core
            .prepare_swarm_delivery(&back.message_id, LATER)
            .unwrap();
        assert_eq!(d.mailbox, alice.core.own_intro_mailbox(LATER).unwrap());
        // Nothing for the owner to decide either way.
        assert!(matches!(
            alice
                .core
                .receive_intro_envelope(&d.envelope, LATER)
                .unwrap(),
            IntroOutcome::Ignored
        ));
    }
    assert!(alice.core.intro_requests().unwrap().is_empty());
    // The next batch gives Carol her place back, and only her.
    let e = epoch(&alice, &g);
    batch_at(&mut alice, &mut [], &g, LATER);
    assert_eq!(
        sorted(alice.core.group(&g).unwrap().members),
        sorted(vec![alice.id.clone(), carol.id.clone()])
    );
    let outcome = invite_at(&mut alice, &mut carol, LATER);
    assert!(
        matches!(&outcome, IntroOutcome::Joined(c) if c.id == g),
        "{outcome:?}"
    );
    assert_eq!(epoch(&carol, &g), e + 1);
    let hello = say_at(&mut alice, &g, "together again", "m-back", LATER);
    read_at(&mut carol, &g, &hello, LATER);
    assert_eq!(texts(&carol, &g).last().unwrap().1, "together again");
}

#[test]
fn a_member_whose_reading_skipped_the_lost_days_is_still_behind_until_back() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let g = team(&mut alice, &mut bob, &mut carol);
    carol
        .core
        .set_swarm_read_through(&g, period(NOW), NOW)
        .unwrap();
    // Back forty days on, its node reads what the mailboxes still keep and
    // moves its reading past the days they no longer do.
    carol
        .core
        .set_swarm_read_through(&g, period(LATER) - 1, LATER)
        .unwrap();
    assert_eq!(
        carol
            .core
            .stale_groups(LATER)
            .unwrap()
            .iter()
            .map(|s| s.group_id.clone())
            .collect::<Vec<_>>(),
        std::slice::from_ref(&g)
    );
    // Across a restart too.
    let Person {
        _dir,
        core,
        id,
        root,
    } = carol;
    drop(core);
    let mut carol = Person {
        core: crate::core(&_dir, DOMAIN),
        _dir,
        id,
        root,
    };
    assert_eq!(carol.core.stale_groups(LATER).unwrap().len(), 1);
    // Back twenty days on, within what the mailboxes keep, is no absence.
    bob.core
        .set_swarm_read_through(&g, period(NOW), NOW)
        .unwrap();
    bob.core
        .set_swarm_read_through(&g, period(NOW) + 19, NOW + 20 * DAY)
        .unwrap();
    assert!(bob.core.stale_groups(NOW + 20 * DAY).unwrap().is_empty());
    // Once her place is given back she is no longer behind, and reading on
    // from there does not make her so.
    let owner_card = alice.core.intro_card(vec![], LATER).unwrap().envelope;
    let back = carol
        .core
        .rejoin_by_owner(&g, &owner_card, "rejoin", LATER)
        .unwrap();
    let d = carol
        .core
        .prepare_swarm_delivery(&back.message_id, LATER)
        .unwrap();
    alice
        .core
        .receive_intro_envelope(&d.envelope, LATER)
        .unwrap();
    batch_at(&mut alice, &mut [], &g, LATER);
    let outcome = invite_at(&mut alice, &mut carol, LATER);
    assert!(
        matches!(&outcome, IntroOutcome::Joined(c) if c.id == g),
        "{outcome:?}"
    );
    assert!(carol.core.stale_groups(LATER).unwrap().is_empty());
    carol
        .core
        .set_swarm_read_through(&g, period(LATER) + 1, LATER + 2 * DAY)
        .unwrap();
    assert!(carol.core.stale_groups(LATER + 2 * DAY).unwrap().is_empty());
}
