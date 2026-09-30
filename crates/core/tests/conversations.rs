#![allow(clippy::unwrap_used, clippy::expect_used)]
use agentic_capabilities::Action;
use agentic_core::AppCore;
use agentic_core::{AgentCall, RuntimeGrant, RuntimeGrantRequest};
use agentic_protocol::{DocumentDraft, DocumentKind, SignedDocument, VerifiedDocument};
use agentic_store::ProfileStore;
use ed25519_dalek::SigningKey;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use tempfile::TempDir;
const DOMAIN: [u8; 32] = [7; 32];
const NOW: u64 = 1_788_570_000;
const KEY: [u8; 32] = [0x11; 32];
#[path = "support/agent_delivery.rs"]
mod agent_delivery;
#[path = "support/agent_inbox.rs"]
mod agent_inbox;
#[path = "support/contact_by_id.rs"]
mod contact_by_id;
#[path = "support/conversation_scan.rs"]
mod conversation_scan;
#[path = "support/desktop_history.rs"]
mod desktop_history;
#[path = "support/groups.rs"]
mod groups;
#[path = "support/mailbox_swarm.rs"]
mod mailbox_swarm;
#[path = "support/mls_persistence.rs"]
mod mls_persistence;
#[path = "support/network_preferences.rs"]
mod network_preferences;
#[path = "support/owner_inbox.rs"]
mod owner_inbox;
#[path = "support/peer_records.rs"]
mod peer_records;
#[path = "support/profile_db.rs"]
mod profile_db;
#[path = "support/receive_admission.rs"]
mod receive_admission;
#[path = "support/runtime_context.rs"]
mod runtime_context;
#[path = "support/runtime_provisioning.rs"]
mod runtime_provisioning;

fn runtime_key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}
fn grant_runtime(
    core: &mut AppCore,
    key: &SigningKey,
    conversation: &str,
    actions: &[Action],
) -> RuntimeGrant {
    core.grant_runtime(runtime_request(key, conversation, actions), NOW)
        .unwrap()
}
fn runtime_request(
    key: &SigningKey,
    conversation: &str,
    actions: &[Action],
) -> RuntimeGrantRequest {
    RuntimeGrantRequest {
        name: "Scoped assistant".into(),
        principal: key.verifying_key().to_bytes(),
        agent_id: [61; 32],
        service_id: [62; 32],
        conversation_ids: vec![conversation.into()],
        actions: actions.iter().copied().collect::<BTreeSet<_>>(),
        expires_at: NOW + 3600,
        max_data_bytes: 4096,
    }
}
fn agent_proof(
    key: &SigningKey,
    grant: &RuntimeGrant,
    method: &str,
    request: serde_json::Value,
    nonce: u8,
) -> Vec<u8> {
    SignedDocument::sign(
        AgentCall {
            grant_id: grant.grant_id,
            method: method.into(),
            request,
            nonce: [nonce; 32],
        }
        .draft(DOMAIN, 0, NOW)
        .unwrap(),
        key,
    )
    .unwrap()
    .to_wire()
}

#[test]
fn scoped_runtime_reads_only_granted_dialog_and_sends_through_actual_mls_reducer() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut carol = profile(&cr, "Carol");
    let allowed = connect(&mut alice, &mut bob);
    let private = connect(&mut alice, &mut carol);
    alice
        .send_message(&private, "Owner private conversation", "private-owner", NOW)
        .unwrap();
    let key = runtime_key(41);
    let grant = grant_runtime(
        &mut alice,
        &key,
        &allowed,
        &[Action::ReadInbox, Action::SendMessage],
    );
    let snapshot = alice
        .agent_call(&agent_proof(&key, &grant, "snapshot", json!({}), 1), NOW)
        .unwrap();
    assert_eq!(snapshot["conversations"].as_array().unwrap().len(), 1);
    assert_eq!(snapshot["conversations"][0]["id"], allowed);
    assert!(!snapshot.to_string().contains("Owner private conversation"));
    assert!(snapshot.get("identity").is_none());
    assert_eq!(snapshot["agentId"], hex::encode([61; 32]));
    assert_eq!(snapshot["serviceId"], hex::encode([62; 32]));
    let sent=alice.agent_call(&agent_proof(&key,&grant,"send_message",json!({"conversationId":allowed,"text":"Authorized agent message","operationId":"agent-send"}),2),NOW).unwrap();
    let work = alice
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|item| item.message_id == sent["id"].as_str().unwrap())
        .unwrap();
    let ack = bob.receive(&work.wire, NOW).unwrap().reply.unwrap();
    alice.receive(&ack, NOW).unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Authorized agent message"
    );
    assert_eq!(
        alice
            .snapshot()
            .unwrap()
            .conversations
            .iter()
            .find(|c| c.id == allowed)
            .unwrap()
            .messages[0]
            .delivery
            .phase,
        "delivered"
    );
    assert!(
        alice
            .agent_call(
                &agent_proof(
                    &key,
                    &grant,
                    "send_message",
                    json!({"conversationId":private,"text":"Forbidden","operationId":"escape"}),
                    3
                ),
                NOW
            )
            .is_err()
    );
    assert_eq!(
        alice
            .snapshot()
            .unwrap()
            .conversations
            .iter()
            .find(|c| c.id == private)
            .unwrap()
            .messages
            .len(),
        1
    );
}

#[test]
fn agent_proofs_cannot_forge_principal_replay_escalate_or_survive_revocation_and_restart() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let key = runtime_key(42);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::ReadInbox]);
    let valid = agent_proof(&key, &grant, "snapshot", json!({}), 1);
    assert!(
        alice
            .agent_call(
                &agent_proof(&runtime_key(43), &grant, "snapshot", json!({}), 2),
                NOW
            )
            .is_err()
    );
    for (i, method) in [
        "create_identity",
        "grant_runtime",
        "revoke_runtime",
        "node_info",
        "sign_arbitrary",
        "export_secrets",
        "send_message",
    ]
    .iter()
    .enumerate()
    {
        let request = if *method == "send_message" {
            json!({"conversationId":group,"text":"not allowed","operationId":"read-only-send"})
        } else {
            json!({"name":"Intruder"})
        };
        assert!(
            alice
                .agent_call(
                    &agent_proof(&key, &grant, method, request, 10 + i as u8),
                    NOW
                )
                .is_err()
        );
    }
    alice.agent_call(&valid, NOW).unwrap();
    assert!(alice.agent_call(&valid, NOW).is_err());
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert!(
        alice.agent_call(&valid, NOW).is_err(),
        "replay state must survive restart"
    );
    alice
        .agent_call(&agent_proof(&key, &grant, "snapshot", json!({}), 3), NOW)
        .unwrap();
    alice.revoke_runtime(grant.grant_id, NOW).unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert!(
        alice
            .agent_call(&agent_proof(&key, &grant, "snapshot", json!({}), 4), NOW)
            .is_err()
    );
    assert_eq!(alice.snapshot().unwrap().identity.unwrap().name, "Alice");
    assert!(
        alice.snapshot().unwrap().conversations[0]
            .messages
            .is_empty()
    );
}

#[test]
fn fresh_proof_retries_original_agent_operation_without_crypto_advance_or_duplicate_delivery() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let key = runtime_key(44);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::SendMessage]);
    let request =
        json!({"conversationId":group,"text":"Retry exactly once","operationId":"same-op"});
    let first = alice
        .agent_call(
            &agent_proof(&key, &grant, "send_message", request.clone(), 1),
            NOW,
        )
        .unwrap();
    let before_retry = persisted_state(&ar);
    let original_wire = alice.outbox(100).unwrap()[0].wire.clone();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let retry_proof = agent_proof(&key, &grant, "send_message", request, 2);
    let retry = alice.agent_call(&retry_proof, NOW).unwrap();
    assert_eq!(retry["id"], first["id"]);
    let after_retry = persisted_state(&ar);
    assert_eq!(after_retry.1, before_retry.1);
    assert_eq!(
        after_retry
            .0
            .iter()
            .filter(|s| !s.0.starts_with("authorization/"))
            .collect::<Vec<_>>(),
        before_retry
            .0
            .iter()
            .filter(|s| !s.0.starts_with("authorization/"))
            .collect::<Vec<_>>()
    );
    assert_ne!(
        after_retry.0, before_retry.0,
        "fresh nonce must be persisted"
    );
    assert_eq!(alice.outbox(100).unwrap()[0].wire, original_wire);
    assert!(
        alice.agent_call(&retry_proof, NOW).is_err(),
        "matching operation retry still consumes the fresh proof nonce"
    );
    let changed = agent_proof(
        &key,
        &grant,
        "send_message",
        json!({"conversationId":group,"text":"Changed text","operationId":"same-op"}),
        3,
    );
    assert!(alice.agent_call(&changed, NOW).is_err());
    assert_eq!(persisted_state(&ar), after_retry);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(persisted_state(&ar), after_retry);
    let queued = alice.outbox(100).unwrap();
    assert_eq!(queued.len(), 1);
    let receipt = bob.receive(&queued[0].wire, NOW).unwrap().reply.unwrap();
    alice.receive(&receipt, NOW).unwrap();
    let second=alice.agent_call(&agent_proof(&key,&grant,"send_message",json!({"conversationId":group,"text":"Next valid ratchet message","operationId":"next-op"}),4),NOW).unwrap();
    let queued = alice.outbox(100).unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].message_id, second["id"]);
    bob.receive(&queued[0].wire, NOW).unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0]
            .messages
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Retry exactly once", "Next valid ratchet message"]
    );
    alice.revoke_runtime(grant.grant_id, NOW).unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let revoked_state = persisted_state(&ar);
    let revoked_retry = agent_proof(
        &key,
        &grant,
        "send_message",
        json!({"conversationId":group,"text":"Retry exactly once","operationId":"same-op"}),
        5,
    );
    assert!(alice.agent_call(&revoked_retry, NOW).is_err());
    assert_eq!(persisted_state(&ar), revoked_state);
}

#[test]
fn failed_authorization_commit_rolls_back_message_mls_outbox_and_nonce_together() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let key = runtime_key(45);
    let grant = grant_runtime(&mut alice, &key, &group, &[Action::SendMessage]);
    let proof = agent_proof(
        &key,
        &grant,
        "send_message",
        json!({"conversationId":group,"text":"Atomic authorized send","operationId":"fault-op"}),
        1,
    );
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    let before = persisted_state(&ar);
    db.execute_batch(&format!("PRAGMA key = \"x'{}'\"; CREATE TRIGGER fail_authorization BEFORE INSERT ON states WHEN NEW.namespace LIKE 'authorization/%' BEGIN SELECT RAISE(ABORT,'injected authorization failure'); END;",hex::encode(KEY))).unwrap();
    assert!(alice.agent_call(&proof, NOW).is_err());
    assert!(
        alice.snapshot().unwrap().conversations[0]
            .messages
            .is_empty()
    );
    assert!(alice.outbox(100).unwrap().is_empty());
    assert_eq!(persisted_state(&ar), before);
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(persisted_state(&ar), before);
    db.execute_batch("DROP TRIGGER fail_authorization").unwrap();
    drop(db);
    alice.agent_call(&proof, NOW).unwrap();
    let queued = alice.outbox(100).unwrap();
    assert_eq!(queued.len(), 1);
    bob.receive(&queued[0].wire, NOW).unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Atomic authorized send"
    );
}

#[test]
fn operation_ids_are_isolated_between_owner_and_authenticated_runtimes() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    alice
        .send_message(&group, "Owner message", "shared-id", NOW)
        .unwrap();
    for (n, text) in [(47, "First runtime"), (48, "Second runtime")] {
        let key = runtime_key(n);
        let grant = grant_runtime(&mut alice, &key, &group, &[Action::SendMessage]);
        assert!(
            alice
                .agent_call(&agent_proof(&key, &grant, "snapshot", json!({}), 1), NOW)
                .is_err()
        );
        let request = json!({"conversationId":group,"text":text,"operationId":"shared-id"});
        alice
            .agent_call(&agent_proof(&key, &grant, "send_message", request, 2), NOW)
            .unwrap();
        let before = persisted_state(&ar);
        let changed_nonce = agent_proof(
            &key,
            &grant,
            "send_message",
            json!({"conversationId":group,"text":"Replayed nonce on another command","operationId":"different-op"}),
            2,
        );
        assert!(alice.agent_call(&changed_nonce, NOW).is_err());
        assert_eq!(persisted_state(&ar), before);
    }
    let queued = alice.outbox(100).unwrap();
    assert_eq!(queued.len(), 3);
    for item in queued {
        let receipt = bob.receive(&item.wire, NOW).unwrap().reply.unwrap();
        alice.receive(&receipt, NOW).unwrap();
    }
    assert!(alice.outbox(100).unwrap().is_empty());
    assert_eq!(
        bob.snapshot().unwrap().conversations[0]
            .messages
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Owner message", "First runtime", "Second runtime"]
    );
}

#[test]
fn owner_registration_rejects_unknown_conversations_unsupported_actions_and_active_duplicate() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let request = RuntimeGrantRequest {
        name: "Runtime".into(),
        principal: runtime_key(49).verifying_key().to_bytes(),
        agent_id: [61; 32],
        service_id: [62; 32],
        conversation_ids: vec![group],
        actions: [Action::ReadInbox].into(),
        expires_at: NOW + 3600,
        max_data_bytes: 4096,
    };
    let mut unknown = request.clone();
    unknown.conversation_ids = vec!["unknown".into()];
    let mut unsupported = request.clone();
    unsupported.actions.insert(Action::ManageGroup);
    let before = persisted_state(&ar);
    assert!(alice.grant_runtime(unknown, NOW).is_err());
    assert!(alice.grant_runtime(unsupported, NOW).is_err());
    assert_eq!(persisted_state(&ar), before);
    alice.grant_runtime(request.clone(), NOW).unwrap();
    let before_duplicate = persisted_state(&ar);
    assert!(alice.grant_runtime(request, NOW).is_err());
    assert_eq!(persisted_state(&ar), before_duplicate);
}

#[test]
fn broker_checks_actual_text_size_strict_proof_lifetime_and_its_own_authority_context() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let group = connect(&mut alice, &mut bob);
    let key = runtime_key(46);
    let grant = grant_runtime(
        &mut alice,
        &key,
        &group,
        &[Action::ReadInbox, Action::SendMessage],
    );
    let oversized = "ä".repeat(2049); // 2049 chars, 4098 bytes: over the runtime grant, valid for the owner
    let request = json!({"conversationId":group,"text":oversized,"operationId":"large"});
    assert!(
        alice
            .agent_call(&agent_proof(&key, &grant, "send_message", request, 1), NOW)
            .is_err()
    );
    assert!(alice.outbox(100).unwrap().is_empty());
    alice
        .send_message(&group, &oversized, "owner-large", NOW)
        .unwrap(); // Valid message, exceeds only the runtime's grant.
    let smuggled = json!({"conversationId":group,"text":"scope injection","operationId":"smuggle","principal":"owner","dataBytes":0});
    assert!(
        alice
            .agent_call(&agent_proof(&key, &grant, "send_message", smuggled, 2), NOW)
            .is_err()
    );
    let call = AgentCall {
        grant_id: grant.grant_id,
        method: "snapshot".into(),
        request: json!({}),
        nonce: [3; 32],
    };
    let valid = call.draft(DOMAIN, 0, NOW).unwrap();
    let mut wrong_domain = valid.clone();
    wrong_domain.domain = [90; 32];
    let mut wrong_epoch = valid.clone();
    wrong_epoch.authority_epoch = 1;
    let mut no_expiry = valid.clone();
    no_expiry.expires_at = None;
    let mut too_long = valid.clone();
    too_long.expires_at = Some(NOW + 31);
    for draft in [wrong_domain, wrong_epoch, no_expiry, too_long] {
        assert!(
            alice
                .agent_call(&SignedDocument::sign(draft, &key).unwrap().to_wire(), NOW)
                .is_err()
        );
    }
    alice
        .agent_call(&SignedDocument::sign(valid, &key).unwrap().to_wire(), NOW)
        .unwrap();
    let late = AgentCall {
        grant_id: grant.grant_id,
        method: "snapshot".into(),
        request: json!({}),
        nonce: [4; 32],
    }
    .draft(DOMAIN, 0, NOW + 3600)
    .unwrap();
    let late = SignedDocument::sign(late, &key).unwrap().to_wire();
    VerifiedDocument::decode(&late, DOMAIN, NOW + 3600).unwrap();
    assert!(
        alice.agent_call(&late, NOW + 3600).is_err(),
        "fresh proof does not renew expired grant"
    );
    assert_eq!(alice.snapshot().unwrap().conversations[0].messages.len(), 1);
}
fn core(root: &TempDir, domain: [u8; 32]) -> AppCore {
    AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &KEY).unwrap(),
        domain,
    )
    .unwrap()
}
fn profile(root: &TempDir, name: &str) -> AppCore {
    let mut c = core(root, DOMAIN);
    c.create_profile(name).unwrap();
    c
}
/// Full logical MLS state bytes: the `mls` prefix plus record fragments in
/// binary key order, matching production's state_records/restore_records path.
/// A raw keyed connection avoids the profile lock held by a live AppCore.
fn mls_logical_bytes(root: &TempDir) -> Vec<u8> {
    let db = profile_db::encrypted_db(root);
    let mut bytes: Vec<u8> = db
        .query_row("SELECT bytes FROM states WHERE namespace='mls'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let mut statement = db
        .prepare("SELECT bytes FROM state_records WHERE namespace='mls' ORDER BY record_key")
        .unwrap();
    let rows = statement
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .unwrap();
    for row in rows {
        bytes.extend(row.unwrap());
    }
    bytes
}
fn connect(alice: &mut AppCore, bob: &mut AppCore) -> String {
    let invite = bob
        .create_invitation(NOW, vec!["/ip4/127.0.0.1/tcp/4002".into()])
        .unwrap();
    let conversation = alice.add_contact("Bob", &invite, NOW).unwrap();
    let queued = alice.outbox(100).unwrap();
    assert_eq!(queued.len(), 1);
    let reply = bob.receive(&queued[0].wire, NOW).unwrap().reply.unwrap();
    assert!(alice.receive(&reply, NOW).unwrap().reply.is_none());
    assert!(alice.outbox(100).unwrap().is_empty());
    conversation.id
}
#[test]
fn onboarding_creates_a_stable_profile_without_oauth_or_reinitializing_it() {
    let root = TempDir::new().unwrap();
    let mut c = core(&root, DOMAIN);
    assert!(c.snapshot().unwrap().identity.is_none());
    let alice = c.create_profile("Alice").unwrap();
    assert_eq!(alice.name, "Alice");
    assert_eq!(c.create_profile("Alice").unwrap(), alice);
    assert!(c.create_profile("Someone else").is_err());
    drop(c);
    let c = core(&root, DOMAIN);
    assert_eq!(c.snapshot().unwrap().identity, Some(alice));
    assert_eq!(c.snapshot().unwrap().network.connected_peers, 0);
    assert!(c.snapshot().unwrap().conversations.is_empty());
}
#[test]
fn two_independent_profiles_exchange_messages_with_real_mls_and_authenticated_acknowledgments() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut alice, &mut bob);
    assert_eq!(alice.snapshot().unwrap().conversations.len(), 1);
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
    let text = "Hi, Bob. This is a real encrypted conversation.";
    let sent = alice.send_message(&id, text, "send-1", NOW).unwrap();
    assert_eq!(sent.text, text);
    assert_eq!(sent.delivery.phase, "queued");
    assert_eq!(sent.delivery.replicas, 0);
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    assert!(!wire.windows(text.len()).any(|p| p == text.as_bytes()));
    let receipt = bob.receive(&wire, NOW).unwrap().reply.unwrap();
    let bs = bob.snapshot().unwrap();
    assert_eq!(bs.conversations[0].messages.len(), 1);
    assert_eq!(bs.conversations[0].messages[0].text, text);
    assert!(!bs.conversations[0].messages[0].own);
    alice.receive(&receipt, NOW).unwrap();
    assert!(alice.outbox(100).unwrap().is_empty());
    assert_eq!(
        alice.snapshot().unwrap().conversations[0].messages[0]
            .delivery
            .phase,
        "delivered"
    );
    bob.send_message(&id, "Received!", "bob-reply", NOW + 1)
        .unwrap();
    let reply = bob.outbox(100).unwrap()[0].wire.clone();
    let ack = alice.receive(&reply, NOW + 1).unwrap().reply.unwrap();
    bob.receive(&ack, NOW + 1).unwrap();
    assert_eq!(
        alice.snapshot().unwrap().conversations[0].messages[1].text,
        "Received!"
    );
    assert_eq!(bob.snapshot().unwrap().conversations[0].messages.len(), 2);
    assert!(bob.outbox(100).unwrap().is_empty());
}
#[test]
fn lost_response_retry_and_duplicate_network_delivery_are_one_history_event_after_restart() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut alice, &mut bob);
    let original = alice
        .send_message(&id, "Saved once", "op-stable", NOW)
        .unwrap();
    let original_wire = alice.outbox(100).unwrap()[0].wire.clone();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    let before_retry = persisted_state(&ar);
    assert_eq!(
        alice
            .send_message(&id, "Saved once", "op-stable", NOW + 10)
            .unwrap()
            .id,
        original.id
    );
    assert_eq!(persisted_state(&ar), before_retry);
    assert_eq!(alice.outbox(100).unwrap().len(), 1);
    assert_eq!(alice.outbox(100).unwrap()[0].wire, original_wire);
    let first = bob
        .receive(&original_wire, NOW + 10)
        .unwrap()
        .reply
        .unwrap();
    drop(bob);
    let mut bob = core(&br, DOMAIN);
    let before_duplicate = persisted_state(&br);
    let repeated = bob
        .receive(&original_wire, NOW + 11)
        .unwrap()
        .reply
        .unwrap();
    assert_eq!(persisted_state(&br), before_duplicate);
    assert_eq!(bob.snapshot().unwrap().conversations[0].messages.len(), 1);
    alice.receive(&first, NOW + 11).unwrap();
    alice.receive(&repeated, NOW + 11).unwrap();
    assert_eq!(
        alice
            .send_message(&id, "Saved once", "op-stable", NOW + 12)
            .unwrap()
            .id,
        original.id
    );
    assert!(alice.outbox(100).unwrap().is_empty());
    assert_eq!(alice.snapshot().unwrap().conversations[0].messages.len(), 1);
    alice
        .send_message(&id, "Next ratchet state", "next", NOW + 12)
        .unwrap();
    let next = alice.outbox(100).unwrap()[0].wire.clone();
    bob.receive(&next, NOW + 12).unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[1].text,
        "Next ratchet state"
    );
}
#[test]
fn a_changed_command_with_the_same_operation_id_cannot_advance_history_or_outbox() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut alice, &mut bob);
    alice.send_message(&id, "First", "op-1", NOW).unwrap();
    let original = alice.outbox(100).unwrap()[0].wire.clone();
    let before_rejection = persisted_state(&ar);
    assert!(alice.send_message(&id, "Changed", "op-1", NOW).is_err());
    assert_eq!(persisted_state(&ar), before_rejection);
    assert_eq!(alice.outbox(100).unwrap().len(), 1);
    assert_eq!(alice.outbox(100).unwrap()[0].wire, original);
    assert_eq!(alice.snapshot().unwrap().conversations[0].messages.len(), 1);
}
#[test]
fn foreign_expired_and_tampered_invitations_do_not_create_contacts() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    let other = TempDir::new().unwrap();
    let mut foreign = core(&other, [8; 32]);
    foreign.create_profile("Foreign").unwrap();
    assert!(foreign.add_contact("Bob", &invite, NOW).is_err());
    assert!(
        alice
            .add_contact("Bob", &invite, NOW + 7 * 86400 + 1)
            .is_err()
    );
    let mut bytes = hex::decode(invite.strip_prefix("ain-invite1:").unwrap()).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert!(
        alice
            .add_contact("Bob", &format!("ain-invite1:{}", hex::encode(bytes)), NOW)
            .is_err()
    );
    assert!(alice.snapshot().unwrap().conversations.is_empty());
    assert!(alice.outbox(100).unwrap().is_empty());
    assert!(alice.add_contact("Bob", &invite, NOW).is_ok());
}
#[test]
fn duplicate_invitation_and_welcome_do_not_duplicate_contacts_or_consume_another_ratchet_step() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    let first = alice.add_contact("Bob", &invite, NOW).unwrap();
    assert_eq!(
        alice.add_contact("Bob", &invite, NOW + 1).unwrap().id,
        first.id
    );
    assert_eq!(alice.outbox(100).unwrap().len(), 1);
    let welcome = alice.outbox(100).unwrap()[0].wire.clone();
    let ack = bob.receive(&welcome, NOW + 1).unwrap().reply.unwrap();
    bob.receive(&welcome, NOW + 2).unwrap();
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
    assert!(bob.snapshot().unwrap().conversations[0].messages.is_empty());
    alice.receive(&ack, NOW + 2).unwrap();
    alice
        .send_message(&first.id, "After duplicate Welcome", "send", NOW + 3)
        .unwrap();
    bob.receive(&alice.outbox(100).unwrap()[0].wire, NOW + 3)
        .unwrap();
    assert_eq!(bob.snapshot().unwrap().conversations[0].messages.len(), 1);
}
fn resign_with_another_root(wire: &[u8], now: u64) -> Vec<u8> {
    let original = VerifiedDocument::decode(wire, DOMAIN, now).unwrap();
    let draft = DocumentDraft {
        domain: DOMAIN,
        kind: original.kind(),
        authority_epoch: 0,
        issued_at: now,
        expires_at: original.expires_at(),
        body: original.body().to_vec(),
        extensions: BTreeMap::new(),
    };
    SignedDocument::sign(draft, &SigningKey::from_bytes(&[0x65; 32]))
        .unwrap()
        .to_wire()
}
#[test]
fn a_valid_signature_from_the_wrong_root_cannot_relabel_mls_sender_or_acknowledge_delivery() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut alice, &mut bob);
    alice
        .send_message(&id, "Root and MLS must agree", "op", NOW)
        .unwrap();
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    let forged = resign_with_another_root(&wire, NOW);
    assert!(VerifiedDocument::decode(&forged, DOMAIN, NOW).is_ok());
    assert!(bob.receive(&forged, NOW).is_err());
    assert!(bob.snapshot().unwrap().conversations[0].messages.is_empty());
    let receipt = bob.receive(&wire, NOW).unwrap().reply.unwrap();
    let forged_receipt = resign_with_another_root(&receipt, NOW);
    assert!(alice.receive(&forged_receipt, NOW).is_err());
    assert_eq!(alice.outbox(100).unwrap().len(), 1);
    alice.receive(&receipt, NOW).unwrap();
    assert!(alice.outbox(100).unwrap().is_empty());
}
#[test]
fn invitation_key_package_credential_must_match_the_signing_root() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    let wire = hex::decode(invite.strip_prefix("ain-invite1:").unwrap()).unwrap();
    let forged = resign_with_another_root(&wire, NOW);
    assert_eq!(
        VerifiedDocument::decode(&forged, DOMAIN, NOW)
            .unwrap()
            .kind(),
        DocumentKind::Invitation
    );
    assert!(
        alice
            .add_contact(
                "Impostor",
                &format!("ain-invite1:{}", hex::encode(forged)),
                NOW
            )
            .is_err()
    );
    assert!(alice.snapshot().unwrap().conversations.is_empty());
}
#[test]
fn send_database_failure_leaves_no_outgoing_message_and_retry_still_decrypts() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut alice, &mut bob);
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_send BEFORE INSERT ON outbox BEGIN SELECT RAISE(ABORT,'disk full'); END;",hex::encode(KEY))).unwrap();
    let before_failure = persisted_state(&ar);
    assert!(
        alice
            .send_message(&id, "Must not disappear", "retry", NOW)
            .is_err()
    );
    assert_eq!(persisted_state(&ar), before_failure);
    assert!(
        alice.snapshot().unwrap().conversations[0]
            .messages
            .is_empty()
    );
    assert!(alice.outbox(100).unwrap().is_empty());
    db.execute_batch("DROP TRIGGER fail_send").unwrap();
    drop(alice);
    let mut alice = core(&ar, DOMAIN);
    assert_eq!(persisted_state(&ar), before_failure);
    alice
        .send_message(&id, "Must not disappear", "retry", NOW + 1)
        .unwrap();
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    bob.receive(&wire, NOW + 1).unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Must not disappear"
    );
}
#[test]
fn ui_snapshot_uses_expected_camel_case_contract_and_does_not_expose_crypto_state() {
    let root = TempDir::new().unwrap();
    let c = profile(&root, "Alice");
    let json = serde_json::to_value(c.snapshot().unwrap()).unwrap();
    assert!(
        json["identity"]["networkId"]
            .as_str()
            .unwrap()
            .starts_with("ain1")
    );
    assert_eq!(json["network"]["connectedPeers"], 0);
    assert_eq!(json["network"]["state"], "offline");
    let text = json.to_string();
    for secret_field in ["owner_seed", "private_key", "next_state", "signer_public"] {
        assert!(!text.contains(secret_field));
    }
}

#[test]
fn an_invitation_consumed_by_one_sender_cannot_create_a_second_session_for_another() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let er = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut eve = profile(&er, "Eve");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    alice.add_contact("Bob", &invite, NOW).unwrap();
    eve.add_contact("Bob", &invite, NOW).unwrap();
    let original = alice.outbox(100).unwrap()[0].wire.clone();
    bob.receive(&original, NOW).unwrap();
    let competing = eve.outbox(100).unwrap()[0].wire.clone();
    assert!(bob.receive(&competing, NOW).is_err());
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
    assert!(bob.receive(&original, NOW + 1).is_ok());
}
#[test]
fn profile_and_message_validation_rejects_empty_data_without_silent_setup_or_queue_entries() {
    let root = TempDir::new().unwrap();
    let mut fresh = core(&root, DOMAIN);
    assert!(fresh.create_invitation(NOW, vec![]).is_err());
    for name in ["", "  ", "hidden\ncontrol"] {
        assert!(fresh.create_profile(name).is_err());
    }
    assert!(fresh.snapshot().unwrap().identity.is_none());
    let br = TempDir::new().unwrap();
    fresh.create_profile("Alice").unwrap();
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut fresh, &mut bob);
    for text in ["".to_owned(), "   ".to_owned(), "x".repeat(12_001)] {
        assert!(fresh.send_message(&id, &text, "invalid", NOW).is_err());
    }
    assert!(
        fresh
            .send_message("missing-conversation", "hello", "missing", NOW)
            .is_err()
    );
    assert!(
        fresh.snapshot().unwrap().conversations[0]
            .messages
            .is_empty()
    );
    assert!(fresh.outbox(100).unwrap().is_empty());
}
#[test]
fn operation_id_reuse_for_another_recipient_is_a_conflict() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let cr = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let mut charlie = profile(&cr, "Charlie");
    let first = connect(&mut alice, &mut bob);
    let second = connect(&mut alice, &mut charlie);
    alice
        .send_message(&first, "Same text", "same-operation", NOW)
        .unwrap();
    assert!(
        alice
            .send_message(&second, "Same text", "same-operation", NOW)
            .is_err()
    );
    assert_eq!(alice.outbox(100).unwrap().len(), 1);
    let snapshot = alice.snapshot().unwrap();
    assert!(
        snapshot
            .conversations
            .iter()
            .find(|c| c.id == second)
            .unwrap()
            .messages
            .is_empty()
    );
}

fn persisted_state(root: &TempDir) -> (Vec<(String, i64, Vec<u8>)>, i64) {
    let db = rusqlite::Connection::open(root.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\";", hex::encode(KEY)))
        .unwrap();
    let mut query = db
        .prepare("SELECT namespace,revision,bytes FROM states ORDER BY namespace")
        .unwrap();
    let states = query
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let operations = db
        .query_row("SELECT count(*) FROM operations", [], |r| r.get(0))
        .unwrap();
    (states, operations)
}

#[test]
fn first_welcome_cannot_bind_a_valid_unrelated_root_to_the_original_creator_leaf() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    alice.add_contact("Bob", &invite, NOW).unwrap();
    let original = alice.outbox(100).unwrap()[0].wire.clone();
    let forged = resign_with_another_root(&original, NOW);
    assert_eq!(
        VerifiedDocument::decode(&forged, DOMAIN, NOW)
            .unwrap()
            .kind(),
        DocumentKind::GroupControl
    );
    let before = persisted_state(&br);
    assert!(bob.receive(&forged, NOW).is_err());
    assert_eq!(persisted_state(&br), before);
    assert!(bob.snapshot().unwrap().conversations.is_empty());
    assert!(bob.outbox(100).unwrap().is_empty());
    drop(bob);
    let mut bob = core(&br, DOMAIN);
    assert_eq!(persisted_state(&br), before);
    assert!(bob.receive(&original, NOW).unwrap().reply.is_some());
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
}
#[test]
fn failure_while_accepting_welcome_does_not_consume_invitation_or_save_partial_contact() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    alice.add_contact("Bob", &invite, NOW).unwrap();
    let welcome = alice.outbox(100).unwrap()[0].wire.clone();
    let before = persisted_state(&br);
    let db = rusqlite::Connection::open(br.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key=\"x'{}'\"; CREATE TRIGGER fail_welcome BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT,'disk full'); END;",hex::encode(KEY))).unwrap();
    assert!(bob.receive(&welcome, NOW).is_err());
    assert_eq!(persisted_state(&br), before);
    db.execute_batch("DROP TRIGGER fail_welcome").unwrap();
    drop(bob);
    let mut bob = core(&br, DOMAIN);
    assert_eq!(persisted_state(&br), before);
    assert!(bob.snapshot().unwrap().conversations.is_empty());
    bob.receive(&welcome, NOW).unwrap();
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
}
#[test]
fn receipts_bind_exact_message_and_conversation_and_correct_peer_cannot_ack_a_different_pair() {
    use sha2::{Digest, Sha256};
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut alice, &mut bob);
    let sent = alice
        .send_message(&id, "Check receipt binding", "receipt-test", NOW)
        .unwrap();
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    let receipt = bob.receive(&wire, NOW).unwrap().reply.unwrap();
    let document = VerifiedDocument::decode(&receipt, DOMAIN, NOW).unwrap();
    assert_eq!(document.kind(), DocumentKind::Message);
    assert_eq!(
        format!("ain1{}", hex::encode(Sha256::digest(document.author()))),
        bob.snapshot().unwrap().identity.unwrap().network_id
    );
    let mut body = minicbor::Decoder::new(document.body());
    assert_eq!(body.array().unwrap(), Some(3));
    assert_eq!(body.u8().unwrap(), 4);
    assert_eq!(body.bytes().unwrap(), hex::decode(&id).unwrap());
    assert_eq!(body.bytes().unwrap(), hex::decode(&sent.id).unwrap());
    assert_eq!(body.position(), document.body().len());
    drop(bob);
    let signer = ProfileStore::open(br.path().join("profile.db"), &KEY).unwrap();
    for wrong_group in [true, false] {
        let mut body = minicbor::Encoder::new(Vec::new());
        body.array(3).unwrap().u8(4).unwrap();
        body.bytes(&if wrong_group {
            vec![0x63; 32]
        } else {
            hex::decode(&id).unwrap()
        })
        .unwrap();
        body.bytes(&if wrong_group {
            hex::decode(&sent.id).unwrap()
        } else {
            vec![0x55; 32]
        })
        .unwrap();
        let draft = DocumentDraft {
            domain: DOMAIN,
            kind: DocumentKind::Message,
            authority_epoch: 0,
            issued_at: NOW,
            expires_at: None,
            body: body.into_writer(),
            extensions: BTreeMap::new(),
        };
        let wrong = signer.sign_document(draft).unwrap().to_wire();
        assert!(alice.receive(&wrong, NOW).is_err());
        assert_eq!(alice.outbox(100).unwrap().len(), 1);
    }
    alice.receive(&receipt, NOW).unwrap();
    let snapshot = serde_json::to_value(alice.snapshot().unwrap()).unwrap();
    let message = &snapshot["conversations"][0]["messages"][0];
    assert_eq!(message["createdAt"], NOW);
    assert_eq!(message["delivery"]["phase"], "delivered");
    assert_eq!(message["delivery"]["replicas"], 0);
    assert_eq!(message["delivery"]["target"], 10);
    assert_eq!(snapshot["conversations"][0]["unread"], 0);
    assert_eq!(message["author"], snapshot["identity"]["networkId"]);
}
#[test]
fn repeated_profile_setup_after_join_does_not_reset_conversation_crypto() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let id = connect(&mut alice, &mut bob);
    let before = persisted_state(&ar);
    alice.create_profile("Alice").unwrap();
    assert_eq!(persisted_state(&ar), before);
    alice
        .send_message(&id, "Profile survived", "same-profile", NOW)
        .unwrap();
    bob.receive(&alice.outbox(100).unwrap()[0].wire, NOW)
        .unwrap();
    assert_eq!(
        bob.snapshot().unwrap().conversations[0].messages[0].text,
        "Profile survived"
    );
}

#[test]
fn authenticated_transport_binding_persists_reverse_route_with_welcome_after_restart() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    let group = alice.add_contact("Bob", &invite, NOW).unwrap().id;
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    let addresses = vec!["/ip4/127.0.0.1/tcp/4101/p2p/peer-A".to_string()];
    let binding = alice
        .create_node_record("peer-A", addresses.clone(), NOW)
        .unwrap();
    let record = VerifiedDocument::decode(&binding, DOMAIN, NOW).unwrap();
    assert_eq!(record.kind(), DocumentKind::Identity);
    assert_eq!(record.expires_at(), Some(NOW + 86400));
    let mut body = minicbor::Decoder::new(record.body());
    assert_eq!(body.array().unwrap(), Some(3));
    assert_eq!(body.u8().unwrap(), 1);
    assert_eq!(body.str().unwrap(), "peer-A");
    assert_eq!(body.array().unwrap(), Some(1));
    assert_eq!(body.str().unwrap(), addresses[0]);
    assert_eq!(body.position(), record.body().len());
    bob.receive_from(&wire, &binding, "peer-A", NOW).unwrap();
    drop(bob);
    let mut bob = core(&br, DOMAIN);
    bob.send_message(
        &group,
        "Reverse route survives restart",
        "reply-route",
        NOW + 1,
    )
    .unwrap();
    assert_eq!(bob.outbox(100).unwrap()[0].addresses, addresses);
}

#[test]
fn invalid_node_binding_cannot_consume_invitation_or_advance_crypto() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let er = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let eve = profile(&er, "Eve");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    alice.add_contact("Bob", &invite, NOW).unwrap();
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    let binding = alice.create_node_record("peer-A", vec![], NOW).unwrap();
    let wrong_root = eve.create_node_record("peer-A", vec![], NOW).unwrap();
    let before = persisted_state(&br);
    assert!(bob.receive_from(&wire, &binding, "peer-E", NOW).is_err());
    assert!(bob.receive_from(&wire, &wrong_root, "peer-A", NOW).is_err());
    assert!(
        bob.receive_from(&wire, &binding, "peer-A", NOW + 86400)
            .is_err()
    );
    let mut tampered = binding.clone();
    *tampered.last_mut().unwrap() ^= 1;
    assert!(bob.receive_from(&wire, &tampered, "peer-A", NOW).is_err());
    assert_eq!(persisted_state(&br), before);
    assert!(bob.snapshot().unwrap().conversations.is_empty());
    drop(bob);
    let mut bob = core(&br, DOMAIN);
    assert!(
        bob.receive_from(&wire, &binding, "peer-A", NOW)
            .unwrap()
            .reply
            .is_some()
    );
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
}

#[test]
fn transport_hints_and_welcome_roll_back_together_when_durable_commit_fails() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    let group = alice.add_contact("Bob", &invite, NOW).unwrap().id;
    let wire = alice.outbox(100).unwrap()[0].wire.clone();
    let binding = alice
        .create_node_record("peer-A", vec!["/ip4/127.0.0.1/tcp/4101".into()], NOW)
        .unwrap();
    let db = rusqlite::Connection::open(br.path().join("profile.db")).unwrap();
    db.pragma_update(None, "key", format!("x'{}'", hex::encode(KEY)))
        .unwrap();
    db.execute_batch("CREATE TRIGGER fail_welcome BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT, 'disk failure'); END;").unwrap();
    let before = persisted_state(&br);
    assert!(bob.receive_from(&wire, &binding, "peer-A", NOW).is_err());
    assert_eq!(persisted_state(&br), before);
    db.execute_batch("DROP TRIGGER fail_welcome;").unwrap();
    drop(db);
    drop(bob);
    let mut bob = core(&br, DOMAIN);
    bob.receive_from(&wire, &binding, "peer-A", NOW).unwrap();
    bob.send_message(&group, "retry works", "reply", NOW)
        .unwrap();
    assert_eq!(
        bob.outbox(10).unwrap()[0].addresses,
        vec!["/ip4/127.0.0.1/tcp/4101"]
    );
}

#[test]
fn root_signed_node_records_still_require_bounded_lifetime_and_canonical_body() {
    let ar = TempDir::new().unwrap();
    let br = TempDir::new().unwrap();
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invitation = bob.create_invitation(NOW, vec![]).unwrap();
    alice.add_contact("Bob", &invitation, NOW).unwrap();
    let wire = alice.outbox(10).unwrap()[0].wire.clone();
    let record = alice.create_node_record("peer-A", vec![], NOW).unwrap();
    let parsed = VerifiedDocument::decode(&record, DOMAIN, NOW).unwrap();
    let db = rusqlite::Connection::open(ar.path().join("profile.db")).unwrap();
    db.pragma_update(None, "key", format!("x'{}'", hex::encode(KEY)))
        .unwrap();
    let seed: Vec<u8> = db
        .query_row(
            "SELECT owner_seed FROM profile WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let key = SigningKey::from_bytes(&seed.try_into().unwrap());
    let mut noncanonical = parsed.body().to_vec();
    noncanonical.splice(1..2, [0x18, 0x01]);
    let mut trailing = parsed.body().to_vec();
    trailing.push(0);
    let before = persisted_state(&br);
    for (domain, expiry, body) in [
        (DOMAIN, None, parsed.body().to_vec()),
        (DOMAIN, Some(NOW + 86401), parsed.body().to_vec()),
        (DOMAIN, Some(NOW + 86400), noncanonical),
        (DOMAIN, Some(NOW + 86400), trailing),
        ([8; 32], Some(NOW + 86400), parsed.body().to_vec()),
    ] {
        let forged = SignedDocument::sign(
            DocumentDraft {
                domain,
                kind: DocumentKind::Identity,
                authority_epoch: 0,
                issued_at: NOW,
                expires_at: expiry,
                body,
                extensions: BTreeMap::new(),
            },
            &key,
        )
        .unwrap()
        .to_wire();
        assert!(bob.receive_from(&wire, &forged, "peer-A", NOW).is_err());
        assert_eq!(persisted_state(&br), before);
    }
    bob.receive_from(&wire, &record, "peer-A", NOW).unwrap();
    assert_eq!(bob.snapshot().unwrap().conversations.len(), 1);
}
