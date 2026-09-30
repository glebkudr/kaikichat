//! Open-read groups at any size (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
//! part 10a): a post carries its author's certificate of membership, so a
//! reader checks it without a roster listing the members; the roster names
//! only those removed and banned; a follower and a newcomer get the group's
//! last day.
use super::*;
use agentic_core::DoorEntry;
use agentic_protocol::group::MemberCert;

const HOUR: u64 = 3_600;

#[test]
fn a_newcomer_an_admin_let_in_writes_for_followers_and_the_roster_does_not_grow() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut frank = person("Frank");
    let g = opened(&mut alice, &mut bob, &mut carol);
    let gref = reference(&alice, &g);
    let bob_admin = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        bob_admin,
        "c-1",
    );
    let before = public_roster(&mut alice, &g).envelope.len();
    // Frank knocks; Bob lets him in with his batch.
    let card = door::door_card(&mut bob, &g);
    let application = door::knock(&mut frank, &gref, &card, "may I join?");
    assert_eq!(
        bob.core
            .receive_door_entry(&g, application.period, &application.envelope, NOW)
            .unwrap(),
        DoorEntry::Admitted(frank.id.clone())
    );
    door::batch_in(&mut bob, &mut [&mut alice, &mut carol], &g, NOW);
    deliver_invites(&mut bob, &mut [&mut frank]);
    // Ten more come in: what followers read of the group stays as big.
    let crowd = GroupChange {
        add: (0..10).map(|i| card_of(&format!("P{i}"))).collect(),
        ..GroupChange::default()
    };
    change(
        &mut alice,
        &mut [&mut bob, &mut carol, &mut frank],
        &g,
        crowd,
        "c-2",
    );
    let roster = public_roster(&mut alice, &g);
    assert!(
        roster.envelope.len() <= before + 2,
        "{} B, then {} B",
        before,
        roster.envelope.len()
    );
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "Team", NOW)
        .unwrap();
    assert_eq!(
        dave.core
            .receive_public_entry(&follow.id, &roster.envelope, NOW, NOW)
            .unwrap(),
        PublicEntry::Roster
    );
    let hello = say(&mut frank, &g, "Bob let me in", "p-1");
    assert_eq!(
        dave.core
            .receive_public_entry(&follow.id, &hello.envelope, NOW, NOW)
            .unwrap(),
        PublicEntry::Post
    );
    assert_eq!(
        texts(&dave, &follow.id),
        [(frank.id.clone(), "Bob let me in".to_owned())]
    );
}

#[test]
fn a_member_removed_and_added_again_writes_for_followers_under_its_new_certificate_only() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let g = opened(&mut alice, &mut bob, &mut carol);
    let gref = reference(&alice, &g);
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "Team", NOW)
        .unwrap();
    // The roster is published again on a removal, not on every epoch: a
    // group that lets people in every minute would pay for each.
    let key = alice.core.public_roster_key(&g).unwrap();
    assert!(key.is_some());
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        add(&mut erin),
        "c-1",
    );
    deliver_invites(&mut alice, &mut [&mut erin]);
    assert_eq!(alice.core.public_roster_key(&g).unwrap(), key);
    let before = say(&mut carol, &g, "before the removal", "p-1");
    let without_carol = remove(&carol);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol, &mut erin],
        &g,
        without_carol,
        "c-2",
    );
    assert_ne!(alice.core.public_roster_key(&g).unwrap(), key);
    let roster = public_roster(&mut alice, &g);
    let take = |dave: &mut Person, d: &SwarmDelivery| {
        dave.core
            .receive_public_entry(&follow.id, &d.envelope, NOW, NOW)
            .unwrap()
    };
    assert_eq!(take(&mut dave, &roster), PublicEntry::Roster);
    assert_eq!(take(&mut dave, &before), PublicEntry::Ignored);
    change(
        &mut alice,
        &mut [&mut bob, &mut erin],
        &g,
        add(&mut carol),
        "c-3",
    );
    deliver_invites(&mut alice, &mut [&mut carol]);
    let again = say(&mut carol, &g, "with you again", "p-2");
    assert_eq!(take(&mut dave, &again), PublicEntry::Post);
    assert_eq!(
        texts(&dave, &follow.id),
        [(carol.id.clone(), "with you again".to_owned())]
    );
    // A closing is published at once too: followers learn of it from it.
    let open = alice.core.public_roster_key(&g).unwrap();
    change(
        &mut alice,
        &mut [&mut bob, &mut carol, &mut erin],
        &g,
        access(Access::Private),
        "c-4",
    );
    assert_ne!(alice.core.public_roster_key(&g).unwrap(), open);
}

#[test]
fn a_member_an_admin_let_in_keeps_writing_for_followers_after_the_owner_demotes_that_admin() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut frank = person("Frank");
    let g = opened(&mut alice, &mut bob, &mut carol);
    let gref = reference(&alice, &g);
    let bob_admin = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        bob_admin,
        "c-1",
    );
    change(
        &mut bob,
        &mut [&mut alice, &mut carol],
        &g,
        add(&mut frank),
        "c-2",
    );
    deliver_invites(&mut bob, &mut [&mut frank]);
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "Team", NOW)
        .unwrap();
    let take = |dave: &mut Person, d: &SwarmDelivery| {
        dave.core
            .receive_public_entry(&follow.id, &d.envelope, NOW, NOW)
            .unwrap()
    };
    // Bob is a plain member again: the owner gives those he let in
    // certificates of its own with the same commit.
    let e = epoch(&alice, &g);
    let made = alice
        .core
        .change_group(&g, admins(&[]), "c-3", NOW)
        .unwrap();
    let commit = pending_in(&mut alice, &g);
    for p in [&mut bob, &mut carol, &mut frank] {
        read(p, &g, &commit).unwrap();
    }
    alice
        .core
        .decide_group_commit(&g, e, &made.commit, NOW)
        .unwrap();
    decide_everywhere(&mut [&mut bob, &mut carol, &mut frank], &g, e, &made.commit);
    let reissued = pending_in(&mut alice, &g);
    assert_ne!(
        reissued.envelope, commit.envelope,
        "the owner's certificates follow its commit"
    );
    read(&mut frank, &g, &reissued).unwrap();
    let roster = public_roster(&mut alice, &g);
    assert_eq!(take(&mut dave, &roster), PublicEntry::Roster);
    let from_frank = say(&mut frank, &g, "I am still here", "p-1");
    assert_eq!(take(&mut dave, &from_frank), PublicEntry::Post);
    // Bob writes under the certificate his invitation brought.
    let from_bob = say(&mut bob, &g, "me too", "p-2");
    assert_eq!(take(&mut dave, &from_bob), PublicEntry::Post);
    assert_eq!(
        texts(&dave, &follow.id),
        [
            (frank.id.clone(), "I am still here".to_owned()),
            (bob.id.clone(), "me too".to_owned()),
        ]
    );
}

/// A follower's checks of certificates, with keys of the test's own: the
/// signer must be the owner or an admin under the newest roster the
/// follower knows, the author not removed since, nor banned.
#[test]
fn a_follower_takes_certificates_only_from_the_owner_and_admins_of_the_newest_roster_it_knows() {
    let owner = SigningKey::from_bytes(&[51; 32]);
    let admin = SigningKey::from_bytes(&[52; 32]);
    let member = SigningKey::from_bytes(&[53; 32]);
    let stranger = SigningKey::from_bytes(&[54; 32]);
    let erin = SigningKey::from_bytes(&[55; 32]);
    let root = |k: &SigningKey| k.verifying_key().to_bytes();
    let gref = group_ref(&DOMAIN, &root(&owner), &[9; 32]);
    let roster = |version: u64, admins: Vec<[u8; 32]>| {
        signed(
            DocumentKind::GroupRoster,
            Roster {
                group: gref,
                version,
                admins,
                access: Access::Public,
                kind: agentic_protocol::group::GroupKind::Group,
            }
            .encode(),
            &owner,
        )
    };
    let cert = |who: &SigningKey, by: &SigningKey, roster: &[u8], epoch: u64| {
        signed(
            DocumentKind::GroupMember,
            MemberCert {
                group: gref,
                member: digest_of(who),
                epoch,
                roster: roster.to_vec(),
            }
            .encode(),
            by,
        )
    };
    let post = |who: &SigningKey, membership: Vec<u8>, n: u8| {
        let post = PublicPost {
            group: gref,
            epoch: 9,
            operation: [n; 32],
            text: format!("post {n}"),
            membership,
        };
        entry(1, &signed(DocumentKind::PublicPost, post.encode(), who))
    };
    let public = |roster: &[u8], epoch: u64, removed: Vec<([u8; 32], u64)>| {
        let public = PublicRoster {
            group: gref,
            epoch,
            roster: roster.to_vec(),
            removed,
            retention: agentic_protocol::group::Retention::Days(1),
        };
        entry(
            2,
            &signed(DocumentKind::PublicRoster, public.encode(), &owner),
        )
    };
    let (v1, v2) = (roster(1, vec![root(&admin)]), roster(2, vec![]));
    let mut dave = person("Dave");
    let owner_id = agentic_protocol::network_id(&root(&owner));
    let follow = dave
        .core
        .follow_group(gref, &owner_id, "Someone else's", NOW)
        .unwrap();
    let mut take = |entry: &[u8]| {
        dave.core
            .receive_public_entry(&follow.id, entry, NOW, NOW)
            .unwrap()
    };
    assert_eq!(take(&public(&v1, 3, vec![])), PublicEntry::Roster);
    let by_admin = cert(&member, &admin, &v1, 3);
    assert_eq!(take(&post(&member, by_admin.clone(), 1)), PublicEntry::Post);
    assert_eq!(take(&impostor(&stranger, gref, 3)), PublicEntry::Ignored);
    // The admin is a plain member again: its certificates no longer hold,
    // nor one it dates back for a stranger under the roster that named it.
    assert_eq!(take(&public(&v2, 4, vec![])), PublicEntry::Roster);
    assert_eq!(take(&post(&member, by_admin, 2)), PublicEntry::Ignored);
    let backdated = cert(&stranger, &admin, &v1, 2);
    assert_eq!(take(&post(&stranger, backdated, 3)), PublicEntry::Ignored);
    // The owner's certificate for the same member does; so does one by an
    // admin of a roster newer than the follower has read yet.
    let by_owner = cert(&member, &owner, &v2, 3);
    assert_eq!(take(&post(&member, by_owner.clone(), 4)), PublicEntry::Post);
    let v3 = roster(3, vec![root(&stranger)]);
    let by_new_admin = cert(&member, &stranger, &v3, 5);
    assert_eq!(take(&post(&member, by_new_admin, 5)), PublicEntry::Post);
    // Removed after its certificate's epoch: nothing more under it; added
    // again, it writes under the new one.
    let removed = vec![(digest_of(&member), 5)];
    assert_eq!(take(&public(&v3, 6, removed.clone())), PublicEntry::Roster);
    assert_eq!(take(&post(&member, by_owner, 6)), PublicEntry::Ignored);
    let again = cert(&member, &owner, &v3, 6);
    assert_eq!(take(&post(&member, again, 7)), PublicEntry::Post);
    // A banned id: none of its certificates, whatever their epoch.
    let mut with_ban = removed;
    with_ban.push((digest_of(&erin), u64::MAX));
    assert_eq!(take(&public(&v3, 7, with_ban)), PublicEntry::Roster);
    let late = cert(&erin, &owner, &v3, 100);
    assert_eq!(take(&post(&erin, late, 8)), PublicEntry::Ignored);
}

#[test]
fn a_follower_and_a_newcomer_get_the_last_day_of_an_open_group() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let g = opened(&mut alice, &mut bob, &mut carol);
    let gref = reference(&alice, &g);
    let older = say_at(&mut bob, &g, "two days ago", "p-1", NOW - 25 * HOUR);
    let recent = say_at(&mut bob, &g, "yesterday afternoon", "p-2", NOW - 23 * HOUR);
    // Epochs go by since: a group with a door moves on every minute.
    let bob_admin = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        bob_admin,
        "c-1",
    );
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        admins(&[]),
        "c-2",
    );
    // The day is counted by when the holders stored a post, not by the
    // date its author put on it.
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "Team", NOW)
        .unwrap();
    let stored = [(&older, NOW - 25 * HOUR), (&recent, NOW - 23 * HOUR)];
    for ((d, at), outcome) in stored.iter().zip([PublicEntry::Ignored, PublicEntry::Post]) {
        assert_eq!(
            dave.core
                .receive_public_entry(&follow.id, &d.envelope, *at, NOW)
                .unwrap(),
            outcome
        );
    }
    assert_eq!(
        texts(&dave, &follow.id),
        [(bob.id.clone(), "yesterday afternoon".to_owned())]
    );
    // Erin joins now: the same day is hers.
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        add(&mut erin),
        "c-3",
    );
    deliver_invites(&mut alice, &mut [&mut erin]);
    for (d, at) in stored {
        erin.core
            .receive_public_entry(&g, &d.envelope, at, NOW)
            .unwrap();
    }
    assert_eq!(
        texts(&erin, &g),
        [(bob.id.clone(), "yesterday afternoon".to_owned())]
    );
}
