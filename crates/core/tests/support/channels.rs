//! Channels (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts 8–10): a
//! channel is a group whose members are its owner and admins, the only ones
//! who write; anyone follows a public one without a door. Its team packs
//! its posts into archive parts and lays them down again for as long as the
//! channel keeps its history.
use super::*;
use agentic_core::ChannelStorage;
use agentic_protocol::group::{ArchivePart, GroupKind, MemberCert, Retention};

const HOUR: u64 = 3_600;
/// An envelope holds at most this.
const ENVELOPE: usize = 65_536;

/// A public channel of Alice with Bob in its team.
fn channel(alice: &mut Person, bob: &mut Person) -> String {
    let team = vec![invitee(bob)];
    let g = alice
        .core
        .create_channel("News", &team, Access::Public, "ch-1", NOW)
        .unwrap()
        .id;
    deliver_invites(alice, &mut [bob]);
    g
}

fn retention(to: Retention) -> GroupChange {
    GroupChange {
        retention: Some(to),
        ..GroupChange::default()
    }
}

/// The texts of the posts a part holds, in its order, as read at `at`.
fn part_texts(part: &SwarmDelivery, gref: &[u8; 32], at: u64) -> Vec<String> {
    assert_eq!(part.envelope[0], 3, "an archive part");
    assert!(part.envelope.len() <= ENVELOPE);
    ArchivePart::decode(&part.envelope[1..])
        .unwrap()
        .posts
        .iter()
        .map(|wire| verify_public_post(wire, DOMAIN, at, gref).unwrap().text)
        .collect()
}

/// `p`'s parts to lay at `at`, each read back from the public mailbox as
/// its node does once it stored them there.
fn lay(p: &mut Person, group: &str, at: u64) -> Vec<SwarmDelivery> {
    let parts = p.core.channel_archive(group, at).unwrap();
    for part in &parts {
        p.core
            .receive_public_entry(group, &part.envelope, at, at)
            .unwrap();
    }
    parts
}

fn used(p: &Person) -> u32 {
    p.core.mailbox_books().unwrap()[0].used
}

#[test]
fn a_channel_is_written_by_its_team_and_read_by_anyone_who_follows() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let g = channel(&mut alice, &mut bob);
    let info = alice.core.group(&g).unwrap();
    assert_eq!(
        (info.kind.as_str(), info.access.as_str()),
        ("channel", "public")
    );
    assert_eq!(info.admins, [bob.id.clone()]);
    assert_eq!(bob.core.group(&g).unwrap().role, "admin");
    // Whoever the owner adds joins the team.
    change(&mut alice, &mut [&mut bob], &g, add(&mut carol), "c-1");
    deliver_invites(&mut alice, &mut [&mut carol]);
    assert_eq!(
        sorted(alice.core.group(&g).unwrap().admins),
        sorted(vec![bob.id.clone(), carol.id.clone()])
    );
    assert_eq!(carol.core.group(&g).unwrap().role, "admin");
    // Only the owner changes the team, and nobody is in it but as an admin.
    assert!(matches!(
        bob.core.change_group(&g, add(&mut erin), "x-1", NOW),
        Err(CoreError::Unauthorized)
    ));
    assert!(matches!(
        alice.core.change_group(&g, admins(&[&bob]), "x-2", NOW),
        Err(CoreError::InvalidInput)
    ));
    // Anyone follows it; it has no door.
    assert!(alice.core.door_card(&g, NOW).unwrap().is_none());
    let gref = reference(&alice, &g);
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "News", NOW)
        .unwrap();
    let roster = public_roster(&mut alice, &g);
    let take = |dave: &mut Person, d: &SwarmDelivery| {
        dave.core
            .receive_public_entry(&follow.id, &d.envelope, NOW, NOW)
            .unwrap()
    };
    assert_eq!(take(&mut dave, &roster), PublicEntry::Roster);
    let followed = dave.core.follows().unwrap();
    assert_eq!(
        (followed[0].kind.as_str(), followed[0].retention),
        ("channel", Some(30))
    );
    let news = say(&mut carol, &g, "first news", "p-1");
    assert_eq!(take(&mut dave, &news), PublicEntry::Post);
    assert_eq!(
        texts(&dave, &follow.id),
        [(carol.id.clone(), "first news".to_owned())]
    );
}

/// With keys of the test's own: in a channel, an owner's certificate for
/// someone outside its team makes no writer.
#[test]
fn a_channels_followers_take_posts_of_its_owner_and_admins_only() {
    let owner = SigningKey::from_bytes(&[61; 32]);
    let admin = SigningKey::from_bytes(&[62; 32]);
    let outsider = SigningKey::from_bytes(&[63; 32]);
    let root = |k: &SigningKey| k.verifying_key().to_bytes();
    let gref = group_ref(&DOMAIN, &root(&owner), &[8; 32]);
    let roster = signed(
        DocumentKind::GroupRoster,
        Roster {
            group: gref,
            version: 1,
            admins: vec![root(&admin)],
            access: Access::Public,
            kind: GroupKind::Channel,
        }
        .encode(),
        &owner,
    );
    let post = |who: &SigningKey, by: &SigningKey, inside: Vec<u8>, n: u8| {
        let cert = signed(
            DocumentKind::GroupMember,
            MemberCert {
                group: gref,
                member: digest_of(who),
                epoch: 1,
                roster: inside,
            }
            .encode(),
            by,
        );
        let post = PublicPost {
            group: gref,
            epoch: 2,
            operation: [n; 32],
            text: format!("post {n}"),
            membership: cert,
        };
        entry(1, &signed(DocumentKind::PublicPost, post.encode(), who))
    };
    let public = PublicRoster {
        group: gref,
        epoch: 2,
        roster: roster.clone(),
        removed: vec![],
        retention: Retention::Days(90),
    };
    let public = entry(
        2,
        &signed(DocumentKind::PublicRoster, public.encode(), &owner),
    );
    let mut dave = person("Dave");
    let owner_id = agentic_protocol::network_id(&root(&owner));
    let follow = dave
        .core
        .follow_group(gref, &owner_id, "Channel", NOW)
        .unwrap();
    let mut take = |entry: &[u8]| {
        dave.core
            .receive_public_entry(&follow.id, entry, NOW, NOW)
            .unwrap()
    };
    assert_eq!(take(&public), PublicEntry::Roster);
    assert_eq!(
        take(&post(&outsider, &owner, vec![], 1)),
        PublicEntry::Ignored
    );
    assert_eq!(
        take(&post(&admin, &admin, roster.clone(), 2)),
        PublicEntry::Post
    );
    assert_eq!(take(&post(&owner, &owner, vec![], 3)), PublicEntry::Post);
}

#[test]
fn a_channels_team_packs_its_posts_into_parts_by_age_and_by_size() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let g = channel(&mut alice, &mut bob);
    let gref = reference(&alice, &g);
    // An admin sets how long the channel keeps its history.
    change(
        &mut bob,
        &mut [&mut alice],
        &g,
        retention(Retention::Days(90)),
        "r-90",
    );
    assert_eq!(alice.core.group(&g).unwrap().retention, Some(90));
    say_at(&mut alice, &g, "the first", "p-1", NOW + HOUR);
    let second = say_at(&mut bob, &g, "the second", "p-2", NOW + DAY);
    // Alice's node took Bob's post from the public mailbox.
    alice
        .core
        .receive_public_entry(&g, &second.envelope, NOW + DAY, NOW + DAY)
        .unwrap();
    // Young and small: the part stays open.
    assert!(
        alice
            .core
            .channel_archive(&g, NOW + 10 * DAY)
            .unwrap()
            .is_empty()
    );
    // Its oldest post is 25 days old: the part closes and is laid down in
    // the public mailbox of the day, for one stamp.
    let at = NOW + HOUR + 25 * DAY;
    let before = used(&alice);
    let parts = alice.core.channel_archive(&g, at).unwrap();
    assert_eq!(parts.len(), 1);
    assert_eq!(used(&alice), before + 1);
    assert_eq!(
        parts[0].mailbox,
        public_group_mailbox_id(&DOMAIN, &gref, period(at))
    );
    assert_eq!(
        part_texts(&parts[0], &gref, at),
        ["the first", "the second"]
    );
    // Until the node reads its copy back from the mailbox the same part is
    // handed out again, under the same stamp: a lost send loses nothing
    // and costs nothing more.
    let again = alice.core.channel_archive(&g, at).unwrap();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].envelope, parts[0].envelope);
    assert_eq!(used(&alice), before + 1);
    alice
        .core
        .receive_public_entry(&g, &parts[0].envelope, at, at)
        .unwrap();
    assert!(alice.core.channel_archive(&g, at).unwrap().is_empty());
    // Posts that fill a part close it at once: five of 12 KB fit (the
    // owner's certificate stands alone, without a roster), the sixth opens
    // the next.
    let big = "ü".repeat(6_000);
    for i in 0..6 {
        say_at(
            &mut alice,
            &g,
            &format!("{i}{big}"),
            &format!("big-{i}"),
            at + HOUR,
        );
    }
    let parts = lay(&mut alice, &g, at + HOUR);
    assert_eq!(parts.len(), 1);
    let texts = part_texts(&parts[0], &gref, at + HOUR);
    assert_eq!(texts.len(), 5);
    assert!(texts[0].starts_with('0') && texts[4].starts_with('4'));
    // Two parts kept: laid every 25 days, 2 × 30 / 25 stamps a month,
    // rounded up.
    let storage = alice.core.channel_storage(&g, at + HOUR).unwrap();
    assert_eq!((storage.parts, storage.stamps_per_month), (2, 3));
}

#[test]
fn a_channel_lays_its_parts_again_while_its_retention_holds_and_shows_what_that_costs() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let g = channel(&mut alice, &mut bob);
    let gref = reference(&alice, &g);
    change(
        &mut alice,
        &mut [&mut bob],
        &g,
        retention(Retention::Days(90)),
        "r-90",
    );
    say_at(&mut alice, &g, "for a long time", "p-1", NOW + HOUR);
    let laid_at = NOW + HOUR + 25 * DAY;
    let laid = lay(&mut alice, &g, laid_at);
    assert_eq!(laid.len(), 1);
    let again_at = laid_at + 25 * DAY;
    let storage: ChannelStorage = alice.core.channel_storage(&g, again_at).unwrap();
    assert_eq!((storage.retention, storage.parts), (Some(90), 1));
    assert_eq!(storage.bytes, laid[0].envelope.len() as u64);
    assert_eq!(storage.stamps_per_month, 2);
    // The same part again 25 days on, into that day's mailbox: the copy
    // before lives 30 days.
    let again = lay(&mut alice, &g, again_at);
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].envelope, laid[0].envelope);
    assert_eq!(
        again[0].mailbox,
        public_group_mailbox_id(&DOMAIN, &gref, period(again_at))
    );
    assert!(lay(&mut alice, &g, again_at + DAY).is_empty());
    // 75 days after its post the part is laid once more; after 90 it is
    // let go.
    assert_eq!(lay(&mut alice, &g, NOW + HOUR + 75 * DAY).len(), 1);
    assert!(lay(&mut alice, &g, NOW + HOUR + 100 * DAY).is_empty());
    assert_eq!(
        alice
            .core
            .channel_storage(&g, NOW + HOUR + 100 * DAY)
            .unwrap()
            .parts,
        0
    );
}

#[test]
fn a_channel_kept_forever_lays_its_parts_again_for_good_and_its_cost_grows() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let g = channel(&mut alice, &mut bob);
    // Kept as long as the mailboxes keep posts, 30 days: no archive.
    say_at(&mut alice, &g, "forever", "p-1", NOW + HOUR);
    let first = NOW + HOUR + 25 * DAY;
    assert!(alice.core.channel_archive(&g, first).unwrap().is_empty());
    // Kept longer: what is still alive goes into the archive too.
    change_at(
        &mut alice,
        &mut [&mut bob],
        &g,
        retention(Retention::Forever),
        "r-forever",
        first,
    );
    assert_eq!(lay(&mut alice, &g, first).len(), 1);
    assert_eq!(lay(&mut alice, &g, first + 25 * DAY).len(), 1);
    say_at(&mut alice, &g, "later", "p-2", NOW + 60 * DAY);
    assert_eq!(lay(&mut alice, &g, first + 50 * DAY).len(), 1);
    let storage = alice.core.channel_storage(&g, NOW + 80 * DAY).unwrap();
    assert_eq!(
        (
            storage.parts,
            storage.stamps_per_month,
            storage.added_last_month
        ),
        (1, 2, 0)
    );
    // The second part closes; the first, laid ten days ago, waits.
    let second = lay(&mut alice, &g, NOW + 85 * DAY);
    assert_eq!(second.len(), 1);
    let storage = alice.core.channel_storage(&g, NOW + 100 * DAY).unwrap();
    assert_eq!(
        (
            storage.retention,
            storage.parts,
            storage.stamps_per_month,
            storage.added_last_month
        ),
        (None, 2, 3, 1)
    );
    // Past 90 days and more, the first part is still laid.
    assert_eq!(lay(&mut alice, &g, first + 75 * DAY).len(), 1);
}

#[test]
fn a_follower_of_a_channel_reads_its_last_30_days_and_its_archive() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut dave = person("Dave");
    let g = channel(&mut alice, &mut bob);
    let gref = reference(&alice, &g);
    change(
        &mut alice,
        &mut [&mut bob],
        &g,
        retention(Retention::Days(90)),
        "r-90",
    );
    say_at(&mut alice, &g, "from the archive", "p-1", NOW + HOUR);
    assert_eq!(lay(&mut alice, &g, NOW + HOUR + 25 * DAY).len(), 1);
    // The copy a follower finds is the one laid again.
    let relaid_at = NOW + HOUR + 50 * DAY;
    let copy = lay(&mut alice, &g, relaid_at).remove(0);
    let fresh_at = NOW + 56 * DAY;
    let fresh = say_at(&mut bob, &g, "fresh", "p-2", fresh_at);
    alice
        .core
        .receive_public_entry(&g, &fresh.envelope, fresh_at, fresh_at)
        .unwrap();
    // Dave follows two weeks after the fresh post.
    let later = NOW + 70 * DAY;
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "News", later)
        .unwrap();
    let roster = alice.core.public_roster(&g, later).unwrap().unwrap();
    let take = |dave: &mut Person, entry: &[u8], stored_at: u64, at: u64| {
        dave.core
            .receive_public_entry(&follow.id, entry, stored_at, at)
            .unwrap()
    };
    assert_eq!(
        take(&mut dave, &roster.envelope, later, later),
        PublicEntry::Roster
    );
    assert_eq!(dave.core.follows().unwrap()[0].retention, Some(90));
    assert_eq!(
        take(&mut dave, &copy.envelope, relaid_at, later),
        PublicEntry::Archive
    );
    // Live in the mailbox for 30 days, not a day only.
    assert_eq!(
        take(&mut dave, &fresh.envelope, fresh_at, later),
        PublicEntry::Post
    );
    // Its part closes later: Dave keeps one copy of the post.
    let closed_at = fresh_at + 25 * DAY;
    // The first part's own turn comes first, apart.
    lay(&mut alice, &g, NOW + HOUR + 75 * DAY);
    let parts = lay(&mut alice, &g, closed_at);
    assert_eq!(parts.len(), 1);
    assert_eq!(
        take(&mut dave, &parts[0].envelope, closed_at, closed_at),
        PublicEntry::Archive
    );
    // Read again, from another holder: nothing new.
    assert_eq!(
        take(&mut dave, &copy.envelope, relaid_at, closed_at),
        PublicEntry::Ignored
    );
    // A part holds only its team's posts: a stranger's inside is passed
    // over.
    let stranger = SigningKey::from_bytes(&[77; 32]);
    let forged = ArchivePart {
        group: gref,
        posts: vec![impostor(&stranger, gref, 3)[1..].to_vec()],
    };
    let forged = [&[3u8][..], &forged.encode()].concat();
    assert_eq!(
        take(&mut dave, &forged, closed_at, closed_at),
        PublicEntry::Ignored
    );
    assert_eq!(
        texts(&dave, &follow.id),
        [
            (alice.id.clone(), "from the archive".to_owned()),
            (bob.id.clone(), "fresh".to_owned()),
        ]
    );
}

#[test]
fn two_admins_of_a_channel_lay_each_part_once_and_either_packs_alone() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let g = channel(&mut alice, &mut bob);
    let gref = reference(&alice, &g);
    change(
        &mut alice,
        &mut [&mut bob],
        &g,
        retention(Retention::Days(90)),
        "r-90",
    );
    let post = say_at(&mut alice, &g, "a single one", "p-1", NOW + HOUR);
    bob.core
        .receive_public_entry(&g, &post.envelope, NOW + HOUR, NOW + HOUR)
        .unwrap();
    let at = NOW + HOUR + 25 * DAY;
    let alices = lay(&mut alice, &g, at);
    assert_eq!(alices.len(), 1);
    // Bob reads Alice's part: he lays none of his own for the same posts.
    assert_eq!(
        bob.core
            .receive_public_entry(&g, &alices[0].envelope, at, at)
            .unwrap(),
        PublicEntry::Archive
    );
    assert!(bob.core.channel_archive(&g, at).unwrap().is_empty());
    // Alice is away when it is due again: Bob lays it; back, Alice reads
    // his copy of the day and does not lay it again.
    let due = at + 25 * DAY;
    let bobs = lay(&mut bob, &g, due);
    assert_eq!(bobs.len(), 1);
    assert_eq!(bobs[0].envelope, alices[0].envelope);
    alice
        .core
        .receive_public_entry(&g, &bobs[0].envelope, due, due)
        .unwrap();
    assert!(alice.core.channel_archive(&g, due).unwrap().is_empty());
    // Away again, Alice leaves Bob to pack a post of his own.
    say_at(&mut bob, &g, "without Alice", "p-2", due + HOUR);
    let packed_at = due + HOUR + 25 * DAY;
    let packed = lay(&mut bob, &g, packed_at);
    assert!(
        packed
            .iter()
            .any(|part| part_texts(part, &gref, packed_at) == ["without Alice"]),
        "{} parts",
        packed.len()
    );
}
