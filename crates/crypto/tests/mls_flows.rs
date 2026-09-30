#![allow(clippy::unwrap_used, clippy::expect_used)]
use agentic_crypto::{CryptoError, MlsClient, Prepared};
use agentic_store::{IncomingCommit, MessageRecord, OutgoingCommit, ProfileStore, StateChange};
use minicbor::Encoder;
use openmls::prelude::{
    tls_codec::{Deserialize, Serialize},
    *,
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use openmls_traits::OpenMlsProvider;
use tempfile::TempDir;
const GROUP: [u8; 32] = [0x47; 32];
const AAD: &[u8] = b"agentic-internet/dev-genesis/conversation-1";
const TEXT: &[u8] = b"The encrypted message must survive a lost response and a restart.";
#[path = "support/contiguous_receive.rs"]
mod contiguous_receive;
#[path = "support/incremental_persistence.rs"]
mod incremental_persistence;
#[path = "support/scoped_records.rs"]
mod scoped_records;
fn accept<T>(client: &mut MlsClient, prepared: Prepared<T>) -> T {
    *client = MlsClient::restore(prepared.next_state.as_bytes()).unwrap();
    prepared.value
}
fn pair() -> (MlsClient, MlsClient) {
    let mut alice = MlsClient::new(b"alice-device").unwrap();
    let mut bob = MlsClient::new(b"bob-device").unwrap();
    let p = bob.key_package().unwrap();
    let kp = accept(&mut bob, p);
    let p = alice.create_group(GROUP).unwrap();
    accept(&mut alice, p);
    let p = alice.add_members(GROUP, &[kp]).unwrap();
    let added = accept(&mut alice, p);
    let p = alice.activate_pending_commit(GROUP).unwrap();
    accept(&mut alice, p);
    let p = bob.join(GROUP, &added.welcome).unwrap();
    accept(&mut bob, p);
    (alice, bob)
}
#[test]
fn two_peers_exchange_authenticated_private_messages_and_resume_after_restart() {
    let (mut alice, mut bob) = pair();
    let sent = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    let wire = accept(&mut alice, sent).wire;
    assert!(!wire.windows(TEXT.len()).any(|p| p == TEXT));
    assert!(matches!(
        MlsMessageIn::tls_deserialize_exact(&wire)
            .unwrap()
            .extract(),
        MlsMessageBodyIn::PrivateMessage(_)
    ));
    let received = bob.decrypt(GROUP, &wire, AAD).unwrap();
    assert_eq!(received.value.plaintext, TEXT);
    assert_eq!(received.value.sender, b"alice-device");
    accept(&mut bob, received);
    let mut bob = MlsClient::restore(bob.snapshot().as_bytes()).unwrap();
    let reply = bob.encrypt(GROUP, b"Got it", AAD).unwrap();
    let reply = accept(&mut bob, reply);
    let received = alice.decrypt(GROUP, &reply.wire, AAD).unwrap();
    assert_eq!(received.value.plaintext, b"Got it");
    assert_eq!(received.value.sender, b"bob-device");
}
#[test]
fn preparation_does_not_advance_the_callers_committed_snapshot() {
    let (alice, bob) = pair();
    let original = alice.snapshot().as_bytes().to_vec();
    let prepared = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    assert_eq!(alice.snapshot().as_bytes(), original);
    assert_ne!(prepared.next_state.as_bytes(), original);
    let retried = alice
        .encrypt(GROUP, b"Different unsaved attempt", AAD)
        .unwrap();
    assert_eq!(
        bob.decrypt(GROUP, &retried.value.wire, AAD)
            .unwrap()
            .value
            .plaintext,
        b"Different unsaved attempt"
    );
}
#[test]
fn tampering_wrong_context_wrong_group_and_trailing_data_do_not_consume_receive_state() {
    let (alice, bob) = pair();
    let wire = alice.encrypt(GROUP, TEXT, AAD).unwrap().value.wire;
    let before = bob.snapshot().as_bytes().to_vec();
    let mut altered = wire.clone();
    let last = altered.len() - 1;
    altered[last] ^= 1;
    let mut trailing = wire.clone();
    trailing.push(0);
    for invalid in [altered, trailing, wire[..wire.len() / 2].to_vec()] {
        assert!(bob.decrypt(GROUP, &invalid, AAD).is_err());
    }
    assert!(bob.decrypt([4; 32], &wire, AAD).is_err());
    assert!(
        bob.decrypt(GROUP, &wire, b"another genesis or command context")
            .is_err()
    );
    assert_eq!(bob.snapshot().as_bytes(), before);
    assert_eq!(
        bob.decrypt(GROUP, &wire, AAD).unwrap().value.plaintext,
        TEXT
    );
}
#[test]
fn replay_is_rejected_but_realistic_out_of_order_delivery_succeeds() {
    let (mut alice, mut bob) = pair();
    let first = alice.encrypt(GROUP, b"first", AAD).unwrap();
    let first = accept(&mut alice, first).wire;
    let second = alice.encrypt(GROUP, b"second", AAD).unwrap();
    let second = accept(&mut alice, second).wire;
    let received = bob.decrypt(GROUP, &second, AAD).unwrap();
    assert_eq!(received.value.plaintext, b"second");
    accept(&mut bob, received);
    let received = bob.decrypt(GROUP, &first, AAD).unwrap();
    assert_eq!(received.value.plaintext, b"first");
    accept(&mut bob, received);
    assert!(bob.decrypt(GROUP, &first, AAD).is_err());
    assert!(bob.decrypt(GROUP, &second, AAD).is_err());
}

#[test]
fn application_message_order_is_authenticated_and_read_only() {
    let (mut alice, bob) = pair();
    let first = alice.encrypt(GROUP, b"first", AAD).unwrap();
    let first = accept(&mut alice, first).wire;
    let second = alice.encrypt(GROUP, b"second", AAD).unwrap();
    let second = accept(&mut alice, second).wire;
    let before = bob.snapshot().as_bytes().to_vec();
    let first_metadata = bob.inspect_application_message(GROUP, &first, AAD).unwrap();
    let second_metadata = bob
        .inspect_application_message(GROUP, &second, AAD)
        .unwrap();
    assert_eq!(first_metadata.epoch, 1);
    assert_eq!(second_metadata.epoch, 1);
    assert_eq!(first_metadata.sender, b"alice-device");
    assert_eq!(second_metadata.sender, b"alice-device");
    assert_eq!(first_metadata.generation, 0);
    assert_eq!(second_metadata.generation, 1);
    let orders = bob
        .inspect_application_messages(GROUP, &[(&first, AAD), (&second, AAD)])
        .unwrap();
    assert_eq!(orders.len(), 2);
    assert_eq!(orders[0].input_index, 0);
    assert_eq!(orders[0].epoch, 1);
    assert_eq!(orders[0].sender, b"alice-device");
    assert_eq!(orders[0].generation, 0);
    assert_eq!(orders[1].input_index, 1);
    assert_eq!(orders[1].epoch, 1);
    assert_eq!(orders[1].sender, b"alice-device");
    assert_eq!(orders[1].generation, 1);
    assert_eq!(bob.snapshot().as_bytes(), before);
    let mut tampered = second.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    assert!(
        bob.inspect_application_message(GROUP, &tampered, AAD)
            .is_err()
    );
    assert!(
        bob.inspect_application_message(GROUP, &second, b"wrong aad")
            .is_err()
    );
    assert_eq!(bob.snapshot().as_bytes(), before);
}

#[test]
fn joining_member_has_no_past_access_and_removed_member_has_no_future_access() {
    let (mut alice, mut bob) = pair();
    let mut charlie = MlsClient::new(b"charlie-device").unwrap();
    let old = alice.encrypt(GROUP, b"Before Charlie joined", AAD).unwrap();
    let old = accept(&mut alice, old).wire;
    let kp = charlie.key_package().unwrap();
    let kp = accept(&mut charlie, kp);
    let added = alice.add_members(GROUP, &[kp]).unwrap();
    let added = accept(&mut alice, added);
    assert!(alice.encrypt(GROUP, TEXT, AAD).is_err());
    let activated = alice.activate_pending_commit(GROUP).unwrap();
    accept(&mut alice, activated);
    let received = bob.apply_finalized_commit(GROUP, &added.commit).unwrap();
    accept(&mut bob, received);
    let joined = charlie.join(GROUP, &added.welcome).unwrap();
    accept(&mut charlie, joined);
    assert!(charlie.decrypt(GROUP, &old, AAD).is_err());
    let shared = alice.encrypt(GROUP, b"For all three", AAD).unwrap();
    let shared = accept(&mut alice, shared).wire;
    assert_eq!(
        bob.decrypt(GROUP, &shared, AAD).unwrap().value.plaintext,
        b"For all three"
    );
    assert_eq!(
        charlie
            .decrypt(GROUP, &shared, AAD)
            .unwrap()
            .value
            .plaintext,
        b"For all three"
    );
    let removal = alice.remove_member(GROUP, b"bob-device").unwrap();
    let removal = accept(&mut alice, removal);
    assert!(alice.encrypt(GROUP, TEXT, AAD).is_err());
    let activated = alice.activate_pending_commit(GROUP).unwrap();
    accept(&mut alice, activated);
    let applied = charlie.apply_finalized_commit(GROUP, &removal).unwrap();
    accept(&mut charlie, applied);
    let new = alice
        .encrypt(GROUP, b"After Bob removal", AAD)
        .unwrap()
        .value
        .wire;
    assert!(bob.decrypt(GROUP, &new, AAD).is_err());
    assert_eq!(
        charlie.decrypt(GROUP, &new, AAD).unwrap().value.plaintext,
        b"After Bob removal"
    );
    let removed = bob.apply_finalized_commit(GROUP, &removal).unwrap();
    accept(&mut bob, removed);
    assert!(bob.encrypt(GROUP, b"Trying after removal", AAD).is_err());
}
#[test]
fn welcome_is_bound_to_recipient_and_expected_group_and_cannot_overwrite_a_joined_group() {
    let mut alice = MlsClient::new(b"alice").unwrap();
    let mut bob = MlsClient::new(b"bob").unwrap();
    let eve = MlsClient::new(b"eve").unwrap();
    let kp = bob.key_package().unwrap();
    let kp = accept(&mut bob, kp);
    let created = alice.create_group(GROUP).unwrap();
    accept(&mut alice, created);
    let added = alice.add_members(GROUP, &[kp]).unwrap();
    let added = accept(&mut alice, added);
    assert!(eve.join(GROUP, &added.welcome).is_err());
    assert!(bob.join([3; 32], &added.welcome).is_err());
    let joined = bob.join(GROUP, &added.welcome).unwrap();
    accept(&mut bob, joined);
    assert!(bob.join(GROUP, &added.welcome).is_err());
}
#[test]
fn malformed_key_packages_and_oversized_inputs_are_rejected_without_state_changes() {
    let (alice, bob) = pair();
    let before = alice.snapshot().as_bytes().to_vec();
    assert!(
        alice
            .add_members(GROUP, &[b"not a TLS key package".to_vec()])
            .is_err()
    );
    assert!(alice.encrypt(GROUP, &vec![0; 49_153], AAD).is_err());
    assert!(alice.encrypt(GROUP, TEXT, &vec![0; 1025]).is_err());
    assert!(bob.decrypt(GROUP, &vec![0; 65_537], AAD).is_err());
    assert_eq!(alice.snapshot().as_bytes(), before);
    assert!(MlsClient::restore(b"truncated snapshot").is_err());
}
#[test]
fn a_raw_openmls_peer_can_decrypt_adapter_traffic_and_reply_without_using_the_adapter() {
    let provider = OpenMlsRustCrypto::default();
    let suite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
    let signer = SignatureKeyPair::new(suite.signature_algorithm()).unwrap();
    signer.store(provider.storage()).unwrap();
    let credential = CredentialWithKey {
        credential: BasicCredential::new(b"independent-bob".to_vec()).into(),
        signature_key: signer.to_public_vec().into(),
    };
    let package = KeyPackage::builder()
        .build(suite, &provider, &signer, credential)
        .unwrap();
    let package = package.key_package().tls_serialize_detached().unwrap();
    let inspected = agentic_crypto::inspect_key_package(&package).unwrap();
    assert_eq!(inspected.identity, b"independent-bob");
    assert_eq!(inspected.signature_key, signer.to_public_vec());
    let mut alice = MlsClient::new(b"adapter-alice").unwrap();
    let created = alice.create_group(GROUP).unwrap();
    accept(&mut alice, created);
    let added = alice.add_members(GROUP, &[package]).unwrap();
    let added = accept(&mut alice, added);
    let activated = alice.activate_pending_commit(GROUP).unwrap();
    accept(&mut alice, activated);
    let welcome = match MlsMessageIn::tls_deserialize_exact(&added.welcome)
        .unwrap()
        .extract()
    {
        MlsMessageBodyIn::Welcome(w) => w,
        _ => panic!("expected Welcome"),
    };
    let config = MlsGroupJoinConfig::builder()
        .use_ratchet_tree_extension(true)
        .build();
    let members = alice.members(GROUP).unwrap();
    let peer = members
        .iter()
        .find(|m| m.identity == b"independent-bob")
        .unwrap();
    assert_eq!(peer.signature_key, signer.to_public_vec());
    let mut raw_bob = StagedWelcome::new_from_welcome(&provider, &config, welcome, None)
        .unwrap()
        .into_group(&provider)
        .unwrap();
    let domain = [0x11; 32];
    let mut context = Encoder::new(Vec::new());
    context
        .array(3)
        .unwrap()
        .bytes(&domain)
        .unwrap()
        .bytes(&GROUP)
        .unwrap()
        .bytes(b"independent-bob")
        .unwrap();
    let raw = raw_bob
        .export_secret(
            provider.crypto(),
            "AgenticInternet/mailbox-export/v1",
            &context.into_writer(),
            32,
        )
        .unwrap();
    let raw_seed = agentic_crypto::mailbox::MailboxSecret::from_bytes(raw.try_into().unwrap());
    let derived = alice
        .mailbox_secret(GROUP, domain, b"independent-bob")
        .unwrap();
    assert_eq!(
        derived.secret.swarm_mailbox(&domain, 20_000),
        raw_seed.swarm_mailbox(&domain, 20_000)
    );
    let sent = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    let sent = accept(&mut alice, sent);
    let protocol = MlsMessageIn::tls_deserialize_exact(&sent.wire)
        .unwrap()
        .try_into_protocol_message()
        .unwrap();
    let processed = raw_bob.process_message(&provider, protocol).unwrap();
    assert_eq!(processed.aad(), AAD);
    assert_eq!(
        processed.credential().serialized_content(),
        b"adapter-alice"
    );
    match processed.into_content() {
        ProcessedMessageContent::ApplicationMessage(m) => assert_eq!(m.into_bytes(), TEXT),
        _ => panic!("expected actual application payload"),
    }
    raw_bob.set_aad(AAD.to_vec());
    let reply = raw_bob
        .create_message(&provider, &signer, b"Independent reply")
        .unwrap()
        .to_bytes()
        .unwrap();
    let received = alice.decrypt(GROUP, &reply, AAD).unwrap();
    assert_eq!(received.value.plaintext, b"Independent reply");
    assert_eq!(received.value.sender, b"independent-bob");
}
#[test]
fn sqlcipher_failure_discards_prepared_ratchet_and_retry_persists_exactly_the_sent_ciphertext() {
    let (alice, bob) = pair();
    let root = TempDir::new().unwrap();
    let key = [0x31; 32];
    let mut store = ProfileStore::open(root.path().join("profile.db"), &key).unwrap();
    let record = |id: &str, text: &[u8]| MessageRecord {
        id: id.into(),
        conversation_id: "room".into(),
        author: "alice".into(),
        content: text.to_vec(),
        created_at: 1_788_480_000,
        own: true,
    };
    store
        .commit_incoming(IncomingCommit {
            message: record("setup", b"group created"),
            states: vec![StateChange {
                namespace: "crypto".into(),
                expected_revision: 0,
                bytes: alice.snapshot().as_bytes().to_vec(),
            }],
        })
        .unwrap();
    let conn = rusqlite::Connection::open(root.path().join("profile.db")).unwrap();
    conn.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_send BEFORE INSERT ON outbox BEGIN SELECT RAISE(ABORT,'disk full'); END;",hex::encode(key))).unwrap();
    let prepared = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    let commit = OutgoingCommit {
        operation_id: "op-1".into(),
        request_hash: [1; 32],
        message: record("sent", TEXT),
        destination: "bob".into(),
        wire: prepared.value.wire,
        states: vec![StateChange {
            namespace: "crypto".into(),
            expected_revision: 1,
            bytes: prepared.next_state.as_bytes().to_vec(),
        }],
    };
    assert!(store.commit_outgoing(commit).is_err());
    assert!(store.pending_outbox(100).unwrap().is_empty());
    let saved = store.state("crypto").unwrap().unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.bytes, alice.snapshot().as_bytes());
    conn.execute_batch("DROP TRIGGER fail_send").unwrap();
    let restored = MlsClient::restore(&saved.bytes).unwrap();
    let prepared = restored.encrypt(GROUP, TEXT, AAD).unwrap();
    let sent = prepared.value.wire.clone();
    store
        .commit_outgoing(OutgoingCommit {
            operation_id: "op-1".into(),
            request_hash: [1; 32],
            message: record("sent", TEXT),
            destination: "bob".into(),
            wire: prepared.value.wire,
            states: vec![StateChange {
                namespace: "crypto".into(),
                expected_revision: 1,
                bytes: prepared.next_state.as_bytes().to_vec(),
            }],
        })
        .unwrap();
    drop(store);
    let store = ProfileStore::open(root.path().join("profile.db"), &key).unwrap();
    assert_eq!(store.pending_outbox(100).unwrap()[0].wire, sent);
    assert_eq!(
        bob.decrypt(GROUP, &sent, AAD).unwrap().value.plaintext,
        TEXT
    );
    let restored = MlsClient::restore(&store.state("crypto").unwrap().unwrap().bytes).unwrap();
    let next = restored
        .encrypt(GROUP, b"Next message", AAD)
        .unwrap()
        .value
        .wire;
    let received = bob.decrypt(GROUP, &sent, AAD).unwrap();
    let bob = MlsClient::restore(received.next_state.as_bytes()).unwrap();
    assert_eq!(
        bob.decrypt(GROUP, &next, AAD).unwrap().value.plaintext,
        b"Next message"
    );
}

#[test]
fn key_package_signature_tampering_is_rejected_and_maximum_plaintext_is_supported() {
    let (alice, bob) = pair();
    let charlie = MlsClient::new(b"charlie").unwrap();
    let mut package = charlie.key_package().unwrap().value;
    let last = package.len() - 1;
    package[last] ^= 1;
    assert!(alice.add_members(GROUP, &[package]).is_err());
    let text = vec![0x51; 49_152];
    let sent = alice.encrypt(GROUP, &text, AAD).unwrap();
    assert!(sent.value.wire.len() <= 65_536);
    assert_eq!(
        bob.decrypt(GROUP, &sent.value.wire, AAD)
            .unwrap()
            .value
            .plaintext,
        text
    );
}

#[test]
fn discarded_group_changes_never_mutate_committed_state_or_activate_a_pending_epoch() {
    let (mut alice, mut bob) = pair();
    let mut charlie = MlsClient::new(b"charlie").unwrap();
    let kp = charlie.key_package().unwrap();
    let kp = accept(&mut charlie, kp);
    let before = alice.snapshot().as_bytes().to_vec();
    let added = alice.add_members(GROUP, &[kp]).unwrap();
    assert_eq!(alice.snapshot().as_bytes(), before);
    let before_commit = alice
        .encrypt(GROUP, b"Original epoch still works", AAD)
        .unwrap()
        .value
        .wire;
    assert_eq!(
        bob.decrypt(GROUP, &before_commit, AAD)
            .unwrap()
            .value
            .plaintext,
        b"Original epoch still works"
    );
    let removed = alice.remove_member(GROUP, b"bob-device").unwrap();
    assert_eq!(alice.snapshot().as_bytes(), before);
    drop(removed);
    assert_eq!(
        bob.decrypt(
            GROUP,
            &alice.encrypt(GROUP, TEXT, AAD).unwrap().value.wire,
            AAD
        )
        .unwrap()
        .value
        .plaintext,
        TEXT
    );
    let added = accept(&mut alice, added);
    let pending = alice.snapshot().as_bytes().to_vec();
    let activation = alice.activate_pending_commit(GROUP).unwrap();
    assert_eq!(alice.snapshot().as_bytes(), pending);
    assert!(alice.encrypt(GROUP, TEXT, AAD).is_err());
    accept(&mut alice, activation);
    let old_bob = bob.snapshot().as_bytes().to_vec();
    let applied = bob.apply_finalized_commit(GROUP, &added.commit).unwrap();
    assert_eq!(bob.snapshot().as_bytes(), old_bob);
    let after_commit = alice.encrypt(GROUP, b"New epoch", AAD).unwrap().value.wire;
    assert!(bob.decrypt(GROUP, &after_commit, AAD).is_err());
    assert_eq!(
        bob.decrypt(GROUP, &before_commit, AAD)
            .unwrap()
            .value
            .plaintext,
        b"Original epoch still works"
    );
    accept(&mut bob, applied);
    assert_eq!(
        bob.decrypt(GROUP, &after_commit, AAD)
            .unwrap()
            .value
            .plaintext,
        b"New epoch"
    );
}
#[test]
fn fresh_valid_welcome_cannot_replace_an_existing_group_with_the_same_id() {
    let (alice, mut bob) = pair();
    let mut impostor_creator = MlsClient::new(b"unrelated-creator").unwrap();
    let fresh = bob.key_package().unwrap();
    let fresh = accept(&mut bob, fresh);
    let created = impostor_creator.create_group(GROUP).unwrap();
    accept(&mut impostor_creator, created);
    let added = impostor_creator.add_members(GROUP, &[fresh]).unwrap();
    let added = accept(&mut impostor_creator, added);
    let before = bob.snapshot().as_bytes().to_vec();
    assert!(bob.join(GROUP, &added.welcome).is_err());
    assert_eq!(bob.snapshot().as_bytes(), before);
    let original = alice
        .encrypt(GROUP, b"Still the original Alice", AAD)
        .unwrap()
        .value
        .wire;
    assert_eq!(
        bob.decrypt(GROUP, &original, AAD).unwrap().value.plaintext,
        b"Still the original Alice"
    );
}
#[test]
fn offline_batch_can_arrive_128_messages_out_of_order_without_silent_loss() {
    let (mut alice, mut bob) = pair();
    let mut sent = vec![];
    for i in 0..128 {
        let p = alice
            .encrypt(GROUP, format!("message {i}").as_bytes(), AAD)
            .unwrap();
        sent.push(accept(&mut alice, p).wire);
    }
    for i in [127, 0].into_iter().chain(1..127) {
        let p = bob.decrypt(GROUP, &sent[i], AAD).unwrap();
        assert_eq!(p.value.plaintext, format!("message {i}").as_bytes());
        accept(&mut bob, p);
    }
    assert!(bob.decrypt(GROUP, &sent[0], AAD).is_err());
}
#[test]
fn three_past_epochs_survive_restart_but_the_fourth_is_retired() {
    let (mut alice, mut bob) = pair();
    let old = alice
        .encrypt(GROUP, b"Delayed before membership changes", AAD)
        .unwrap();
    let old = accept(&mut alice, old).wire;
    for epoch in 0..4 {
        let mut newcomer = MlsClient::new(format!("newcomer-{epoch}").as_bytes()).unwrap();
        let kp = newcomer.key_package().unwrap();
        let kp = accept(&mut newcomer, kp);
        let added = alice.add_members(GROUP, &[kp]).unwrap();
        let added = accept(&mut alice, added);
        let next = alice.activate_pending_commit(GROUP).unwrap();
        accept(&mut alice, next);
        let next = bob.apply_finalized_commit(GROUP, &added.commit).unwrap();
        accept(&mut bob, next);
        bob = MlsClient::restore(bob.snapshot().as_bytes()).unwrap();
        if epoch == 2 {
            // A temporary receive policy must not truncate the retained epoch
            // store before restoring the original group configuration.
            let current = alice.encrypt(GROUP, b"Current epoch", AAD).unwrap();
            let current = accept(&mut alice, current).wire;
            let next = bob.decrypt_contiguous(GROUP, &current, AAD).unwrap();
            assert_eq!(next.value.plaintext, b"Current epoch");
            accept(&mut bob, next);
            bob = MlsClient::restore(bob.snapshot().as_bytes()).unwrap();
            assert_eq!(
                bob.decrypt(GROUP, &old, AAD).unwrap().value.plaintext,
                b"Delayed before membership changes"
            );
        }
    }
    assert!(bob.decrypt(GROUP, &old, AAD).is_err());
}
#[test]
fn rejected_control_frames_leave_original_commit_applicable_and_accepted_commit_cannot_replay() {
    let (mut alice, mut bob) = pair();
    let charlie = MlsClient::new(b"charlie").unwrap();
    let added = alice
        .add_members(GROUP, &[charlie.key_package().unwrap().value])
        .unwrap();
    let added = accept(&mut alice, added);
    let mut bad = added.commit.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    let mut trailing = added.commit.clone();
    trailing.push(0);
    let before = bob.snapshot().as_bytes().to_vec();
    for bytes in [bad, trailing] {
        assert!(bob.apply_finalized_commit(GROUP, &bytes).is_err());
    }
    assert_eq!(bob.snapshot().as_bytes(), before);
    let next = bob.apply_finalized_commit(GROUP, &added.commit).unwrap();
    accept(&mut bob, next);
    assert!(bob.apply_finalized_commit(GROUP, &added.commit).is_err());
}

#[test]
fn inspected_key_package_returns_cryptographically_verified_binding_material() {
    let alice = MlsClient::new(b"root-bound-device").unwrap();
    let kp = alice.key_package().unwrap().value;
    let binding = agentic_crypto::inspect_key_package(&kp).unwrap();
    assert_eq!(binding.identity, b"root-bound-device");
    assert_eq!(binding.signature_key.len(), 32);
    let mut bad = kp.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    assert!(agentic_crypto::inspect_key_package(&bad).is_err());
}

#[test]
fn mailbox_export_is_shared_direction_bound_and_stable_across_application_traffic_and_restart() {
    let (mut alice, mut bob) = pair();
    let domain = [0x11; 32];
    let slot = 20_000;
    let before = alice.snapshot().as_bytes().to_vec();
    let a = alice.mailbox_secret(GROUP, domain, b"bob-device").unwrap();
    let b = bob.mailbox_secret(GROUP, domain, b"bob-device").unwrap();
    assert_eq!(a.epoch, 1);
    assert_eq!(
        a.secret.swarm_mailbox(&domain, slot),
        b.secret.swarm_mailbox(&domain, slot)
    );
    assert_eq!(alice.snapshot().as_bytes(), before);
    assert_ne!(
        a.secret.swarm_mailbox(&domain, slot),
        alice
            .mailbox_secret(GROUP, domain, b"alice-device")
            .unwrap()
            .secret
            .swarm_mailbox(&domain, slot)
    );
    assert_ne!(
        a.secret.swarm_mailbox(&domain, slot),
        alice
            .mailbox_secret(GROUP, [0x12; 32], b"bob-device")
            .unwrap()
            .secret
            .swarm_mailbox(&domain, slot)
    );
    assert!(alice.mailbox_secret(GROUP, domain, b"non-member").is_err());
    let encrypted = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    let wire = accept(&mut alice, encrypted).wire;
    let received = bob.decrypt(GROUP, &wire, AAD).unwrap();
    accept(&mut bob, received);
    let reopened = MlsClient::restore(bob.snapshot().as_bytes()).unwrap();
    assert_eq!(
        a.secret.swarm_mailbox(&domain, slot),
        reopened
            .mailbox_secret(GROUP, domain, b"bob-device")
            .unwrap()
            .secret
            .swarm_mailbox(&domain, slot)
    );
}
#[test]
fn pending_membership_and_finalized_removal_gate_mailbox_keys() {
    let (mut alice, mut bob) = pair();
    let domain = [0x11; 32];
    let slot = 20_000;
    let old = alice
        .mailbox_secret(GROUP, domain, b"alice-device")
        .unwrap()
        .secret
        .swarm_mailbox(&domain, slot);
    let removal = alice.remove_member(GROUP, b"bob-device").unwrap();
    let removal = accept(&mut alice, removal);
    assert!(
        alice
            .mailbox_secret(GROUP, domain, b"alice-device")
            .is_err(),
        "pending membership must not silently publish another mailbox epoch"
    );
    let activated = alice.activate_pending_commit(GROUP).unwrap();
    accept(&mut alice, activated);
    let current = alice
        .mailbox_secret(GROUP, domain, b"alice-device")
        .unwrap();
    assert_eq!(current.epoch, 2);
    assert_ne!(old, current.secret.swarm_mailbox(&domain, slot));
    assert!(alice.mailbox_secret(GROUP, domain, b"bob-device").is_err());
    let applied = bob.apply_finalized_commit(GROUP, &removal).unwrap();
    accept(&mut bob, applied);
    assert!(bob.mailbox_secret(GROUP, domain, b"alice-device").is_err());
}
/// A KeyPackage of a client made before group data: its leaf does not list
/// the extension that carries it.
fn package_without_group_data(identity: &[u8]) -> Vec<u8> {
    let provider = OpenMlsRustCrypto::default();
    let suite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
    let signer = SignatureKeyPair::new(suite.signature_algorithm()).unwrap();
    signer.store(provider.storage()).unwrap();
    let credential = CredentialWithKey {
        credential: BasicCredential::new(identity.to_vec()).into(),
        signature_key: signer.to_public_vec().into(),
    };
    KeyPackage::builder()
        .build(suite, &provider, &signer, credential)
        .unwrap()
        .key_package()
        .tls_serialize_detached()
        .unwrap()
}
/// Data every member agrees on rides in the group context: a commit sets
/// it, readers and newcomers see it, and a commit without it keeps it. A
/// client whose leaf cannot carry it is refused, invited or already in.
#[test]
fn group_data_is_agreed_by_commits_and_refuses_clients_that_cannot_carry_it() {
    let (mut alice, mut bob) = pair();
    assert_eq!(alice.group_data(GROUP).unwrap(), None);
    let set = alice
        .commit_changes(GROUP, &[], &[], b"roster", Some(b"bans v1"))
        .unwrap();
    let set = accept(&mut alice, set);
    let view = bob.view_commit(GROUP, &set.commit).unwrap();
    assert_eq!(view.data.as_deref(), Some(&b"bans v1"[..]));
    let next = alice.activate_pending_commit(GROUP).unwrap();
    accept(&mut alice, next);
    let applied = bob.apply_commit(GROUP, &set.commit).unwrap();
    accept(&mut bob, applied);
    for client in [&alice, &bob] {
        assert_eq!(
            client.group_data(GROUP).unwrap().as_deref(),
            Some(&b"bans v1"[..])
        );
    }
    // A newcomer, added by a commit that leaves the data as it is.
    let mut carol = MlsClient::new(b"carol-device").unwrap();
    let kp = carol.key_package().unwrap();
    let kp = accept(&mut carol, kp);
    let added = alice
        .commit_changes(GROUP, &[kp], &[], b"roster", None)
        .unwrap();
    let added = accept(&mut alice, added);
    assert_eq!(
        bob.view_commit(GROUP, &added.commit)
            .unwrap()
            .data
            .as_deref(),
        Some(&b"bans v1"[..])
    );
    let next = alice.activate_pending_commit(GROUP).unwrap();
    accept(&mut alice, next);
    let joined = carol.join(GROUP, added.welcome.as_ref().unwrap()).unwrap();
    accept(&mut carol, joined);
    assert_eq!(
        carol.group_data(GROUP).unwrap().as_deref(),
        Some(&b"bans v1"[..])
    );
    // An older client is not invited into a group with data...
    let old = package_without_group_data(b"old-dave");
    assert!(matches!(
        alice.commit_changes(GROUP, std::slice::from_ref(&old), &[], b"roster", None),
        Err(CryptoError::Outdated)
    ));
    // ...and a group with an older member takes none.
    let mut erin = MlsClient::new(b"erin-device").unwrap();
    let created = erin.create_group(GROUP).unwrap();
    accept(&mut erin, created);
    let added = erin.add_members(GROUP, &[old]).unwrap();
    accept(&mut erin, added);
    let next = erin.activate_pending_commit(GROUP).unwrap();
    accept(&mut erin, next);
    assert!(matches!(
        erin.commit_changes(GROUP, &[], &[], b"roster", Some(b"bans v1")),
        Err(CryptoError::Outdated)
    ));
}
