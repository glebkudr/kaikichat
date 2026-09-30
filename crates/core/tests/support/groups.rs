//! Groups (spec/groups-v1.md), core side: an owner creates a group from
//! intro cards, members talk through one group mailbox per epoch, commits
//! are applied in the order the notary names, and a removed member reads
//! nothing new.
use super::*;
use agentic_core::{
    CommitDecision, CoreError, GroupChange, GroupClaim, IntroMode, IntroOutcome, IntroPolicy,
    Invitee, PublicEntry, SwarmDelivery,
};
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, period, public_group_mailbox_id};
use agentic_protocol::group::{
    Access, CommitClaim, PublicPost, PublicRoster, Roster, check_transition, group_ref,
    verify_claim, verify_public_post,
};
use agentic_protocol::group::{Ban, Bans, MAX_BANS, check_bans};

const DAY: u64 = PERIOD_SECONDS;

#[path = "channels.rs"]
mod channels;
#[path = "closed_channels.rs"]
mod closed_channels;
#[path = "door.rs"]
mod door;
#[path = "large_groups.rs"]
mod large_groups;
#[path = "public_scale.rs"]
mod public_scale;

struct Person {
    _dir: TempDir,
    core: AppCore,
    id: String,
    /// The root key that signs for the profile.
    root: [u8; 32],
}

fn person(name: &str) -> Person {
    let dir = TempDir::new().unwrap();
    let mut core = profile(&dir, name);
    core.mailbox_book_account().unwrap();
    core.add_mailbox_book([0xb0; 32], 1_000, NOW + 120 * DAY)
        .unwrap();
    let card = core.intro_card(vec![], NOW).unwrap().envelope;
    let root = *VerifiedDocument::decode(&card[1..], DOMAIN, NOW)
        .unwrap()
        .author();
    let id = core.snapshot().unwrap().identity.unwrap().network_id;
    Person {
        _dir: dir,
        core,
        id,
        root,
    }
}

fn invitee(p: &mut Person) -> Invitee {
    Invitee {
        network_id: p.id.clone(),
        card: p.core.intro_card(vec![], NOW).unwrap().envelope,
    }
}

fn add(p: &mut Person) -> GroupChange {
    GroupChange {
        add: vec![invitee(p)],
        ..GroupChange::default()
    }
}

fn remove(p: &Person) -> GroupChange {
    GroupChange {
        remove: vec![p.id.clone()],
        ..GroupChange::default()
    }
}

fn admins(ids: &[&Person]) -> GroupChange {
    GroupChange {
        admins: Some(ids.iter().map(|p| p.id.clone()).collect()),
        ..GroupChange::default()
    }
}

/// Open the group to anyone's reading, or close it again.
fn access(to: Access) -> GroupChange {
    GroupChange {
        access: Some(to),
        ..GroupChange::default()
    }
}

fn ban(ids: &[impl AsRef<str>]) -> GroupChange {
    GroupChange {
        ban: ids.iter().map(|id| id.as_ref().to_owned()).collect(),
        ..GroupChange::default()
    }
}

fn unban(ids: &[impl AsRef<str>]) -> GroupChange {
    GroupChange {
        unban: ids.iter().map(|id| id.as_ref().to_owned()).collect(),
        ..GroupChange::default()
    }
}

/// The bans `p` sees in `group`: each id, and whether the owner banned it.
fn bans(p: &Person, group: &str) -> Vec<(String, bool)> {
    let mut bans: Vec<(String, bool)> = p
        .core
        .group(group)
        .unwrap()
        .banned
        .into_iter()
        .map(|b| (b.id, b.by_owner))
        .collect();
    bans.sort();
    bans
}

/// `from`'s pending invitations: the intro mailbox each goes to.
fn invitations(from: &Person) -> Vec<[u8; 32]> {
    from.core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .filter_map(|item| {
            from.core
                .outbox(100)
                .unwrap()
                .into_iter()
                .find(|w| w.message_id == item.message_id)
        })
        .filter_map(|direct| from.core.intro_mailbox(&direct.destination, NOW).ok())
        .collect()
}

/// Deliver every pending invitation of `from` through the intro mailboxes of
/// the people it names; each answers the direct copy with a receipt, which
/// ends the invitation's delivery.
fn deliver_invites(from: &mut Person, to: &mut [&mut Person]) -> Vec<IntroOutcome> {
    let mut outcomes = vec![];
    let direct: Vec<String> = from
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .map(|w| w.message_id)
        .collect();
    for item in from.core.swarm_outbox(64).unwrap() {
        // Invitations only: they alone also go directly.
        if !direct.contains(&item.message_id) {
            continue;
        }
        let delivery = from
            .core
            .prepare_swarm_delivery(&item.message_id, NOW)
            .unwrap();
        for p in to.iter_mut() {
            if p.core.own_intro_mailbox(NOW).unwrap() != delivery.mailbox {
                continue;
            }
            outcomes.push(
                p.core
                    .receive_intro_envelope(&delivery.envelope, NOW)
                    .unwrap(),
            );
            let direct = from
                .core
                .outbox(100)
                .unwrap()
                .into_iter()
                .find(|w| w.message_id == item.message_id)
                .expect("an invitation also goes directly");
            assert_eq!(direct.destination, p.id);
            let receipt = p.core.receive(&direct.wire, NOW).unwrap().reply.unwrap();
            from.core.receive(&receipt, NOW).unwrap();
        }
    }
    outcomes
}

/// `from`'s newest pending swarm delivery in `group`.
fn pending_in(from: &mut Person, group: &str) -> SwarmDelivery {
    let item = from
        .core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .rfind(|p| p.conversation_id == group)
        .expect("a pending group message");
    from.core
        .prepare_swarm_delivery(&item.message_id, NOW)
        .unwrap()
}

/// Read a group envelope as `to` does from the group mailbox.
fn read(to: &mut Person, group: &str, d: &SwarmDelivery) -> Result<(), CoreError> {
    to.core
        .receive_swarm_envelope(group, d.period, &d.envelope, NOW)
        .map(|_| ())
}

fn texts(p: &Person, group: &str) -> Vec<(String, String)> {
    p.core
        .snapshot()
        .unwrap()
        .conversations
        .into_iter()
        .find(|c| c.id == group)
        .map(|c| c.messages.into_iter().map(|m| (m.author, m.text)).collect())
        .unwrap_or_default()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn say(from: &mut Person, group: &str, text: &str, op: &str) -> SwarmDelivery {
    from.core.send_message(group, text, op, NOW).unwrap();
    pending_in(from, group)
}

fn epoch(p: &Person, group: &str) -> u64 {
    p.core.group(group).unwrap().epoch
}

/// The notary's choice (`winner`, a commit hash) applied by `people`.
fn decide_everywhere(people: &mut [&mut Person], group: &str, epoch: u64, winner: &str) {
    for p in people.iter_mut() {
        p.core
            .decide_group_commit(group, epoch, winner, NOW)
            .unwrap();
    }
}

/// `committer` changes the group; its commit is read by `readers` and chosen.
fn change(
    committer: &mut Person,
    readers: &mut [&mut Person],
    group: &str,
    what: GroupChange,
    op: &str,
) -> String {
    change_at(committer, readers, group, what, op, NOW)
}

/// `change` at `at`.
fn change_at(
    committer: &mut Person,
    readers: &mut [&mut Person],
    group: &str,
    what: GroupChange,
    op: &str,
    at: u64,
) -> String {
    let e = epoch(committer, group);
    let commit = committer
        .core
        .change_group(group, what, op, at)
        .unwrap()
        .commit;
    let item = committer
        .core
        .swarm_outbox(64)
        .unwrap()
        .into_iter()
        .rfind(|p| p.conversation_id == group)
        .expect("a pending commit");
    let d = committer
        .core
        .prepare_swarm_delivery(&item.message_id, at)
        .unwrap();
    for p in readers.iter_mut() {
        p.core
            .receive_swarm_envelope(group, d.period, &d.envelope, at)
            .unwrap();
    }
    committer
        .core
        .decide_group_commit(group, e, &commit, at)
        .unwrap();
    for p in readers.iter_mut() {
        p.core.decide_group_commit(group, e, &commit, at).unwrap();
    }
    commit
}

/// `p`'s MLS head and records as its database holds them.
fn mls_rows(p: &Person) -> (Vec<u8>, std::collections::BTreeMap<Vec<u8>, Vec<u8>>) {
    let db = super::profile_db::encrypted_db(&p._dir);
    let head: Vec<u8> = db
        .query_row("SELECT bytes FROM states WHERE namespace='mls'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let rows = db
        .prepare("SELECT record_key,bytes FROM state_records WHERE namespace='mls'")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    (head, rows)
}

/// `p`'s whole MLS state restores from its rows: every operation, loading
/// its own group alone, kept the count.
fn whole_mls(p: &Person) {
    let (head, rows) = mls_rows(p);
    agentic_crypto::MlsClient::restore_records(&head, rows).unwrap();
}

/// `outsider` banned if it is not, unbanned if it is: a commit that moves
/// the group on and leaves its mailbox.
fn toggle(owner: &Person, group: &str, outsider: &str) -> GroupChange {
    if bans(owner, group).iter().any(|(id, _)| id == outsider) {
        unban(&[outsider])
    } else {
        ban(&[outsider])
    }
}

/// `n` epochs on by `owner`'s commits, read by `readers`.
fn move_on(owner: &mut Person, readers: &mut [&mut Person], group: &str, outsider: &str, n: usize) {
    for _ in 0..n {
        let what = toggle(owner, group, outsider);
        let op = format!("move-{}-{}", &group[..8], epoch(owner, group));
        change(owner, readers, group, what, &op);
    }
}

/// A group of Alice (owner) with Bob and Carol, both joined.
fn team(alice: &mut Person, bob: &mut Person, carol: &mut Person) -> String {
    let invitees = vec![invitee(bob), invitee(carol)];
    let g = alice
        .core
        .create_group("Team", &invitees, "g-1", NOW)
        .unwrap()
        .id;
    deliver_invites(alice, &mut [bob, carol]);
    g
}

#[test]
fn an_owner_makes_a_group_from_cards_and_everyone_talks_through_one_mailbox() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let invitees = vec![invitee(&mut bob), invitee(&mut carol)];
    let group = alice
        .core
        .create_group("Team", &invitees, "g-1", NOW)
        .unwrap();
    assert_eq!(
        (
            group.name.as_str(),
            group.owner.as_str(),
            group.role.as_str()
        ),
        ("Team", alice.id.as_str(), "owner")
    );
    assert_eq!(
        sorted(group.members.clone()),
        sorted(vec![alice.id.clone(), bob.id.clone(), carol.id.clone()])
    );
    assert!(group.admins.is_empty());
    // Safe to retry.
    assert_eq!(
        alice
            .core
            .create_group("Team", &invitees, "g-1", NOW)
            .unwrap()
            .id,
        group.id
    );
    let g = group.id.clone();
    // Each invitation goes to its invitee's intro mailbox; they join.
    let outcomes = deliver_invites(&mut alice, &mut [&mut bob, &mut carol]);
    assert_eq!(outcomes.len(), 2);
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o, IntroOutcome::Joined(c) if c.id == g && c.title == "Team"))
    );
    for p in [&bob, &carol] {
        let info = p.core.group(&g).unwrap();
        assert_eq!(
            (info.owner.as_str(), info.role.as_str(), info.epoch),
            (alice.id.as_str(), "member", group.epoch)
        );
        assert_eq!(sorted(info.members), sorted(group.members.clone()));
    }
    // One mailbox for everyone of the epoch.
    let mailbox = alice.core.swarm_mailbox(&g, false, NOW).unwrap();
    for p in [&alice, &bob, &carol] {
        assert_eq!(p.core.swarm_mailbox(&g, true, NOW).unwrap(), mailbox);
        assert_eq!(p.core.group_mailboxes(&g, NOW).unwrap(), [mailbox]);
    }
    let hello = say(&mut alice, &g, "hello everyone", "m-1");
    assert_eq!(hello.mailbox, mailbox);
    // A group message goes only through the swarm: no direct copy.
    assert!(
        alice
            .core
            .outbox(100)
            .unwrap()
            .iter()
            .all(|w| w.message_id != hello.message_id)
    );
    for p in [&mut bob, &mut carol] {
        read(p, &g, &hello).unwrap();
    }
    let reply = say(&mut bob, &g, "hi", "m-2");
    for p in [&mut alice, &mut carol] {
        read(p, &g, &reply).unwrap();
    }
    for p in [&alice, &bob, &carol] {
        assert_eq!(
            texts(p, &g),
            [
                (alice.id.clone(), "hello everyone".to_owned()),
                (bob.id.clone(), "hi".to_owned())
            ]
        );
    }
    // One slot for a message to the whole group.
    let used = |p: &Person| p.core.mailbox_books().unwrap()[0].used;
    let before = used(&carol);
    let last = say(&mut carol, &g, "and me too", "m-3");
    assert_eq!(used(&carol), before + 1);
    read(&mut alice, &g, &last).unwrap();
    // Not a member: nothing to read.
    let mut dave = person("Dave");
    assert!(read(&mut dave, &g, &last).is_err());
    assert!(alice.core.groups().unwrap().iter().any(|i| i.id == g));
}

#[test]
fn a_removed_member_reads_nothing_new_and_an_added_one_reads_from_its_epoch() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = team(&mut alice, &mut bob, &mut carol);
    let before = say(&mut alice, &g, "before", "m-1");
    for p in [&mut bob, &mut carol] {
        read(p, &g, &before).unwrap();
    }
    let e = epoch(&alice, &g);
    let what = GroupChange {
        add: vec![invitee(&mut dave)],
        remove: vec![carol.id.clone()],
        ..GroupChange::default()
    };
    let made = alice
        .core
        .change_group(&g, what.clone(), "c-1", NOW)
        .unwrap();
    assert_eq!(made.epoch, e);
    // Safe to retry: the same commit, one claim.
    assert_eq!(
        alice
            .core
            .change_group(&g, what, "c-1", NOW)
            .unwrap()
            .commit,
        made.commit
    );
    // Waiting for the notary, the committer sends nothing else, and nobody
    // is invited yet.
    assert!(matches!(
        alice.core.send_message(&g, "too early", "m-x", NOW),
        Err(CoreError::GroupBusy)
    ));
    assert!(invitations(&alice).is_empty());
    // The commit travels in the group mailbox of its epoch.
    let commit = pending_in(&mut alice, &g);
    assert_eq!(commit.mailbox, before.mailbox);
    for p in [&mut bob, &mut carol] {
        read(p, &g, &commit).unwrap();
    }
    // Bob and Carol write in the epoch before Carol's removal is decided:
    // Carol's first message reaches the others at once, the rest later.
    let early = say(&mut carol, &g, "still here for now", "m-2");
    for p in [&mut alice, &mut bob] {
        read(p, &g, &early).unwrap();
    }
    let late = say(&mut bob, &g, "made it before the commit", "m-2b");
    let kicked = say(&mut carol, &g, "you cannot kick me out", "m-2c");
    // Everyone holds the committer's signed claim for the notary.
    let claim_key = |c: &GroupClaim| verify_claim(&c.claim, DOMAIN, NOW).unwrap().key;
    let mut keys = vec![];
    for p in [&alice, &bob, &carol] {
        assert_eq!(epoch(p, &g), e);
        let claims = p.core.group_claims(&g).unwrap();
        assert_eq!(claims.len(), 1);
        let claim = &claims[0];
        assert_eq!(
            (claim.commit.as_str(), claim.epoch, claim.round),
            (made.commit.as_str(), e, 0)
        );
        let verified = verify_claim(&claim.claim, DOMAIN, NOW).unwrap();
        assert_eq!(verified.committer, alice.root);
        assert_eq!(verified.claim.epoch, e);
        assert_eq!(hex::encode(verified.claim.commit), made.commit);
        keys.push(claim_key(claim));
    }
    assert!(keys.windows(2).all(|w| w[0] == w[1]));
    assert_eq!(
        alice
            .core
            .decide_group_commit(&g, e, &made.commit, NOW)
            .unwrap(),
        CommitDecision::Applied
    );
    // Decided again: nothing changes; another commit for a decided epoch is
    // refused.
    assert_eq!(
        alice
            .core
            .decide_group_commit(&g, e, &made.commit, NOW)
            .unwrap(),
        CommitDecision::Applied
    );
    assert!(
        alice
            .core
            .decide_group_commit(&g, e, &"ab".repeat(32), NOW)
            .is_err()
    );
    decide_everywhere(&mut [&mut bob, &mut carol], &g, e, &made.commit);
    let info = alice.core.group(&g).unwrap();
    assert_eq!(info.epoch, e + 1);
    assert_eq!(
        sorted(info.members.clone()),
        sorted(vec![alice.id.clone(), bob.id.clone(), dave.id.clone()])
    );
    assert_eq!(bob.core.group(&g).unwrap().members, info.members);
    // A message of the epoch before is still read after the move; of the
    // member the commit removed, nothing more is taken, though it is sealed
    // for that epoch. What was read of it before stays.
    read(&mut alice, &g, &late).unwrap();
    assert_eq!(
        texts(&alice, &g).last().unwrap().1,
        "made it before the commit"
    );
    for p in [&mut alice, &mut bob] {
        assert!(read(p, &g, &kicked).is_err());
        let seen: Vec<String> = texts(p, &g).into_iter().map(|(_, t)| t).collect();
        assert!(seen.iter().any(|t| t == "still here for now"), "{seen:?}");
        assert!(
            !seen.iter().any(|t| t == "you cannot kick me out"),
            "{seen:?}"
        );
        assert_eq!(p.core.group_mailboxes(&g, NOW).unwrap().len(), 2);
    }
    // And the commit, sent late, still goes to its own epoch's mailbox.
    assert_eq!(
        alice
            .core
            .prepare_swarm_delivery(&made.message_id, NOW)
            .unwrap()
            .mailbox,
        before.mailbox
    );
    // Carol is out: she derives no mailbox of the new epoch.
    assert!(carol.core.swarm_mailbox(&g, true, NOW).is_err());
    // Only now Dave is invited; he joins and reads from his epoch on.
    assert_eq!(
        invitations(&alice),
        [dave.core.own_intro_mailbox(NOW).unwrap()]
    );
    let outcomes = deliver_invites(&mut alice, &mut [&mut dave]);
    assert!(matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g));
    let his = dave.core.group(&g).unwrap();
    assert_eq!((his.epoch, his.owner.as_str()), (e + 1, alice.id.as_str()));
    assert_eq!(sorted(his.members), sorted(info.members.clone()));
    assert!(read(&mut dave, &g, &before).is_err());
    let after = say(&mut alice, &g, "after", "m-3");
    assert_ne!(after.mailbox, before.mailbox);
    for p in [&mut bob, &mut dave] {
        read(p, &g, &after).unwrap();
    }
    assert!(read(&mut carol, &g, &after).is_err());
    assert!(!texts(&carol, &g).iter().any(|(_, t)| t == "after"));
    assert_eq!(texts(&dave, &g), [(alice.id.clone(), "after".to_owned())]);

    // Carol, invited again, joins the current epoch: new messages yes, the
    // ones in between no.
    let e2 = epoch(&alice, &g);
    change(
        &mut alice,
        &mut [&mut bob, &mut dave],
        &g,
        add(&mut carol),
        "c-2",
    );
    assert_eq!(epoch(&alice, &g), e2 + 1);
    let outcomes = deliver_invites(&mut alice, &mut [&mut carol]);
    assert!(matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g));
    assert_eq!(epoch(&carol, &g), e2 + 1);
    // Joined by a commit that removed nobody, she shares the mailbox and
    // passes over what came before her, unread.
    read(&mut carol, &g, &after).unwrap();
    assert!(!texts(&carol, &g).iter().any(|(_, t)| t == "after"));
    let back = say(&mut bob, &g, "welcome back", "m-4");
    read(&mut carol, &g, &back).unwrap();
    assert_eq!(texts(&carol, &g).last().unwrap().1, "welcome back");
    // Back in, she is heard again, but not what she wrote while out.
    assert!(read(&mut alice, &g, &kicked).is_err());
    let again = say(&mut carol, &g, "here again", "m-5");
    read(&mut alice, &g, &again).unwrap();
    assert_eq!(
        texts(&alice, &g).last().unwrap(),
        &(carol.id.clone(), "here again".to_owned())
    );
}

#[test]
fn concurrent_commits_follow_the_notarys_choice_and_the_loser_changes_again() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let g = team(&mut alice, &mut bob, &mut carol);
    // The owner makes Bob an admin: a new roster. The claim carries the
    // roster in force before it.
    let e1 = epoch(&alice, &g);
    let promote = alice
        .core
        .change_group(&g, admins(&[&bob]), "c-1", NOW)
        .unwrap();
    let claim = verify_claim(&alice.core.group_claims(&g).unwrap()[0].claim, DOMAIN, NOW).unwrap();
    assert_eq!((claim.roster.version, claim.roster.admins.len()), (1, 0));
    let commit = pending_in(&mut alice, &g);
    for p in [&mut bob, &mut carol] {
        read(p, &g, &commit).unwrap();
    }
    decide_everywhere(
        &mut [&mut alice, &mut bob, &mut carol],
        &g,
        e1,
        &promote.commit,
    );
    for p in [&alice, &bob, &carol] {
        assert_eq!(p.core.group(&g).unwrap().admins, [bob.id.clone()]);
    }
    assert_eq!(bob.core.group(&g).unwrap().role, "admin");

    // At the same epoch the owner adds Dave and the admin adds Erin.
    let e2 = e1 + 1;
    let mine = alice
        .core
        .change_group(&g, add(&mut dave), "c-2", NOW)
        .unwrap();
    let theirs = bob
        .core
        .change_group(&g, add(&mut erin), "c-3", NOW)
        .unwrap();
    assert_eq!((mine.epoch, theirs.epoch), (e2, e2));
    // Bob's claim carries the roster naming him.
    let bobs = verify_claim(&bob.core.group_claims(&g).unwrap()[0].claim, DOMAIN, NOW).unwrap();
    assert_eq!(
        (bobs.roster.version, bobs.roster.admins.clone()),
        (2, vec![bob.root])
    );
    assert_eq!(bobs.committer, bob.root);
    // A round without a winner: the committer signs its claim again.
    let renewed = alice.core.renew_group_claim(&g, 1).unwrap();
    assert_eq!(
        (renewed.round, renewed.commit.as_str()),
        (1, mine.commit.as_str())
    );
    let renewed = verify_claim(&renewed.claim, DOMAIN, NOW).unwrap();
    assert_eq!((renewed.claim.round, renewed.claim.epoch), (1, e2));
    let from_alice = pending_in(&mut alice, &g);
    let from_bob = pending_in(&mut bob, &g);
    read(&mut bob, &g, &from_alice).unwrap();
    read(&mut carol, &g, &from_alice).unwrap();
    read(&mut alice, &g, &from_bob).unwrap();
    read(&mut carol, &g, &from_bob).unwrap();
    assert_eq!(carol.core.group_claims(&g).unwrap().len(), 2);
    // The notary named Bob's commit first.
    assert_eq!(
        alice
            .core
            .decide_group_commit(&g, e2, &theirs.commit, NOW)
            .unwrap(),
        CommitDecision::Lost
    );
    // The loser invites nobody.
    assert!(invitations(&alice).is_empty());
    decide_everywhere(&mut [&mut bob, &mut carol], &g, e2, &theirs.commit);
    for p in [&alice, &bob, &carol] {
        let info = p.core.group(&g).unwrap();
        assert_eq!(info.epoch, e2 + 1);
        assert!(info.members.contains(&erin.id));
        assert!(!info.members.contains(&dave.id));
        assert!(p.core.group_claims(&g).unwrap().is_empty());
    }
    // The winner invites Erin, who sees the roster naming Bob.
    let outcomes = deliver_invites(&mut bob, &mut [&mut erin]);
    assert!(matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g));
    let hers = erin.core.group(&g).unwrap();
    assert_eq!(
        (hers.owner.as_str(), hers.admins.clone(), hers.role.as_str()),
        (alice.id.as_str(), vec![bob.id.clone()], "member")
    );
    assert_eq!(
        sorted(hers.members),
        sorted(alice.core.group(&g).unwrap().members)
    );
    // The loser changes again on the new epoch; this time it applies.
    change(
        &mut alice,
        &mut [&mut bob, &mut carol, &mut erin],
        &g,
        add(&mut dave),
        "c-4",
    );
    deliver_invites(&mut alice, &mut [&mut dave]);
    let members = sorted(alice.core.group(&g).unwrap().members);
    assert_eq!(members.len(), 5);
    for p in [&bob, &carol, &erin, &dave] {
        assert_eq!(sorted(p.core.group(&g).unwrap().members), members);
    }
    // A decision before its commit is read waits for that commit; a rival
    // commit of the same epoch read first is not applied.
    let e4 = epoch(&alice, &g);
    assert_eq!(epoch(&dave, &g), e4);
    let named = alice
        .core
        .change_group(&g, remove(&carol), "c-5", NOW)
        .unwrap();
    let named_commit = pending_in(&mut alice, &g);
    bob.core
        .change_group(&g, remove(&erin), "c-6", NOW)
        .unwrap();
    let rival_commit = pending_in(&mut bob, &g);
    assert_eq!(
        dave.core
            .decide_group_commit(&g, e4, &named.commit, NOW)
            .unwrap(),
        CommitDecision::Waiting
    );
    read(&mut dave, &g, &rival_commit).unwrap();
    assert_eq!(epoch(&dave, &g), e4);
    read(&mut dave, &g, &named_commit).unwrap();
    let his = dave.core.group(&g).unwrap();
    assert_eq!(his.epoch, e4 + 1);
    assert!(!his.members.contains(&carol.id));
    assert!(his.members.contains(&erin.id));
}

#[test]
fn only_the_owner_and_admins_change_a_group_within_their_role() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = team(&mut alice, &mut bob, &mut carol);
    // A plain member changes nothing.
    let what = add(&mut dave);
    assert!(matches!(
        carol.core.change_group(&g, what, "x-1", NOW),
        Err(CoreError::Unauthorized)
    ));
    // Bob and Carol become admins.
    let both = admins(&[&bob, &carol]);
    change(&mut alice, &mut [&mut bob, &mut carol], &g, both, "c-1");
    assert_eq!(carol.core.group(&g).unwrap().role, "admin");
    // An admin removes neither the owner nor another admin, and changes no
    // roster.
    for what in [remove(&alice), remove(&carol), admins(&[&bob])] {
        assert!(matches!(
            bob.core.change_group(&g, what, "x-2", NOW),
            Err(CoreError::Unauthorized)
        ));
    }
    // Nobody removes the owner; a roster names members only.
    assert!(matches!(
        alice.core.change_group(&g, remove(&alice), "x-3", NOW),
        Err(CoreError::InvalidInput)
    ));
    assert!(matches!(
        alice.core.change_group(&g, admins(&[&dave]), "x-4", NOW),
        Err(CoreError::InvalidInput)
    ));
    // An empty change, a stranger to remove, a member to add again.
    let again = add(&mut carol);
    for what in [GroupChange::default(), remove(&dave), again] {
        assert!(matches!(
            alice.core.change_group(&g, what, "x-5", NOW),
            Err(CoreError::InvalidInput)
        ));
    }
    // The owner removes an admin: it leaves the roster too.
    let mut dave_in = person("Dave2");
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        add(&mut dave_in),
        "c-r0",
    );
    deliver_invites(&mut alice, &mut [&mut dave_in]);
    change(
        &mut alice,
        &mut [&mut bob, &mut dave_in],
        &g,
        remove(&carol),
        "c-r1",
    );
    for p in [&alice, &bob, &dave_in] {
        assert_eq!(p.core.group(&g).unwrap().admins, [bob.id.clone()]);
    }
    // The owner replaces the admins with none: Bob is a member again,
    // everywhere, and can change nothing.
    change(
        &mut alice,
        &mut [&mut bob, &mut dave_in],
        &g,
        admins(&[]),
        "c-2",
    );
    for p in [&alice, &bob, &dave_in] {
        assert!(p.core.group(&g).unwrap().admins.is_empty());
    }
    assert_eq!(bob.core.group(&g).unwrap().role, "member");
    let what = add(&mut dave);
    assert!(matches!(
        bob.core.change_group(&g, what, "x-6", NOW),
        Err(CoreError::Unauthorized)
    ));
    // A name and cards are checked before anything is made.
    assert!(matches!(
        alice.core.create_group(" ", &[], "g-2", NOW),
        Err(CoreError::InvalidInput)
    ));
    let mut forged = invitee(&mut dave);
    forged.network_id = carol.id.clone();
    assert!(matches!(
        alice.core.create_group("Other", &[forged], "g-3", NOW),
        Err(CoreError::Unauthorized)
    ));
    assert_eq!(alice.core.groups().unwrap().len(), 1);
}

/// The owner bans an id: a member it removes in the same commit, an id that
/// never was a member ahead. Nobody adds a banned id back, an admin does not
/// lift the owner's ban, and the owner's unban lets it be invited again. A
/// newcomer sees the bans everyone sees.
#[test]
fn the_owner_bans_an_id_and_nobody_adds_it_back_until_unbanned() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = team(&mut alice, &mut bob, &mut carol);
    let bob_admin = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        bob_admin,
        "c-1",
    );
    let e = epoch(&alice, &g);
    let carol_id = carol.id.clone();
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        ban(&[&carol_id]),
        "c-2",
    );
    assert_eq!(epoch(&alice, &g), e + 1);
    let both = sorted(vec![alice.id.clone(), bob.id.clone()]);
    for p in [&alice, &bob] {
        assert_eq!(sorted(p.core.group(&g).unwrap().members), both);
        assert_eq!(bans(p, &g), [(carol.id.clone(), true)]);
    }
    assert!(carol.core.swarm_mailbox(&g, true, NOW).is_err());
    // Neither the owner nor an admin adds her back.
    for (p, op) in [(&mut alice, "x-1"), (&mut bob, "x-2")] {
        let what = add(&mut carol);
        assert!(matches!(
            p.core.change_group(&g, what, op, NOW),
            Err(CoreError::Banned)
        ));
    }
    // An admin does not lift the owner's ban.
    assert!(matches!(
        bob.core.change_group(&g, unban(&[&carol.id]), "x-3", NOW),
        Err(CoreError::Unauthorized)
    ));
    // An id that never was a member is banned ahead.
    let dave_id = dave.id.clone();
    change(&mut alice, &mut [&mut bob], &g, ban(&[&dave_id]), "c-3");
    let mut expected = vec![(carol.id.clone(), true), (dave.id.clone(), true)];
    expected.sort();
    assert_eq!(bans(&bob, &g), expected);
    assert_eq!(sorted(bob.core.group(&g).unwrap().members), both);
    let what = add(&mut dave);
    assert!(matches!(
        bob.core.change_group(&g, what, "x-4", NOW),
        Err(CoreError::Banned)
    ));
    // The owner lifts Carol's ban: an admin invites her again, she joins
    // and sees the bans the others see.
    change(&mut alice, &mut [&mut bob], &g, unban(&[&carol_id]), "c-4");
    assert_eq!(bans(&alice, &g), [(dave.id.clone(), true)]);
    change(&mut bob, &mut [&mut alice], &g, add(&mut carol), "c-5");
    let outcomes = deliver_invites(&mut bob, &mut [&mut carol]);
    assert!(matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g));
    assert_eq!(
        sorted(alice.core.group(&g).unwrap().members),
        sorted(vec![alice.id.clone(), bob.id.clone(), carol.id.clone()])
    );
    assert_eq!(bans(&carol, &g), [(dave.id.clone(), true)]);
    let hello = say(&mut carol, &g, "I am back", "m-1");
    read(&mut alice, &g, &hello).unwrap();
    assert_eq!(texts(&alice, &g).last().unwrap().1, "I am back");
}

/// An admin bans and unbans plain members, as it removes them: not the
/// owner, not another admin, and a plain member bans nobody. The owner
/// banning an admin removes it from the group and from the admins.
#[test]
fn an_admin_bans_plain_members_and_the_owner_bans_anyone_but_itself() {
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
        "c-0",
    );
    deliver_invites(&mut alice, &mut [&mut dave]);
    let both = admins(&[&bob, &carol]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol, &mut dave],
        &g,
        both,
        "c-1",
    );
    let (alice_id, carol_id, dave_id) = (alice.id.clone(), carol.id.clone(), dave.id.clone());
    // A plain member bans nobody; an admin bans neither the owner nor
    // another admin.
    assert!(matches!(
        dave.core.change_group(&g, ban(&[&carol_id]), "x-1", NOW),
        Err(CoreError::Unauthorized)
    ));
    for what in [ban(&[&alice_id]), ban(&[&carol_id])] {
        assert!(matches!(
            bob.core.change_group(&g, what, "x-1", NOW),
            Err(CoreError::Unauthorized)
        ));
    }
    change(
        &mut bob,
        &mut [&mut alice, &mut carol, &mut dave],
        &g,
        ban(&[&dave_id]),
        "c-2",
    );
    for p in [&alice, &bob, &carol] {
        assert_eq!(bans(p, &g), [(dave.id.clone(), false)]);
        assert!(!p.core.group(&g).unwrap().members.contains(&dave.id));
    }
    // Another admin lifts an admin's ban.
    change(
        &mut carol,
        &mut [&mut alice, &mut bob],
        &g,
        unban(&[&dave_id]),
        "c-3",
    );
    for p in [&alice, &bob, &carol] {
        assert!(bans(p, &g).is_empty());
    }
    // Nobody bans the owner, only a banned id is unbanned, and an id is
    // well formed; an id is banned once.
    for what in [ban(&[&alice_id]), unban(&[&dave_id]), ban(&["ain1xyz"])] {
        assert!(matches!(
            alice.core.change_group(&g, what, "x-2", NOW),
            Err(CoreError::InvalidInput)
        ));
    }
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        ban(&[&dave_id]),
        "c-4",
    );
    assert!(matches!(
        alice.core.change_group(&g, ban(&[&dave_id]), "x-3", NOW),
        Err(CoreError::InvalidInput)
    ));
    // The owner bans an admin: out of the group and of the admins.
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        ban(&[&carol_id]),
        "c-5",
    );
    for p in [&alice, &bob] {
        let info = p.core.group(&g).unwrap();
        assert_eq!(info.admins, [bob.id.clone()]);
        assert_eq!(
            sorted(info.members),
            sorted(vec![alice.id.clone(), bob.id.clone()])
        );
    }
    let mut expected = vec![(carol.id.clone(), true), (dave.id.clone(), true)];
    expected.sort();
    assert_eq!(bans(&bob, &g), expected);
}

#[test]
fn a_group_invitation_meets_the_recipients_policy() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    bob.core
        .set_intro_policy(IntroPolicy {
            mode: IntroMode::Manual,
            daily_limit: 20,
            allowed: vec![],
        })
        .unwrap();
    let invite = |alice: &mut Person, bob: &mut Person, name: &str, op: &str| {
        let invitees = vec![invitee(bob)];
        let g = alice
            .core
            .create_group(name, &invitees, op, NOW)
            .unwrap()
            .id;
        (g, deliver_invites(alice, &mut [bob]))
    };
    // A stranger's invitation waits, listed with the group.
    let (g, outcomes) = invite(&mut alice, &mut bob, "Club", "g-1");
    let [IntroOutcome::Pending(waiting)] = outcomes.as_slice() else {
        panic!("a manual policy holds the invitation: {outcomes:?}");
    };
    assert_eq!(
        (
            waiting.network_id.as_str(),
            waiting.name.as_str(),
            waiting.group.as_deref()
        ),
        (alice.id.as_str(), "Club", Some(g.as_str()))
    );
    assert!(bob.core.groups().unwrap().is_empty());
    // One per inviter and group: a second group waits beside it.
    let (_, outcomes) = invite(&mut alice, &mut bob, "Second", "g-2");
    assert!(matches!(outcomes.as_slice(), [IntroOutcome::Pending(_)]));
    assert_eq!(bob.core.intro_requests().unwrap().len(), 2);
    let joined = bob
        .core
        .accept_intro_request(&waiting.request_id, NOW)
        .unwrap();
    assert_eq!(
        (joined.id.as_str(), joined.title.as_str()),
        (g.as_str(), "Club")
    );
    assert_eq!(bob.core.group(&g).unwrap().role, "member");
    // Being a contact lets the inviter in, even under a manual policy, and
    // again for another group.
    connect(&mut alice.core, &mut bob.core);
    for (name, op) in [("Third", "g-3"), ("Fourth", "g-4")] {
        let (g, outcomes) = invite(&mut alice, &mut bob, name, op);
        assert!(
            matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g),
            "{name}: {outcomes:?}"
        );
    }
    // So does the list.
    let mut carol = person("Carol");
    bob.core
        .set_intro_policy(IntroPolicy {
            mode: IntroMode::Manual,
            daily_limit: 20,
            allowed: vec![carol.id.clone()],
        })
        .unwrap();
    let (g, outcomes) = invite(&mut carol, &mut bob, "Fifth", "g-5");
    assert!(matches!(outcomes.as_slice(), [IntroOutcome::Joined(c)] if c.id == g));
}

/// What a notary checks: the roster is the owner's for this group, and the
/// committer is the owner or one of its admins; and what a reader checks of
/// a commit's roster change.
#[test]
fn a_commit_claim_holds_only_under_the_owners_roster() {
    let owner = SigningKey::from_bytes(&[31; 32]);
    let admin = SigningKey::from_bytes(&[32; 32]);
    let member = SigningKey::from_bytes(&[33; 32]);
    let root = |k: &SigningKey| k.verifying_key().to_bytes();
    let group_id = [44; 32];
    let g = group_ref(&DOMAIN, &root(&owner), &group_id);
    assert_ne!(g, group_ref(&DOMAIN, &root(&admin), &group_id));
    assert_ne!(g, group_ref(&DOMAIN, &root(&owner), &[45; 32]));
    assert_ne!(g, group_ref(&[8; 32], &root(&owner), &group_id));
    let document = |kind: DocumentKind, body: Vec<u8>, key: &SigningKey, domain: [u8; 32]| {
        SignedDocument::sign(
            DocumentDraft {
                domain,
                kind,
                authority_epoch: 0,
                issued_at: NOW,
                expires_at: None,
                body,
                extensions: BTreeMap::new(),
            },
            key,
        )
        .unwrap()
        .to_wire()
    };
    let roster_of = |group: [u8; 32], version: u64, admins: Vec<[u8; 32]>| Roster {
        group,
        version,
        admins,
        access: Access::Private,
        kind: agentic_protocol::group::GroupKind::Group,
    };
    let roster = |group: [u8; 32], key: &SigningKey| {
        document(
            DocumentKind::GroupRoster,
            roster_of(group, 2, vec![root(&admin)]).encode(),
            key,
            DOMAIN,
        )
    };
    let claim_body = |roster: Vec<u8>, epoch: u64, round: u32| {
        CommitClaim {
            owner: root(&owner),
            group_id,
            epoch,
            round,
            commit: [9; 32],
            roster,
        }
        .encode()
    };
    let claim = |roster: Vec<u8>, epoch: u64, round: u32, key: &SigningKey| {
        document(
            DocumentKind::GroupCommit,
            claim_body(roster, epoch, round),
            key,
            DOMAIN,
        )
    };
    let good = roster(g, &owner);
    for key in [&owner, &admin] {
        let verified = verify_claim(&claim(good.clone(), 5, 0, key), DOMAIN, NOW).unwrap();
        assert_eq!(verified.committer, root(key));
        assert_eq!(verified.roster.version, 2);
        assert_eq!((verified.claim.epoch, verified.claim.commit), (5, [9; 32]));
    }
    let key_of = |epoch, round| {
        verify_claim(&claim(good.clone(), epoch, round, &owner), DOMAIN, NOW)
            .unwrap()
            .key
    };
    assert_eq!(key_of(5, 0), key_of(5, 0));
    assert_ne!(key_of(5, 0), key_of(6, 0));
    assert_ne!(key_of(5, 0), key_of(5, 1));
    // Same key whoever claims it: a race is decided at one place.
    assert_eq!(
        verify_claim(&claim(good.clone(), 5, 0, &admin), DOMAIN, NOW)
            .unwrap()
            .key,
        key_of(5, 0)
    );
    let rejected = [
        ("a plain member", claim(good.clone(), 5, 0, &member)),
        (
            "a roster by another key",
            claim(roster(g, &admin), 5, 0, &admin),
        ),
        (
            "another group's roster",
            claim(
                roster(group_ref(&DOMAIN, &root(&owner), &[45; 32]), &owner),
                5,
                0,
                &owner,
            ),
        ),
        (
            "a roster that is not one",
            claim(b"nonsense".to_vec(), 5, 0, &owner),
        ),
        ("a roster as the claim", good.clone()),
        (
            "a claim of the roster kind",
            document(
                DocumentKind::GroupRoster,
                claim_body(good.clone(), 5, 0),
                &owner,
                DOMAIN,
            ),
        ),
        (
            "another domain",
            document(
                DocumentKind::GroupCommit,
                claim_body(good.clone(), 5, 0),
                &owner,
                [8; 32],
            ),
        ),
    ];
    for (label, wire) in rejected {
        assert!(verify_claim(&wire, DOMAIN, NOW).is_err(), "{label}");
    }
    // A reader's checks of a commit's roster change.
    let (o, a, m) = (root(&owner), root(&admin), root(&member));
    let v2 = roster_of(g, 2, vec![a]);
    assert!(check_transition(&v2, &v2, o, o, &[m]).is_ok());
    assert!(check_transition(&v2, &v2, o, a, &[m]).is_ok());
    assert!(check_transition(&v2, &roster_of(g, 3, vec![]), o, o, &[a]).is_ok());
    // Only the owner opens or closes a group, in a new roster version.
    let open = |version| Roster {
        access: Access::Public,
        kind: agentic_protocol::group::GroupKind::Group,
        ..roster_of(g, version, vec![a])
    };
    assert!(check_transition(&v2, &open(3), o, o, &[]).is_ok());
    assert!(check_transition(&open(3), &roster_of(g, 4, vec![a]), o, o, &[]).is_ok());
    // An admin still commits while the group is open.
    assert!(check_transition(&open(3), &open(3), o, a, &[m]).is_ok());
    for (label, next, committer, removed) in [
        ("a member commits", v2.clone(), m, vec![]),
        ("an admin removes the owner", v2.clone(), a, vec![o]),
        ("the owner removes itself", v2.clone(), o, vec![o]),
        ("an admin removes an admin", v2.clone(), a, vec![a]),
        (
            "an admin changes the roster",
            roster_of(g, 3, vec![a, m]),
            a,
            vec![],
        ),
        ("a version jumps", roster_of(g, 4, vec![a]), o, vec![]),
        ("a version goes back", roster_of(g, 1, vec![]), o, vec![]),
        (
            "another group's roster",
            roster_of([1; 32], 3, vec![a]),
            o,
            vec![],
        ),
        (
            "same version, other admins",
            roster_of(g, 2, vec![a, m]),
            a,
            vec![],
        ),
        (
            "same version, other admins by the owner",
            roster_of(g, 2, vec![]),
            o,
            vec![],
        ),
        (
            "an admin removed but kept in the roster",
            v2.clone(),
            o,
            vec![a],
        ),
        (
            "an admin opens the group in the same version",
            open(2),
            a,
            vec![],
        ),
        ("the owner opens it in the same version", open(2), o, vec![]),
    ] {
        assert!(
            check_transition(&v2, &next, o, committer, &removed).is_err(),
            "{label}"
        );
    }
}

/// What a reader checks of a commit's change to the bans, which ride in the
/// MLS group context. Every id here is a network id's digest (its `ain1`
/// hex), not a root key; `admins` are the admins before the commit and
/// `members` the members after it. The owner's bans are marked the owner's
/// and only the owner lifts or re-marks them; an admin bans and unbans plain
/// members; nobody bans the owner; no banned id is a member.
#[test]
fn a_commit_changes_the_bans_only_within_its_committers_role() {
    let [o, a, b, m, x, y] = [1u8, 2, 3, 4, 5, 6].map(|n| [n; 32]);
    let ban = |id: [u8; 32], by_owner: bool| Ban { id, by_owner };
    let bans = |entries: &[Ban]| Bans {
        entries: entries.to_vec(),
        ..Bans::default()
    };
    let none = bans(&[]);
    let admins = [a, b];
    let everyone = [o, a, b, m];
    // Everyone but whom a commit bans.
    let (but_o, but_b, but_m) = ([a, b, m], [o, a, m], [o, a, b]);
    type Case<'a> = (&'a str, Bans, Bans, [u8; 32], &'a [[u8; 32]]);
    let ok: Vec<Case> = vec![
        (
            "the owner bans an id ahead",
            none.clone(),
            bans(&[ban(x, true)]),
            o,
            &everyone[..],
        ),
        (
            "an admin bans an id ahead",
            none.clone(),
            bans(&[ban(x, false)]),
            a,
            &everyone[..],
        ),
        (
            "an admin bans a member who leaves",
            none.clone(),
            bans(&[ban(m, false)]),
            a,
            &but_m[..],
        ),
        (
            "the owner bans an admin who leaves",
            none.clone(),
            bans(&[ban(b, true)]),
            o,
            &but_b[..],
        ),
        (
            "an admin lifts an admin's ban",
            bans(&[ban(x, false)]),
            none.clone(),
            a,
            &everyone[..],
        ),
        (
            "the owner lifts any",
            bans(&[ban(x, false), ban(y, true)]),
            none.clone(),
            o,
            &everyone[..],
        ),
        (
            "nothing changes",
            bans(&[ban(x, true)]),
            bans(&[ban(x, true)]),
            a,
            &everyone[..],
        ),
    ];
    for (label, before, after, committer, members) in ok {
        assert!(
            check_bans(&before, &after, o, committer, &admins, members).is_ok(),
            "{label}"
        );
    }
    // The members after leave out whom a refused ban names: only the role
    // rule refuses it.
    let refused: Vec<Case> = vec![
        (
            "an admin's ban marked the owner's",
            none.clone(),
            bans(&[ban(x, true)]),
            a,
            &everyone[..],
        ),
        (
            "the owner's ban marked an admin's",
            none.clone(),
            bans(&[ban(x, false)]),
            o,
            &everyone[..],
        ),
        (
            "an admin lifts the owner's ban",
            bans(&[ban(x, true)]),
            none.clone(),
            a,
            &everyone[..],
        ),
        (
            "an admin re-marks the owner's ban",
            bans(&[ban(x, true)]),
            bans(&[ban(x, false)]),
            a,
            &everyone[..],
        ),
        (
            "an admin bans the owner",
            none.clone(),
            bans(&[ban(o, false)]),
            a,
            &but_o[..],
        ),
        (
            "the owner bans itself",
            none.clone(),
            bans(&[ban(o, true)]),
            o,
            &but_o[..],
        ),
        (
            "an admin bans an admin",
            none.clone(),
            bans(&[ban(b, false)]),
            a,
            &but_b[..],
        ),
        (
            "a plain member changes the bans",
            none.clone(),
            bans(&[ban(x, false)]),
            m,
            &everyone[..],
        ),
        (
            "a banned member stays",
            none.clone(),
            bans(&[ban(m, true)]),
            o,
            &everyone[..],
        ),
        (
            "a banned id is added back",
            bans(&[ban(m, true)]),
            bans(&[ban(m, true)]),
            a,
            &everyone[..],
        ),
    ];
    for (label, before, after, committer, members) in refused {
        assert!(
            check_bans(&before, &after, o, committer, &admins, members).is_err(),
            "{label}"
        );
    }
    // One entry per id, at most MAX_BANS.
    let two = bans(&[ban(x, true), ban(y, false)]);
    assert_eq!(Bans::decode(&two.encode()).unwrap(), two);
    let twice = bans(&[ban(x, true), ban(x, false)]);
    let many = Bans {
        entries: (0..=MAX_BANS as u16)
            .map(|i| {
                let mut id = [0; 32];
                id[..2].copy_from_slice(&i.to_be_bytes());
                ban(id, true)
            })
            .collect(),
        ..Bans::default()
    };
    for bad in [twice, many] {
        assert!(
            Bans::decode(&bad.encode()).is_err(),
            "{} entries",
            bad.entries.len()
        );
    }
}

/// The owner's inbox covers groups (spec/owner-cli-v1.md): other members'
/// messages wait until acknowledged, the owner's own are passed over.
#[test]
fn the_owner_inbox_pages_a_groups_incoming_messages() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let g = team(&mut alice, &mut bob, &mut carol);
    say(&mut alice, &g, "the plan for Saturday", "a-1");
    for (text, op) in [
        ("I will take the tent", "c-1"),
        ("and the cooking pot", "c-2"),
    ] {
        let d = say(&mut carol, &g, text, op);
        read(&mut alice, &g, &d).unwrap();
    }
    let unread: Vec<_> = alice
        .core
        .owner_inbox_unread(NOW)
        .unwrap()
        .into_iter()
        .map(|u| (u.conversation_id, u.unread, u.leased))
        .collect();
    assert_eq!(unread, [(g.clone(), 2, false)]);
    let page = alice.core.owner_inbox_poll(&g, 10, 60, NOW).unwrap();
    let items: Vec<_> = page
        .items
        .iter()
        .map(|i| (i.author.as_str(), i.text.as_str()))
        .collect();
    assert_eq!(
        items,
        [
            (carol.id.as_str(), "I will take the tent"),
            (carol.id.as_str(), "and the cooking pot")
        ]
    );
    assert_eq!(page.conversation_id, g);
    assert!(alice.core.owner_inbox_unread(NOW).unwrap()[0].leased);
    alice
        .core
        .owner_inbox_ack(&g, page.lease_id.as_deref().unwrap(), NOW)
        .unwrap();
    assert!(alice.core.owner_inbox_unread(NOW).unwrap().is_empty());
    let next = alice.core.owner_inbox_poll(&g, 10, 60, NOW).unwrap();
    assert!(next.items.is_empty() && next.lease_id.is_none());
}

/// A member's messages are taken in the order they were written: one that
/// arrives before its predecessor is refused as a gap, with its place in
/// its sender's order known, and another member's messages do not wait.
#[test]
fn a_members_messages_are_taken_in_the_order_they_were_written() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let g = team(&mut alice, &mut bob, &mut carol);
    let first = say(&mut carol, &g, "I will take the tent", "c-1");
    let second = say(&mut carol, &g, "and the cooking pot", "c-2");
    let from_bob = say(&mut bob, &g, "I am on groceries", "b-1");
    let order = |p: &Person, d: &SwarmDelivery| {
        p.core
            .swarm_envelope_order(&g, d.period, &d.envelope, NOW)
            .unwrap()
    };
    let (one, two, bobs) = (
        order(&alice, &first),
        order(&alice, &second),
        order(&alice, &from_bob),
    );
    assert_eq!((two.epoch, &two.sender), (one.epoch, &one.sender));
    assert_eq!(two.generation, one.generation + 1);
    assert_ne!(bobs.sender, one.sender);
    assert!(matches!(
        read(&mut alice, &g, &second),
        Err(CoreError::Crypto(agentic_crypto::CryptoError::ReceiveGap))
    ));
    // Bob's message does not wait for Carol's.
    read(&mut alice, &g, &from_bob).unwrap();
    assert_eq!(
        texts(&alice, &g),
        [(bob.id.clone(), "I am on groceries".to_owned())]
    );
    read(&mut alice, &g, &first).unwrap();
    read(&mut alice, &g, &second).unwrap();
    assert_eq!(
        texts(&alice, &g),
        [
            (bob.id.clone(), "I am on groceries".to_owned()),
            (carol.id.clone(), "I will take the tent".to_owned()),
            (carol.id.clone(), "and the cooking pot".to_owned()),
        ]
    );
}

/// The desktop window's list and history (V1-GF01): a group is listed with
/// the contacts in one id-ordered, paged sequence, with its last message and
/// unread count, and its history pages like a direct conversation's, each
/// message with its author.
#[test]
fn the_desktop_lists_a_group_with_the_contacts_and_pages_its_history() {
    let (mut alice, mut bob, mut carol) = (person("Alice"), person("Bob"), person("Carol"));
    let mut dave = person("Dave");
    let direct = connect(&mut bob.core, &mut dave.core);
    let g = team(&mut alice, &mut bob, &mut carol);
    // The owner sees the group at once, before anyone wrote in it.
    let made = alice.core.desktop_overview(None).unwrap();
    let listed = made
        .conversations
        .iter()
        .find(|c| c.id == g)
        .expect("the owner's new group");
    assert_eq!((listed.title.as_str(), listed.unread), ("Team", 0));
    assert!(listed.messages.is_empty());

    let first = say(&mut alice, &g, "Hello team", "m-1");
    read(&mut bob, &g, &first).unwrap();
    let own = say(&mut bob, &g, "Bob is in", "b-1");
    read(&mut carol, &g, &own).unwrap();
    let reply = say(&mut carol, &g, "Carol here", "c-1");
    read(&mut bob, &g, &reply).unwrap();

    let overview = bob.core.desktop_overview(None).unwrap();
    let mut both = vec![direct.clone(), g.clone()];
    both.sort();
    assert_eq!(
        overview
            .conversations
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
        both,
        "contacts and groups in one id order"
    );
    let listed = overview.conversations.iter().find(|c| c.id == g).unwrap();
    assert_eq!(listed.title, "Team");
    assert_eq!(listed.unread, 2, "one's own message is not unread");
    assert_eq!(
        listed
            .messages
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>(),
        ["Carol here"]
    );
    // One cursor pages both kinds.
    let after_first = bob.core.desktop_overview(Some(&both[0])).unwrap();
    assert_eq!(
        after_first
            .conversations
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
        [both[1].clone()]
    );
    assert!(
        bob.core
            .desktop_overview(Some(&both[1]))
            .unwrap()
            .conversations
            .is_empty()
    );

    let history = bob.core.conversation_history(&g, None).unwrap();
    assert_eq!(
        history
            .messages
            .iter()
            .map(|m| (m.author.as_str(), m.text.as_str(), m.own))
            .collect::<Vec<_>>(),
        [
            (alice.id.as_str(), "Hello team", false),
            (bob.id.as_str(), "Bob is in", true),
            (carol.id.as_str(), "Carol here", false),
        ]
    );
    // A group message id is a history cursor.
    let older = bob
        .core
        .conversation_history(&g, Some(&history.messages[2].id))
        .unwrap();
    assert_eq!(
        older
            .messages
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>(),
        ["Hello team", "Bob is in"]
    );
}

// --- open-read groups (Docs/V1_DISCOVERY_2026_09_27.md, part 2) --------------

/// The group's reference `G`, as a member sees it.
fn reference(p: &Person, group: &str) -> [u8; 32] {
    hex::decode(p.core.group(group).unwrap().group_ref)
        .unwrap()
        .try_into()
        .unwrap()
}

/// `p`'s current public roster, as it publishes it.
fn public_roster(p: &mut Person, group: &str) -> SwarmDelivery {
    p.core
        .public_roster(group, NOW)
        .unwrap()
        .expect("an owner or admin of an open group or a channel publishes its roster")
}

/// A signed document of `kind` by `key`.
fn signed(kind: DocumentKind, body: Vec<u8>, key: &SigningKey) -> Vec<u8> {
    SignedDocument::sign(
        DocumentDraft {
            domain: DOMAIN,
            kind,
            authority_epoch: 0,
            issued_at: NOW,
            expires_at: None,
            body,
            extensions: BTreeMap::new(),
        },
        key,
    )
    .unwrap()
    .to_wire()
}

/// An entry of a public mailbox: a post (1) or a roster (2).
fn entry(tag: u8, wire: &[u8]) -> Vec<u8> {
    [&[tag][..], wire].concat()
}

/// A network id's digest: what certificates and removals name.
fn digest_of(key: &SigningKey) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(key.verifying_key().to_bytes()).into()
}

/// A post into `gref`'s public mailbox by `key`, which is no member: its
/// certificate stands on a roster `key` signed itself for G.
fn impostor(key: &SigningKey, gref: [u8; 32], epoch: u64) -> Vec<u8> {
    use agentic_protocol::group::MemberCert;
    let roster = signed(
        DocumentKind::GroupRoster,
        Roster {
            group: gref,
            version: 1,
            admins: vec![],
            access: Access::Public,
            kind: agentic_protocol::group::GroupKind::Group,
        }
        .encode(),
        key,
    );
    let cert = signed(
        DocumentKind::GroupMember,
        MemberCert {
            group: gref,
            member: digest_of(key),
            epoch,
            roster,
        }
        .encode(),
        key,
    );
    let post = PublicPost {
        group: gref,
        epoch,
        operation: [6; 32],
        text: "I am the owner here".into(),
        membership: cert,
    };
    entry(1, &signed(DocumentKind::PublicPost, post.encode(), key))
}

/// Someone to invite whose profile is closed again at once: a crowd of
/// open profiles would hold too many files.
fn card_of(name: &str) -> Invitee {
    invitee(&mut person(name))
}

/// `from` writes `text` at `at`: its delivery.
fn say_at(from: &mut Person, group: &str, text: &str, op: &str, at: u64) -> SwarmDelivery {
    let sent = from.core.send_message(group, text, op, at).unwrap();
    from.core.prepare_swarm_delivery(&sent.id, at).unwrap()
}

fn opened(alice: &mut Person, bob: &mut Person, carol: &mut Person) -> String {
    let g = team(alice, bob, carol);
    change(alice, &mut [bob, carol], &g, access(Access::Public), "open");
    g
}

#[test]
fn an_owner_opens_a_group_to_read_and_its_members_post_in_the_clear() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let g = team(&mut alice, &mut bob, &mut carol);
    let gref = reference(&alice, &g);
    assert_eq!(alice.core.group(&g).unwrap().access, "private");
    // A private group publishes no roster.
    assert!(alice.core.public_roster(&g, NOW).unwrap().is_none());
    // Only the owner opens it, even to an admin.
    let bob_admin = admins(&[&bob]);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        bob_admin,
        "c-1",
    );
    assert!(matches!(
        bob.core
            .change_group(&g, access(Access::Public), "x-1", NOW),
        Err(CoreError::Unauthorized)
    ));
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        access(Access::Public),
        "c-2",
    );
    for p in [&alice, &bob, &carol] {
        assert_eq!(p.core.group(&g).unwrap().access, "public");
    }
    // A plain member publishes no roster; the owner and admins do.
    assert!(carol.core.public_roster(&g, NOW).unwrap().is_none());
    // A post is a document signed by its author, in the clear, in the
    // public mailbox anyone can compute from G.
    let posted = say(&mut carol, &g, "open to everyone", "p-1");
    assert_eq!(
        posted.mailbox,
        public_group_mailbox_id(&DOMAIN, &gref, period(NOW))
    );
    assert_eq!(posted.envelope[0], 1);
    let post = verify_public_post(&posted.envelope[1..], DOMAIN, NOW, &gref).unwrap();
    assert_eq!(
        (post.author, post.text.as_str()),
        (carol.root, "open to everyone")
    );
    assert_eq!(
        texts(&carol, &g),
        [(carol.id.clone(), "open to everyone".to_owned())]
    );
    // Members take it from the public mailbox under their own view of the
    // group, whether or not a roster was published; once.
    for _ in 0..2 {
        bob.core
            .receive_public_entry(&g, &posted.envelope, NOW, NOW)
            .unwrap();
    }
    carol
        .core
        .receive_public_entry(&g, &posted.envelope, NOW, NOW)
        .unwrap();
    assert_eq!(
        texts(&bob, &g),
        [(carol.id.clone(), "open to everyone".to_owned())]
    );
    assert_eq!(texts(&carol, &g).len(), 1, "the author keeps one copy");
    // A post by someone who is not a member is passed over by members too,
    // even with a certificate under a roster it signed itself for G.
    let stranger = SigningKey::from_bytes(&[77; 32]);
    assert_eq!(
        bob.core
            .receive_public_entry(&g, &impostor(&stranger, gref, epoch(&alice, &g)), NOW, NOW)
            .unwrap(),
        PublicEntry::Ignored
    );
    // Closed again: the next message is sealed in the epoch's mailbox, and
    // after the closing roster nothing is published.
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        access(Access::Private),
        "c-3",
    );
    let sealed = say(&mut alice, &g, "members only again", "p-2");
    assert_eq!(
        sealed.mailbox,
        alice.core.group_mailboxes(&g, NOW).unwrap()[0]
    );
    read(&mut bob, &g, &sealed).unwrap();
    assert_eq!(
        texts(&bob, &g).last().unwrap(),
        &(alice.id.clone(), "members only again".to_owned())
    );
    public_roster(&mut alice, &g);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        admins(&[]),
        "c-4",
    );
    assert!(alice.core.public_roster(&g, NOW).unwrap().is_none());
}

#[test]
fn a_follower_reads_the_writers_posts_and_stops_when_the_group_closes() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = opened(&mut alice, &mut bob, &mut carol);
    let gref = reference(&alice, &g);
    // The roster is published before Dave follows; a post of the day before
    // he followed is history he gets.
    let roster = public_roster(&mut alice, &g);
    let early = say(&mut alice, &g, "before subscribing", "p-0");
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "Team", NOW)
        .unwrap();
    assert_eq!(follow.id, hex::encode(gref));
    assert!(!follow.closed);
    let receive = |dave: &mut Person, entry: &[u8], at: u64| {
        dave.core
            .receive_public_entry(&follow.id, entry, at, NOW)
            .unwrap()
    };
    assert_eq!(
        receive(&mut dave, &roster.envelope, NOW - 1),
        PublicEntry::Roster
    );
    assert_eq!(
        receive(&mut dave, &early.envelope, NOW - 1),
        PublicEntry::Post
    );
    let hello = say(&mut carol, &g, "hello readers", "p-1");
    assert_eq!(receive(&mut dave, &hello.envelope, NOW), PublicEntry::Post);
    assert_eq!(
        receive(&mut dave, &hello.envelope, NOW),
        PublicEntry::Ignored,
        "once"
    );
    // Nobody writes into the group but its writers: not a stranger's post,
    // not a roster whose committer is neither the owner nor an admin, not a
    // roster behind which stands another "owner".
    let stranger = SigningKey::from_bytes(&[77; 32]);
    let now_epoch = epoch(&alice, &g);
    let intruder = signed(
        DocumentKind::PublicPost,
        PublicPost {
            group: gref,
            epoch: now_epoch,
            operation: [5; 32],
            text: "not a member".into(),
            membership: vec![],
        }
        .encode(),
        &stranger,
    );
    let owners = PublicRoster::decode(
        agentic_protocol::VerifiedDocument::decode(&roster.envelope[1..], DOMAIN, NOW)
            .unwrap()
            .body(),
    )
    .unwrap()
    .roster;
    let usurper = signed(
        DocumentKind::PublicRoster,
        PublicRoster {
            group: gref,
            epoch: now_epoch + 5,
            roster: owners,
            removed: vec![],
            retention: agentic_protocol::group::Retention::Days(1),
        }
        .encode(),
        &stranger,
    );
    let own_roster = signed(
        DocumentKind::GroupRoster,
        Roster {
            group: gref,
            version: 9,
            admins: vec![],
            access: Access::Public,
            kind: agentic_protocol::group::GroupKind::Group,
        }
        .encode(),
        &stranger,
    );
    let pretender = signed(
        DocumentKind::PublicRoster,
        PublicRoster {
            group: gref,
            epoch: now_epoch + 5,
            roster: own_roster,
            removed: vec![],
            retention: agentic_protocol::group::Retention::Days(1),
        }
        .encode(),
        &stranger,
    );
    for forged in [
        entry(1, &intruder),
        impostor(&stranger, gref, now_epoch),
        entry(2, &usurper),
        entry(2, &pretender),
    ] {
        assert_eq!(receive(&mut dave, &forged, NOW), PublicEntry::Ignored);
    }
    // Carol is removed; each committer publishes the new roster. Once it is
    // read, nothing more of Carol's is taken, not even what she wrote before
    // (she could date anything so); Bob's post is.
    let late = say(&mut carol, &g, "late", "p-2");
    let bobs_late = say(&mut bob, &g, "also late", "p-2b");
    let without_carol = remove(&carol);
    change(
        &mut alice,
        &mut [&mut bob, &mut carol],
        &g,
        without_carol,
        "c-2",
    );
    let removed = public_roster(&mut alice, &g);
    let in_flight = say(&mut bob, &g, "in flight", "p-3");
    let bob_admin = admins(&[&bob]);
    change(&mut alice, &mut [&mut bob], &g, bob_admin, "c-3");
    // Bob, now an admin, publishes while Alice is away: a post carries its
    // author's certificate, so it needs no roster of its epoch.
    let ahead = say(&mut bob, &g, "we continue", "p-4");
    assert_eq!(receive(&mut dave, &ahead.envelope, NOW), PublicEntry::Post);
    assert_eq!(
        receive(&mut dave, &removed.envelope, NOW),
        PublicEntry::Roster
    );
    let by_admin = public_roster(&mut bob, &g);
    assert_eq!(
        receive(&mut dave, &by_admin.envelope, NOW),
        PublicEntry::Roster
    );
    assert_eq!(
        receive(&mut dave, &late.envelope, NOW),
        PublicEntry::Ignored
    );
    // Bob's post of two epochs back is still his: he is a member.
    assert_eq!(
        receive(&mut dave, &bobs_late.envelope, NOW),
        PublicEntry::Post
    );
    assert_eq!(
        receive(&mut dave, &in_flight.envelope, NOW),
        PublicEntry::Post
    );
    assert_eq!(
        texts(&dave, &follow.id),
        [
            (alice.id.clone(), "before subscribing".to_owned()),
            (carol.id.clone(), "hello readers".to_owned()),
            (bob.id.clone(), "we continue".to_owned()),
            (bob.id.clone(), "also late".to_owned()),
            (bob.id.clone(), "in flight".to_owned()),
        ]
    );
    // Closed: the last roster says so, and nothing more is taken, not even
    // a post that was on its way when the group closed.
    let on_its_way = say(&mut bob, &g, "made it before closing", "p-5");
    change(
        &mut alice,
        &mut [&mut bob],
        &g,
        access(Access::Private),
        "c-4",
    );
    let closing = public_roster(&mut alice, &g);
    assert_eq!(
        receive(&mut dave, &closing.envelope, NOW),
        PublicEntry::Closed
    );
    assert!(dave.core.follows().unwrap()[0].closed);
    assert_eq!(
        receive(&mut dave, &on_its_way.envelope, NOW),
        PublicEntry::Ignored
    );
    // A follower only reads; unfollowing ends it.
    assert!(matches!(
        dave.core.send_message(&follow.id, "may I?", "d-1", NOW),
        Err(CoreError::UnknownConversation)
    ));
    dave.core.unfollow_group(&follow.id).unwrap();
    assert!(dave.core.follows().unwrap().is_empty());
}

/// A followed group's posts reach the owner's inbox, for the agent, and the
/// window's history, like a conversation's.
#[test]
fn the_owner_inbox_and_the_window_read_a_followed_groups_posts() {
    let mut alice = person("Alice");
    let mut bob = person("Bob");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let g = opened(&mut alice, &mut bob, &mut carol);
    let gref = reference(&alice, &g);
    let follow = dave
        .core
        .follow_group(gref, &alice.id, "Team", NOW)
        .unwrap();
    let roster = public_roster(&mut alice, &g);
    dave.core
        .receive_public_entry(&follow.id, &roster.envelope, NOW, NOW)
        .unwrap();
    let posted = say(&mut bob, &g, "news of the day", "p-1");
    dave.core
        .receive_public_entry(&follow.id, &posted.envelope, NOW, NOW)
        .unwrap();
    let unread: Vec<_> = dave
        .core
        .owner_inbox_unread(NOW)
        .unwrap()
        .into_iter()
        .map(|u| (u.conversation_id, u.unread))
        .collect();
    assert_eq!(unread, [(follow.id.clone(), 1)]);
    let page = dave.core.owner_inbox_poll(&follow.id, 10, 60, NOW).unwrap();
    assert_eq!(page.items[0].author, bob.id);
    assert_eq!(page.items[0].text, "news of the day");
    let overview = dave.core.desktop_overview(None).unwrap();
    let listed = overview
        .conversations
        .iter()
        .find(|c| c.id == follow.id)
        .expect("the window lists the follow");
    assert_eq!(
        (
            listed.title.as_str(),
            listed.messages.last().map(|m| m.text.as_str())
        ),
        ("Team", Some("news of the day"))
    );
    let history = dave.core.conversation_history(&follow.id, None).unwrap();
    let read: Vec<_> = history
        .messages
        .iter()
        .map(|m| (m.author.as_str(), m.text.as_str(), m.own))
        .collect();
    assert_eq!(read, [(bob.id.as_str(), "news of the day", false)]);
}

/// What a reader checks of a public roster and of a post with its
/// author's certificate: the kind, the group, the domain, and who signed
/// the certificate under which roster.
#[test]
fn a_public_roster_and_post_hold_only_for_their_group_and_kind() {
    use agentic_protocol::group::{MemberCert, verify_public_roster};
    let owner = SigningKey::from_bytes(&[31; 32]);
    let admin = SigningKey::from_bytes(&[32; 32]);
    let member = SigningKey::from_bytes(&[33; 32]);
    let root = |k: &SigningKey| k.verifying_key().to_bytes();
    let gref = group_ref(&DOMAIN, &root(&owner), &[44; 32]);
    let owners = signed(
        DocumentKind::GroupRoster,
        Roster {
            group: gref,
            version: 2,
            admins: vec![root(&admin)],
            access: Access::Public,
            kind: agentic_protocol::group::GroupKind::Group,
        }
        .encode(),
        &owner,
    );
    let public = |group: [u8; 32]| PublicRoster {
        group,
        epoch: 7,
        roster: owners.clone(),
        removed: vec![(digest_of(&member), 5)],
        retention: agentic_protocol::group::Retention::Days(1),
    };
    let by_admin = signed(DocumentKind::PublicRoster, public(gref).encode(), &admin);
    let verified = verify_public_roster(&by_admin, DOMAIN, NOW, &gref).unwrap();
    assert_eq!(
        (verified.owner, verified.committer),
        (root(&owner), root(&admin))
    );
    assert_eq!((verified.epoch, verified.access), (7, Access::Public));
    assert_eq!(verified.removed, [(digest_of(&member), 5)]);
    // The owner's roster of another group of its, where the admin is also
    // an admin, wrapped for this one.
    let elsewhere = signed(
        DocumentKind::GroupRoster,
        Roster {
            group: group_ref(&DOMAIN, &root(&owner), &[45; 32]),
            version: 2,
            admins: vec![root(&admin)],
            access: Access::Public,
            kind: agentic_protocol::group::GroupKind::Group,
        }
        .encode(),
        &owner,
    );
    let smuggled = PublicRoster {
        roster: elsewhere.clone(),
        ..public(gref)
    };
    for (label, wire, group) in [
        ("another group", by_admin.clone(), [1; 32]),
        (
            "the public roster's group is another",
            signed(DocumentKind::PublicRoster, public([1; 32]).encode(), &admin),
            gref,
        ),
        (
            "the owner's roster is another group's",
            signed(DocumentKind::PublicRoster, smuggled.encode(), &admin),
            gref,
        ),
        (
            "not a public roster",
            signed(DocumentKind::GroupRoster, public(gref).encode(), &admin),
            gref,
        ),
    ] {
        assert!(
            verify_public_roster(&wire, DOMAIN, NOW, &group).is_err(),
            "{label}"
        );
    }
    // A post carries its author's certificate of membership: signed by the
    // owner or an admin, with the owner's roster behind it.
    let cert = |who: &SigningKey, by: &SigningKey, roster: &[u8], group: [u8; 32]| {
        signed(
            DocumentKind::GroupMember,
            MemberCert {
                group,
                member: digest_of(who),
                epoch: 6,
                roster: roster.to_vec(),
            }
            .encode(),
            by,
        )
    };
    let post = |membership: Vec<u8>| PublicPost {
        group: gref,
        epoch: 7,
        operation: [1; 32],
        text: "text".into(),
        membership,
    };
    let by_member = signed(
        DocumentKind::PublicPost,
        post(cert(&member, &admin, &owners, gref)).encode(),
        &member,
    );
    let taken = verify_public_post(&by_member, DOMAIN, NOW, &gref).unwrap();
    assert_eq!(
        (taken.author, taken.owner, taken.since),
        (root(&member), root(&owner), 6)
    );
    for (label, membership) in [
        ("no certificate", vec![]),
        (
            "a certificate of another member",
            cert(&admin, &owner, &owners, gref),
        ),
        (
            "a certificate a plain member signed",
            cert(&member, &member, &owners, gref),
        ),
        (
            "a certificate of another group",
            cert(
                &member,
                &admin,
                &elsewhere,
                group_ref(&DOMAIN, &root(&owner), &[45; 32]),
            ),
        ),
        // The owner's roster of another group, where the signer is an
        // admin, inside a certificate for G.
        (
            "a certificate under another group's roster",
            cert(&member, &admin, &elsewhere, gref),
        ),
    ] {
        let wire = signed(DocumentKind::PublicPost, post(membership).encode(), &member);
        assert!(
            verify_public_post(&wire, DOMAIN, NOW, &gref).is_err(),
            "{label}"
        );
    }
    assert!(verify_public_post(&by_member, DOMAIN, NOW, &[1; 32]).is_err());
    let as_roster = signed(
        DocumentKind::PublicRoster,
        post(cert(&member, &admin, &owners, gref)).encode(),
        &member,
    );
    assert!(verify_public_post(&as_roster, DOMAIN, NOW, &gref).is_err());
    assert!(verify_public_post(&by_member, [8; 32], NOW, &gref).is_err());
}
