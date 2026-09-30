//! An operation on one group reads and writes the profile's own records and
//! that group's alone (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 1,
//! phase 1b): a profile in several big groups no longer copies all of them
//! on every message, and a client acts only on what it loaded.
use super::*;
use agentic_crypto::{SecretState, record_scope};
use openmls_traits::storage::CURRENT_VERSION;
use std::collections::BTreeMap;

const OTHER: [u8; 32] = [0x48; 32];
const THIRD: [u8; 32] = [0x49; 32];

/// A profile's records as its database holds them.
type Rows = BTreeMap<Vec<u8>, Vec<u8>>;

fn rows_of(client: &MlsClient) -> Rows {
    client
        .snapshot()
        .record_changes(None)
        .map(|(key, bytes)| (key.to_vec(), bytes.unwrap().to_vec()))
        .collect()
}

fn in_scope(key: &[u8], scope: &[Vec<u8>]) -> bool {
    scope.iter().any(|prefix| key.starts_with(prefix))
}

/// The prefixes of `group`'s own records, the profile's left out.
fn group_only(group: [u8; 32]) -> Vec<Vec<u8>> {
    let common = record_scope(None);
    record_scope(Some(group))
        .into_iter()
        .filter(|prefix| !common.contains(prefix))
        .collect()
}

/// The rows of `scope`, as a scoped read returns them.
fn scoped(rows: &Rows, scope: &[Vec<u8>]) -> Rows {
    rows.iter()
        .filter(|(key, _)| in_scope(key, scope))
        .map(|(key, bytes)| (key.clone(), bytes.clone()))
        .collect()
}

/// Each key is the profile's own or exactly one of `groups`'.
fn assert_classified<'a>(keys: impl Iterator<Item = &'a Vec<u8>>, groups: &[[u8; 32]]) {
    let common = record_scope(None);
    for key in keys {
        let places = std::iter::once(in_scope(key, &common))
            .chain(
                groups
                    .iter()
                    .map(|group| in_scope(key, &group_only(*group))),
            )
            .filter(|found| *found)
            .count();
        assert_eq!(places, 1, "{}", String::from_utf8_lossy(key));
    }
}

/// Apply what `after` changed since the loaded `before` to the rows; the new
/// prefix comes along.
fn apply(
    rows: &mut Rows,
    prefix: &mut Vec<u8>,
    before: &MlsClient,
    after: &SecretState,
    scope: &[Vec<u8>],
) {
    for (key, bytes) in after.record_changes(before.persisted_record_state()) {
        assert!(
            in_scope(key, scope),
            "a change outside its scope: {}",
            String::from_utf8_lossy(key)
        );
        match bytes {
            Some(bytes) => rows.insert(key.to_vec(), bytes.to_vec()),
            None => rows.remove(key),
        };
    }
    *prefix = after.record_prefix().to_vec();
}

/// Load `group`'s scope from the rows, as the core does before an operation.
fn load(rows: &Rows, prefix: &[u8], group: Option<[u8; 32]>) -> (MlsClient, Vec<Vec<u8>>) {
    let scope = record_scope(group);
    let client = MlsClient::restore_scope(prefix, scoped(rows, &scope), group).unwrap();
    (client, scope)
}

/// The whole profile, restored from all its rows: its count holds.
fn whole(rows: &Rows, prefix: &[u8]) -> MlsClient {
    MlsClient::restore_records(prefix, rows.clone()).unwrap()
}

/// Alice in two groups: GROUP with Bob, OTHER with Carol; each a Welcome
/// joined. Alice's rows and prefix as persisted.
fn two_groups() -> (Rows, Vec<u8>, MlsClient, MlsClient) {
    let mut alice = MlsClient::new(b"alice-scoped").unwrap();
    let mut bob = MlsClient::new(b"bob-scoped").unwrap();
    let mut carol = MlsClient::new(b"carol-scoped").unwrap();
    let p = bob.key_package().unwrap();
    let bob_package = accept(&mut bob, p);
    let p = carol.key_package().unwrap();
    let carol_package = accept(&mut carol, p);
    for (group, package, peer) in [
        (GROUP, bob_package, &mut bob),
        (OTHER, carol_package, &mut carol),
    ] {
        let p = alice.create_group(group).unwrap();
        accept(&mut alice, p);
        let p = alice.add_members(group, &[package]).unwrap();
        let added = accept(&mut alice, p);
        let p = alice.activate_pending_commit(group).unwrap();
        accept(&mut alice, p);
        let p = peer.join(group, &added.welcome).unwrap();
        accept(peer, p);
    }
    let rows = rows_of(&alice);
    let prefix = alice.snapshot().record_prefix().to_vec();
    (rows, prefix, bob, carol)
}

/// The prefix of a record-backed state with `more` records counted.
fn counting(prefix: &[u8], more: u64) -> Vec<u8> {
    let mut decoder = minicbor::Decoder::new(prefix);
    assert_eq!(decoder.array().unwrap(), Some(4));
    assert_eq!(decoder.u8().unwrap(), 1);
    let identity = decoder.bytes().unwrap().to_vec();
    let signer = decoder.bytes().unwrap().to_vec();
    let count = decoder.array().unwrap().unwrap();
    let mut out = Vec::new();
    Encoder::new(&mut out)
        .array(4)
        .unwrap()
        .u8(1)
        .unwrap()
        .bytes(&identity)
        .unwrap()
        .bytes(&signer)
        .unwrap()
        .array(count + more)
        .unwrap();
    out
}

/// A record as the database holds it: `[key, value]`.
fn record(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    Encoder::new(&mut out)
        .array(2)
        .unwrap()
        .bytes(key)
        .unwrap()
        .bytes(value)
        .unwrap();
    out
}

#[test]
fn every_openmls_label_falls_in_exactly_one_scope() {
    // Keys as openmls_memory_storage 0.6 builds them: label, JSON key,
    // version.
    let version = CURRENT_VERSION.to_be_bytes();
    let key = |label: &[u8], json: Vec<u8>| [label, &json, &version].concat();
    let gid = |id: [u8; 32]| serde_json::to_vec(&GroupId::from_slice(&id)).unwrap();
    let by_group: [&[u8]; 15] = [
        b"Tree",
        b"GroupContext",
        b"InterimTranscriptHash",
        b"ConfirmationTag",
        b"MlsGroupJoinConfig",
        b"OwnLeafNodes",
        b"GroupState",
        b"ProposalQueueRefs",
        b"OwnLeafNodeIndex",
        b"EpochSecrets",
        b"ResumptionPsk",
        b"MessageSecrets",
        b"ApplicationExportTree",
        b"VcEmulationBinding",
        b"RegisteredVcEmulationEpoch",
    ];
    let own: [&[u8]; 8] = [
        b"KeyPackage",
        b"Psk",
        b"EncryptionKeyPair",
        b"SignatureKeyPair",
        b"RetainedKeyPackageMaterial",
        b"RetainedKeyPackageEpoch",
        b"VcEmulationEpochState",
        b"VcOperationTree",
    ];
    let mut keys = vec![];
    for group in [GROUP, OTHER] {
        for label in by_group {
            keys.push((key(label, gid(group)), Some(group)));
        }
        let epoch_pairs = [
            gid(group),
            serde_json::to_vec(&7u64).unwrap(),
            serde_json::to_vec(&3u32).unwrap(),
        ]
        .concat();
        keys.push((key(b"EpochKeyPairs", epoch_pairs), Some(group)));
        let proposal = serde_json::to_vec(&(GroupId::from_slice(&group), vec![1u8, 2, 3])).unwrap();
        keys.push((key(b"QueuedProposal", proposal), Some(group)));
    }
    for label in own {
        keys.push((
            key(label, serde_json::to_vec(&vec![0x47u8; 32]).unwrap()),
            None,
        ));
    }
    let common = record_scope(None);
    for (key, owner) in &keys {
        assert_eq!(
            in_scope(key, &common),
            owner.is_none(),
            "{}",
            String::from_utf8_lossy(key)
        );
        for group in [GROUP, OTHER] {
            assert_eq!(
                in_scope(key, &group_only(group)),
                *owner == Some(group),
                "{}",
                String::from_utf8_lossy(key)
            );
        }
    }
    assert_classified(keys.iter().map(|(key, _)| key), &[GROUP, OTHER]);
    // A scope is the profile's records and its group's.
    let group_scope = record_scope(Some(GROUP));
    assert!(common.iter().all(|prefix| group_scope.contains(prefix)));
}

#[test]
fn every_record_a_workflow_makes_is_the_profiles_own_or_one_groups() {
    let (rows, _, bob, _) = two_groups();
    assert_classified(rows.keys().chain(rows_of(&bob).keys()), &[GROUP, OTHER]);
    for group in [GROUP, OTHER] {
        assert!(
            rows.keys()
                .any(|key| in_scope(key, &group_only(group)) && key.starts_with(b"Tree"))
        );
    }
}

#[test]
fn a_client_acts_only_on_the_group_it_loaded() {
    let (rows, prefix, _, mut carol) = two_groups();
    let p = carol.key_package().unwrap();
    let package = accept(&mut carol, p);
    let (alice, _) = load(&rows, &prefix, Some(GROUP));
    assert!(alice.epoch(GROUP).is_ok());
    assert!(matches!(
        alice.knows_group(OTHER),
        Err(CryptoError::OutOfScope)
    ));
    assert!(matches!(alice.epoch(OTHER), Err(CryptoError::OutOfScope)));
    assert!(matches!(
        alice.encrypt(OTHER, TEXT, AAD),
        Err(CryptoError::OutOfScope)
    ));
    assert!(matches!(
        alice.create_group(THIRD),
        Err(CryptoError::OutOfScope)
    ));
    assert!(matches!(
        alice.add_members(OTHER, std::slice::from_ref(&package)),
        Err(CryptoError::OutOfScope)
    ));
    assert!(matches!(
        alice.join_with_tree(OTHER, b"welcome", b"tree"),
        Err(CryptoError::OutOfScope)
    ));
    // The profile's own records alone: key packages, and no group at all.
    let (own, _) = load(&rows, &prefix, None);
    assert!(own.key_package().is_ok());
    assert!(matches!(own.epoch(GROUP), Err(CryptoError::OutOfScope)));
    assert!(matches!(
        own.knows_group(GROUP),
        Err(CryptoError::OutOfScope)
    ));
    assert!(matches!(
        own.create_group(THIRD),
        Err(CryptoError::OutOfScope)
    ));
    // A group's scope still makes key packages: the profile's records come
    // with it.
    assert!(alice.key_package().is_ok());
    // The whole profile acts on any group.
    let everything = whole(&rows, &prefix);
    assert!(everything.epoch(OTHER).is_ok() && everything.epoch(GROUP).is_ok());
}

#[test]
fn an_operation_on_one_group_changes_its_records_alone_and_keeps_the_count() {
    let (mut rows, mut prefix, bob, carol) = two_groups();
    let other_rows = scoped(&rows, &record_scope(Some(OTHER)));
    let other_only = scoped(&rows, &group_only(OTHER));

    // Alice writes in GROUP having loaded GROUP's records only.
    let (alice, scope) = load(&rows, &prefix, Some(GROUP));
    let sent = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    apply(&mut rows, &mut prefix, &alice, &sent.next_state, &scope);
    assert_eq!(scoped(&rows, &group_only(OTHER)), other_only);
    assert_eq!(
        bob.decrypt(GROUP, &sent.value.wire, AAD)
            .unwrap()
            .value
            .plaintext,
        TEXT
    );

    // Carol's reply read in OTHER, loaded alone.
    let reply = carol.encrypt(OTHER, b"from the other group", AAD).unwrap();
    let (alice, scope) = load(&rows, &prefix, Some(OTHER));
    let received = alice.decrypt(OTHER, &reply.value.wire, AAD).unwrap();
    assert_eq!(received.value.plaintext, b"from the other group");
    apply(&mut rows, &mut prefix, &alice, &received.next_state, &scope);
    assert_ne!(scoped(&rows, &record_scope(Some(OTHER))), other_rows);

    // The whole profile restores from its rows: the count held.
    let everything = whole(&rows, &prefix);
    let again = everything.encrypt(OTHER, b"whole", AAD).unwrap();
    assert_eq!(
        carol
            .decrypt(OTHER, &again.value.wire, AAD)
            .unwrap()
            .value
            .plaintext,
        b"whole"
    );
    // A partial state is never a whole snapshot.
    let (alice, _) = load(&rows, &prefix, Some(GROUP));
    assert!(MlsClient::restore(alice.snapshot().as_bytes()).is_err());
}

#[test]
fn chains_of_operations_in_one_scope_keep_the_count() {
    let (mut rows, mut prefix, mut bob, _) = two_groups();
    let mut dave = MlsClient::new(b"dave-scoped").unwrap();
    // Create, add and activate, one after another in THIRD's scope.
    let p = dave.key_package().unwrap();
    let package = accept(&mut dave, p);
    let (alice, scope) = load(&rows, &prefix, Some(THIRD));
    let created = alice.create_group(THIRD).unwrap();
    let added = MlsClient::from_state(created.next_state)
        .add_members(THIRD, &[package])
        .unwrap();
    let activated = MlsClient::from_state(added.next_state)
        .activate_pending_commit(THIRD)
        .unwrap();
    apply(
        &mut rows,
        &mut prefix,
        &alice,
        &activated.next_state,
        &scope,
    );
    let p = dave.join(THIRD, &added.value.welcome).unwrap();
    accept(&mut dave, p);
    assert_eq!(whole(&rows, &prefix).members(THIRD).unwrap().len(), 2);

    // A commit made and let go, then one made and activated.
    let (alice, scope) = load(&rows, &prefix, Some(GROUP));
    let made = alice.commit_changes(GROUP, &[], &[], b"", None).unwrap();
    let cleared = MlsClient::from_state(made.next_state)
        .clear_pending_commit(GROUP)
        .unwrap();
    apply(&mut rows, &mut prefix, &alice, &cleared.next_state, &scope);
    let (alice, scope) = load(&rows, &prefix, Some(GROUP));
    let made = alice.commit_changes(GROUP, &[], &[], b"", None).unwrap();
    let commit = made.value.commit.clone();
    let activated = MlsClient::from_state(made.next_state)
        .activate_pending_commit(GROUP)
        .unwrap();
    apply(
        &mut rows,
        &mut prefix,
        &alice,
        &activated.next_state,
        &scope,
    );
    let p = bob.apply_commit(GROUP, &commit).unwrap();
    accept(&mut bob, p);
    assert_eq!(
        whole(&rows, &prefix).epoch(GROUP).unwrap(),
        bob.epoch(GROUP).unwrap()
    );

    // Bob's commit, received: the epoch moves, its keys change.
    let mut erin = MlsClient::new(b"erin-scoped").unwrap();
    let p = erin.key_package().unwrap();
    let package = accept(&mut erin, p);
    let p = bob
        .commit_changes(GROUP, &[package], &[], b"", None)
        .unwrap();
    let made = accept(&mut bob, p);
    let p = bob.activate_pending_commit(GROUP).unwrap();
    accept(&mut bob, p);
    let (alice, scope) = load(&rows, &prefix, Some(GROUP));
    let applied = alice.apply_commit(GROUP, &made.commit).unwrap();
    apply(&mut rows, &mut prefix, &alice, &applied.next_state, &scope);
    let everything = whole(&rows, &prefix);
    assert_eq!(everything.epoch(GROUP).unwrap(), bob.epoch(GROUP).unwrap());
    assert_eq!(everything.members(GROUP).unwrap().len(), 3);
    assert_classified(rows.keys(), &[GROUP, OTHER, THIRD]);
}

#[test]
fn joining_leaving_and_key_packages_keep_the_count_in_their_scopes() {
    let (mut rows, mut prefix, mut bob, _) = two_groups();
    // A key package is the profile's own: made without any group loaded.
    let (alice, scope) = load(&rows, &prefix, None);
    let made = alice.key_package().unwrap();
    apply(&mut rows, &mut prefix, &alice, &made.next_state, &scope);
    let (alice, scope) = load(&rows, &prefix, None);
    let last = alice.last_resort_key_package().unwrap();
    apply(&mut rows, &mut prefix, &alice, &last.next_state, &scope);

    // Bob makes THIRD, its tree apart, and adds Alice; she joins with
    // THIRD's scope loaded (empty) and the tree fetched apart.
    let p = bob.create_group(THIRD).unwrap();
    accept(&mut bob, p);
    let p = bob.set_tree_apart(THIRD).unwrap();
    accept(&mut bob, p);
    let p = bob.add_members(THIRD, &[made.value]).unwrap();
    let added = accept(&mut bob, p);
    let p = bob.activate_pending_commit(THIRD).unwrap();
    accept(&mut bob, p);
    let tree = bob.ratchet_tree(THIRD).unwrap();
    let (alice, scope) = load(&rows, &prefix, Some(THIRD));
    let joined = alice.join_with_tree(THIRD, &added.welcome, &tree).unwrap();
    apply(&mut rows, &mut prefix, &alice, &joined.next_state, &scope);
    assert_eq!(whole(&rows, &prefix).members(THIRD).unwrap().len(), 2);

    // A group joined already cannot be joined over, loaded alone.
    let (alice, _) = load(&rows, &prefix, Some(THIRD));
    assert!(matches!(
        alice.join_with_tree(THIRD, &added.welcome, &tree),
        Err(CryptoError::AlreadyJoined)
    ));

    // Bob removes her; she applies it, forgets THIRD and joins again by the
    // last-resort package in one chain, as the core's rejoin does.
    let p = bob
        .commit_changes(THIRD, &[], &[b"alice-scoped".to_vec()], b"", None)
        .unwrap();
    let removal = accept(&mut bob, p);
    let p = bob.activate_pending_commit(THIRD).unwrap();
    accept(&mut bob, p);
    let (alice, scope) = load(&rows, &prefix, Some(THIRD));
    let applied = alice.apply_commit(THIRD, &removal.commit).unwrap();
    assert!(applied.value.self_removed);
    apply(&mut rows, &mut prefix, &alice, &applied.next_state, &scope);
    let p = bob
        .add_members(THIRD, std::slice::from_ref(&last.value))
        .unwrap();
    let back = accept(&mut bob, p);
    let p = bob.activate_pending_commit(THIRD).unwrap();
    accept(&mut bob, p);
    let tree = bob.ratchet_tree(THIRD).unwrap();
    let (alice, scope) = load(&rows, &prefix, Some(THIRD));
    let forgotten = alice.forget_group(THIRD).unwrap();
    let rejoined = MlsClient::from_state(forgotten.next_state)
        .join_with_tree(THIRD, &back.welcome, &tree)
        .unwrap();
    apply(&mut rows, &mut prefix, &alice, &rejoined.next_state, &scope);
    let everything = whole(&rows, &prefix);
    assert_eq!(everything.members(THIRD).unwrap().len(), 2);
    assert!(everything.knows_group(GROUP).unwrap());

    // Forgotten for good: THIRD's own records go, the count follows.
    let (alice, scope) = load(&rows, &prefix, Some(THIRD));
    let forgotten = alice.forget_group(THIRD).unwrap();
    apply(
        &mut rows,
        &mut prefix,
        &alice,
        &forgotten.next_state,
        &scope,
    );
    let left: Vec<String> = scoped(&rows, &group_only(THIRD))
        .keys()
        .map(|k| String::from_utf8_lossy(k).into_owned())
        .collect();
    assert!(left.is_empty(), "{left:?}");
    assert!(!whole(&rows, &prefix).knows_group(THIRD).unwrap());
    assert_classified(rows.keys(), &[GROUP, OTHER, THIRD]);
}

#[test]
fn one_operation_loads_its_scope_under_the_cap_the_profile_may_hold_more() {
    let (mut rows, prefix, bob, _) = two_groups();
    // OTHER holds more than one operation may load.
    let mut padding = group_only(OTHER)
        .into_iter()
        .find(|prefix| prefix.starts_with(b"Tree"))
        .unwrap();
    padding.extend_from_slice(b"-padding");
    rows.insert(
        padding.clone(),
        record(&padding, &vec![0x5a; 33 * 1024 * 1024]),
    );
    let mut prefix = counting(&prefix, 1);
    // GROUP's operations go on.
    let (alice, scope) = load(&rows, &prefix, Some(GROUP));
    let sent = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    apply(&mut rows, &mut prefix, &alice, &sent.next_state, &scope);
    assert_eq!(
        bob.decrypt(GROUP, &sent.value.wire, AAD)
            .unwrap()
            .value
            .plaintext,
        TEXT
    );
    // OTHER's, and the whole profile, do not fit.
    let other = record_scope(Some(OTHER));
    assert!(matches!(
        MlsClient::restore_scope(&prefix, scoped(&rows, &other), Some(OTHER)),
        Err(CryptoError::TooLarge)
    ));
    assert!(matches!(
        MlsClient::restore_records(&prefix, rows),
        Err(CryptoError::TooLarge)
    ));
}

#[test]
fn a_scoped_restore_is_checked() {
    let (rows, prefix, _, _) = two_groups();
    let scope = record_scope(Some(GROUP));
    // More records than the state counts.
    let mut extra = scoped(&rows, &scope);
    let mut forged = group_only(GROUP)[0].clone();
    forged.extend_from_slice(b"-forged");
    extra.insert(forged.clone(), record(&forged, b"x"));
    for _ in 0..rows.len() {
        let mut more = forged.clone();
        more.extend_from_slice(&extra.len().to_be_bytes());
        extra.insert(more.clone(), record(&more, b"x"));
    }
    assert!(MlsClient::restore_scope(&prefix, extra, Some(GROUP)).is_err());
    // Records outside the scope it names.
    assert!(matches!(
        MlsClient::restore_scope(
            &prefix,
            scoped(&rows, &record_scope(Some(OTHER))),
            Some(GROUP)
        ),
        Err(CryptoError::OutOfScope)
    ));
    // A group's records without the profile's own: no signer.
    assert!(
        MlsClient::restore_scope(&prefix, scoped(&rows, &group_only(GROUP)), Some(GROUP)).is_err()
    );
    // A record-backed prefix with no records is not a legacy snapshot.
    assert!(MlsClient::restore_scope(&prefix, Rows::new(), Some(GROUP)).is_err());
    // A legacy snapshot, with no records, still restores whole.
    let legacy = MlsClient::new(b"legacy").unwrap();
    let restored =
        MlsClient::restore_scope(legacy.snapshot().as_bytes(), Rows::new(), None).unwrap();
    assert!(restored.key_package().is_ok());
}
