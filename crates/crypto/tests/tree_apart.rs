#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Big groups send their ratchet tree apart from the Welcome
//! (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 1): a Welcome must
//! fit one mailbox envelope however big the group is, and the newcomer joins
//! with the tree it fetched on its own. Messages are read the way groups
//! read them, in each sender's order (`decrypt_contiguous`).
use agentic_crypto::{MlsClient, Prepared};

const GROUP: [u8; 32] = [0x47; 32];
const AAD: &[u8] = b"agentic-internet/dev-genesis/group";
/// Members already in the group besides its admin.
const MEMBERS: usize = 3;
/// A signed document, and so a mailbox envelope, holds at most this.
const ENVELOPE: usize = 65_536;

fn accept<T>(client: &mut MlsClient, prepared: Prepared<T>) -> T {
    *client = MlsClient::from_state(prepared.next_state);
    prepared.value
}

fn id(i: usize) -> Vec<u8> {
    format!("ain1{i:064x}").into_bytes()
}

/// A client and the last-resort KeyPackage its intro card would carry.
fn invitee(i: usize) -> (MlsClient, Vec<u8>) {
    let mut client = MlsClient::new(&id(i)).unwrap();
    let p = client.last_resort_key_package().unwrap();
    let package = accept(&mut client, p);
    (client, package)
}

/// `committer` adds `packages` and applies its commit: the commit and the
/// Welcome for them.
fn add(committer: &mut MlsClient, packages: &[Vec<u8>]) -> (Vec<u8>, Vec<u8>) {
    let p = committer
        .commit_changes(GROUP, packages, &[], b"", None)
        .unwrap();
    let made = accept(committer, p);
    let p = committer.activate_pending_commit(GROUP).unwrap();
    accept(committer, p);
    (made.commit, made.welcome.unwrap())
}

/// The admin (id 0) of a group of `members` others, the tree kept apart when
/// `apart`.
fn group_of(members: usize, apart: bool) -> MlsClient {
    let mut admin = MlsClient::new(&id(0)).unwrap();
    let p = admin.create_group(GROUP).unwrap();
    accept(&mut admin, p);
    if apart {
        let p = admin.set_tree_apart(GROUP).unwrap();
        accept(&mut admin, p);
    }
    let packages: Vec<_> = (1..=members).map(|i| invitee(i).1).collect();
    add(&mut admin, &packages);
    admin
}

/// `from` (id `sender`) writes `text`; `to` reads it as a group member does.
fn say(from: &mut MlsClient, sender: usize, to: &mut MlsClient, text: &str) {
    let p = from.encrypt(GROUP, text.as_bytes(), AAD).unwrap();
    let wire = accept(from, p).wire;
    // Readers first place it in its sender's order.
    let order = to.inspect_application_message(GROUP, &wire, AAD).unwrap();
    assert_eq!(order.sender, id(sender));
    let p = to.decrypt_contiguous(GROUP, &wire, AAD).unwrap();
    let read = accept(to, p);
    assert_eq!(read.sender, id(sender));
    assert_eq!(read.plaintext, text.as_bytes());
}

fn apply(member: &mut MlsClient, commit: &[u8]) {
    let p = member.apply_commit(GROUP, commit).unwrap();
    accept(member, p);
}

#[test]
fn a_newcomer_joins_with_the_tree_it_fetched_and_talks_both_ways() {
    let mut admin = group_of(MEMBERS, true);
    let n = MEMBERS + 1;
    let (mut newcomer, package) = invitee(n);
    let (_, welcome) = add(&mut admin, &[package]);
    let tree = admin.ratchet_tree(GROUP).unwrap();

    // The Welcome does not carry the tree: alone it cannot be taken.
    assert!(newcomer.join(GROUP, &welcome).is_err());
    let p = newcomer.join_with_tree(GROUP, &welcome, &tree).unwrap();
    accept(&mut newcomer, p);
    assert_eq!(newcomer.epoch(GROUP).unwrap(), admin.epoch(GROUP).unwrap());
    assert_eq!(newcomer.members(GROUP).unwrap().len(), MEMBERS + 2);
    say(&mut admin, 0, &mut newcomer, "welcome");
    say(&mut newcomer, n, &mut admin, "thank you");
}

#[test]
fn only_the_tree_of_the_welcomes_epoch_joins_and_the_welcome_stays_bound() {
    let mut admin = group_of(MEMBERS, true);
    let before = admin.ratchet_tree(GROUP).unwrap();
    let n = MEMBERS + 1;
    let (mut newcomer, package) = invitee(n);
    let (_, welcome) = add(&mut admin, &[package]);
    let current = admin.ratchet_tree(GROUP).unwrap();
    let mut altered = current.clone();
    let middle = altered.len() / 2;
    altered[middle] ^= 1;
    // The admin moves on before the newcomer fetched the tree.
    let (next_commit, _) = add(&mut admin, &[invitee(n + 1).1]);
    let later = admin.ratchet_tree(GROUP).unwrap();

    for (name, tree) in [
        ("the tree before the commit", &before),
        ("the tree of a later epoch", &later),
        ("an altered tree", &altered),
    ] {
        assert!(
            newcomer.join_with_tree(GROUP, &welcome, tree).is_err(),
            "{name} joined"
        );
    }
    assert!(
        newcomer
            .join_with_tree([3; 32], &welcome, &current)
            .is_err(),
        "a Welcome joined another group than the one it was expected for"
    );
    // Nothing was taken by the failures: the right tree still joins, and the
    // newcomer follows the commit it missed.
    let p = newcomer.join_with_tree(GROUP, &welcome, &current).unwrap();
    accept(&mut newcomer, p);
    apply(&mut newcomer, &next_commit);
    say(&mut admin, 0, &mut newcomer, "in the same epoch");

    // Joined once, the same Welcome does not overwrite the group.
    let joined = newcomer.snapshot().as_bytes().to_vec();
    assert!(newcomer.join_with_tree(GROUP, &welcome, &current).is_err());
    assert_eq!(newcomer.snapshot().as_bytes(), joined);
    say(&mut admin, 0, &mut newcomer, "still here");
}

#[test]
fn a_member_who_joined_with_the_tree_apart_reads_and_then_welcomes_without_the_tree() {
    let mut admin = group_of(MEMBERS, true);
    let m = MEMBERS + 1;
    let (mut member, package) = invitee(m);
    let (_, welcome) = add(&mut admin, &[package]);
    let tree = admin.ratchet_tree(GROUP).unwrap();
    let p = member.join_with_tree(GROUP, &welcome, &tree).unwrap();
    accept(&mut member, p);
    say(&mut admin, 0, &mut member, "before the promotion");

    // Made an admin, it adds someone: its Welcome goes without the tree too.
    let (mut next, package) = invitee(m + 1);
    let (commit, welcome) = add(&mut member, &[package]);
    apply(&mut admin, &commit);
    assert!(next.join(GROUP, &welcome).is_err());
    let tree = member.ratchet_tree(GROUP).unwrap();
    assert_eq!(tree, admin.ratchet_tree(GROUP).unwrap());
    let p = next.join_with_tree(GROUP, &welcome, &tree).unwrap();
    accept(&mut next, p);
    say(&mut admin, 0, &mut next, "the third");
    say(&mut next, m + 1, &mut member, "from the third");
}

#[test]
fn a_group_that_sent_its_tree_along_keeps_it_apart_from_then_on() {
    let mut admin = group_of(MEMBERS, false);
    let e = MEMBERS + 1;
    let (mut early, package) = invitee(e);
    let (_, welcome) = add(&mut admin, &[package]);
    // Before the switch the tree rides along, as in every group so far.
    let p = early.join(GROUP, &welcome).unwrap();
    accept(&mut early, p);

    let p = admin.set_tree_apart(GROUP).unwrap();
    accept(&mut admin, p);
    let (mut late, package) = invitee(e + 1);
    let (commit, welcome) = add(&mut admin, &[package]);
    apply(&mut early, &commit);
    assert!(late.join(GROUP, &welcome).is_err());
    let tree = admin.ratchet_tree(GROUP).unwrap();
    let p = late.join_with_tree(GROUP, &welcome, &tree).unwrap();
    accept(&mut late, p);
    say(&mut early, e, &mut admin, "from the old-timer");
    say(&mut admin, 0, &mut late, "to the newcomer");
    say(&mut late, e + 1, &mut early, "from the newcomer");
}

#[test]
fn a_newcomer_joins_a_group_whose_tree_is_bigger_than_an_envelope() {
    // About 300 members: the tree no longer fits one envelope.
    let mut admin = group_of(300, true);
    let n = 301;
    let (mut newcomer, package) = invitee(n);
    let (_, welcome) = add(&mut admin, &[package]);
    let tree = admin.ratchet_tree(GROUP).unwrap();
    assert!(tree.len() > ENVELOPE, "tree of {} B", tree.len());
    assert!(welcome.len() < ENVELOPE, "Welcome of {} B", welcome.len());
    let p = newcomer.join_with_tree(GROUP, &welcome, &tree).unwrap();
    accept(&mut newcomer, p);
    say(&mut admin, 0, &mut newcomer, "there are many of us");
}

#[test]
fn each_newcomer_of_a_batch_gets_a_welcome_of_its_own_that_only_it_can_take() {
    let mut admin = group_of(MEMBERS, true);
    let (a, b) = (MEMBERS + 1, MEMBERS + 2);
    let (mut first, first_package) = invitee(a);
    let (mut second, second_package) = invitee(b);
    // A batch of fifty: one Welcome carries every newcomer's secrets.
    let mut batch = vec![first_package.clone(), second_package.clone()];
    batch.extend((b + 1..b + 49).map(|i| invitee(i).1));
    let (_, welcome) = add(&mut admin, &batch);
    let tree = admin.ratchet_tree(GROUP).unwrap();

    let for_first = agentic_crypto::welcome_for(&welcome, &first_package).unwrap();
    let for_second = agentic_crypto::welcome_for(&welcome, &second_package).unwrap();
    assert!(
        for_first.len() * 10 < welcome.len(),
        "{} B of {} B",
        for_first.len(),
        welcome.len()
    );
    // Each takes its own, not the other's.
    assert!(second.join_with_tree(GROUP, &for_first, &tree).is_err());
    let p = first.join_with_tree(GROUP, &for_first, &tree).unwrap();
    accept(&mut first, p);
    let p = second.join_with_tree(GROUP, &for_second, &tree).unwrap();
    accept(&mut second, p);
    // One message of the admin, read by both in its order.
    let p = admin.encrypt(GROUP, "to both".as_bytes(), AAD).unwrap();
    let wire = accept(&mut admin, p).wire;
    for newcomer in [&mut first, &mut second] {
        let p = newcomer.decrypt_contiguous(GROUP, &wire, AAD).unwrap();
        assert_eq!(accept(newcomer, p).plaintext, "to both".as_bytes());
    }
    // Someone the batch did not add has no Welcome in it.
    assert!(agentic_crypto::welcome_for(&welcome, &invitee(b + 100).1).is_err());
}
