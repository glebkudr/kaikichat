#![allow(clippy::unwrap_used, clippy::expect_used)]
use agentic_capabilities::{
    Action, AuthContext, CapabilityError, GrantChain, GrantClaims, ScopeRequest, prepare_debit,
};
use agentic_protocol::{DocumentDraft, DocumentKind, SignedDocument, VerifiedDocument, network_id};
use agentic_store::{ProfileStore, StateChange};
use ed25519_dalek::SigningKey;
use minicbor::Encoder;
use std::collections::{BTreeMap, BTreeSet};

const DOMAIN: [u8; 32] = [81; 32];
const NOW: u64 = 1_789_000_000;
fn key(n: u8) -> SigningKey {
    SigningKey::from_bytes(&[n; 32])
}
fn public(n: u8) -> [u8; 32] {
    key(n).verifying_key().to_bytes()
}
fn set<T: Ord>(items: impl IntoIterator<Item = T>) -> BTreeSet<T> {
    items.into_iter().collect()
}
fn claims(owner: [u8; 32], subject: [u8; 32]) -> GrantClaims {
    GrantClaims {
        owner,
        agent: [31; 32],
        service: [32; 32],
        subject,
        parent: None,
        device_epoch: 3,
        service_epoch: 7,
        actions: set([Action::ReadInbox, Action::SendMessage, Action::SpendPostage]),
        resources: set(["conversation:project".into(), "conversation:team".into()]),
        recipients: set([network_id(&public(8)), network_id(&public(9))]),
        budget_asset: "ain:postage:storage-v1".into(),
        budget_units: 100,
        max_data_bytes: 4096,
        remaining_depth: 2,
        no_subcontract: true,
    }
}
fn context(owner: [u8; 32], principal: [u8; 32]) -> AuthContext {
    AuthContext {
        domain: DOMAIN,
        owner,
        ownership_epoch: 5,
        agent: [31; 32],
        service: [32; 32],
        service_epoch: 7,
        subject_epochs: BTreeMap::from([(public(2), 3), (public(3), 3), (public(4), 3)]),
        principal,
        now: NOW,
        revoked: BTreeSet::new(),
    }
}
fn sign(claims: &GrantClaims, signer: &SigningKey) -> Vec<u8> {
    SignedDocument::sign(
        claims.draft(DOMAIN, 5, NOW - 10, NOW + 3600).unwrap(),
        signer,
    )
    .unwrap()
    .to_wire()
}
fn id(wire: &[u8]) -> [u8; 32] {
    VerifiedDocument::decode(wire, DOMAIN, NOW).unwrap().id()
}
fn request(action: Action, units: u64) -> ScopeRequest {
    ScopeRequest {
        action,
        resource: "conversation:project".into(),
        recipient: Some(network_id(&public(8))),
        asset: "ain:postage:storage-v1".into(),
        units,
        data_bytes: 256,
        subcontract: false,
    }
}
fn child(parent: &[u8], owner: [u8; 32], subject: [u8; 32]) -> GrantClaims {
    let mut c = claims(owner, subject);
    c.parent = Some(id(parent));
    c.remaining_depth = 0;
    c.budget_units = 70;
    c.resources = set(["conversation:project".into()]);
    c.recipients = set([network_id(&public(8))]);
    c
}

#[test]
fn owner_agent_runtime_chain_attenuates_real_send_read_and_spending_rights() {
    let root = sign(&claims(public(1), public(2)), &key(1));
    let leaf = sign(&child(&root, public(1), public(3)), &key(2));
    let chain = GrantChain::verify(&[root, leaf], &context(public(1), public(3))).unwrap();
    chain.authorize(&request(Action::ReadInbox, 0)).unwrap();
    chain.authorize(&request(Action::SendMessage, 0)).unwrap();
    chain.authorize(&request(Action::SpendPostage, 60)).unwrap();
    let mut wrong = request(Action::ReadInbox, 0);
    wrong.resource = "conversation:team".into();
    assert!(chain.authorize(&wrong).is_err());
    wrong = request(Action::SendMessage, 0);
    wrong.recipient = Some(network_id(&public(9)));
    assert!(chain.authorize(&wrong).is_err());
    assert!(chain.authorize(&request(Action::ManageGroup, 0)).is_err());
    assert!(
        chain.authorize(&request(Action::SendMessage, 60)).is_err(),
        "send scope is not an implicit spending permission"
    );
    assert!(chain.authorize(&request(Action::SpendPostage, 71)).is_err());
    wrong = request(Action::SendMessage, 0);
    wrong.data_bytes = 4097;
    assert!(chain.authorize(&wrong).is_err());
    wrong = request(Action::SendMessage, 0);
    wrong.subcontract = true;
    assert!(chain.authorize(&wrong).is_err());
    wrong = request(Action::SendMessage, 0);
    wrong.recipient = None;
    assert!(
        chain.authorize(&wrong).is_err(),
        "missing recipient cannot bypass send restrictions"
    );
    wrong.action = Action::ReadInbox;
    chain.authorize(&wrong).unwrap();
}

#[test]
fn independent_peer_cbor_grant_is_accepted_but_unknown_action_and_noncanonical_scope_are_not() {
    // This peer assembles the body independently; it never calls GrantClaims::draft.
    fn wire(actions: &[u8], resources: &[&str]) -> Vec<u8> {
        let mut e = Encoder::new(Vec::new());
        e.array(16)
            .unwrap()
            .u8(3)
            .unwrap()
            .bytes(&public(1))
            .unwrap()
            .bytes(&[31; 32])
            .unwrap()
            .bytes(&[32; 32])
            .unwrap()
            .bytes(&public(2))
            .unwrap()
            .null()
            .unwrap()
            .u8(3)
            .unwrap()
            .u8(7)
            .unwrap();
        e.array(actions.len() as u64).unwrap();
        for a in actions {
            e.u8(*a).unwrap();
        }
        e.array(resources.len() as u64).unwrap();
        for resource in resources {
            e.str(resource).unwrap();
        }
        e.array(1)
            .unwrap()
            .str(&network_id(&public(8)))
            .unwrap()
            .str("ain:postage:storage-v1")
            .unwrap()
            .u8(100)
            .unwrap()
            .u16(4096)
            .unwrap()
            .u8(0)
            .unwrap()
            .bool(true)
            .unwrap();
        SignedDocument::sign(
            DocumentDraft {
                domain: DOMAIN,
                kind: DocumentKind::Identity,
                authority_epoch: 5,
                issued_at: NOW - 10,
                expires_at: Some(NOW + 3600),
                body: e.into_writer(),
                extensions: BTreeMap::new(),
            },
            &key(1),
        )
        .unwrap()
        .to_wire()
    }
    let ctx = context(public(1), public(2));
    let valid = wire(&[1, 2, 13], &["conversation:project"]);
    GrantChain::verify(&[valid], &ctx)
        .unwrap()
        .authorize(&request(Action::SendMessage, 0))
        .unwrap();
    for invalid in [
        wire(&[1, 99], &["conversation:project"]),
        wire(&[2, 1], &["conversation:project"]),
        wire(&[1, 1], &["conversation:project"]),
        wire(&[1], &["conversation:team", "conversation:project"]),
        wire(&[1], &["*"]),
    ] {
        assert!(GrantChain::verify(&[invalid], &ctx).is_err());
    }
}

#[test]
fn a_runtime_cannot_expand_any_parent_bound_or_forge_a_new_root() {
    let root = sign(&claims(public(1), public(2)), &key(1));
    let base = child(&root, public(1), public(3));
    let ctx = context(public(1), public(3));
    let mut attempts = Vec::new();
    let mut c = base.clone();
    c.actions.insert(Action::ManageGroup);
    attempts.push(c);
    let mut c = base.clone();
    c.resources.insert("conversation:private".into());
    attempts.push(c);
    let mut c = base.clone();
    c.recipients.insert(network_id(&public(7)));
    attempts.push(c);
    let mut c = base.clone();
    c.budget_units = 101;
    attempts.push(c);
    let mut c = base.clone();
    c.budget_asset = "other:asset".into();
    attempts.push(c);
    let mut c = base.clone();
    c.max_data_bytes = 4097;
    attempts.push(c);
    let mut c = base.clone();
    c.remaining_depth = 2;
    attempts.push(c);
    let mut c = base.clone();
    c.no_subcontract = false;
    attempts.push(c);
    let mut c = base.clone();
    c.agent = [91; 32];
    attempts.push(c);
    let mut c = base.clone();
    c.service = [91; 32];
    attempts.push(c);
    let mut c = base.clone();
    c.parent = Some([91; 32]);
    attempts.push(c);
    for invalid in attempts {
        assert!(GrantChain::verify(&[root.clone(), sign(&invalid, &key(2))], &ctx).is_err());
    }
    assert!(GrantChain::verify(&[root.clone(), sign(&base, &key(4))], &ctx).is_err());
    let forged = sign(&claims(public(1), public(3)), &key(2));
    assert!(GrantChain::verify(&[forged], &ctx).is_err());
    let mut draft = base.draft(DOMAIN, 5, NOW - 10, NOW + 7200).unwrap();
    let longer = SignedDocument::sign(draft.clone(), &key(2))
        .unwrap()
        .to_wire();
    assert!(GrantChain::verify(&[root.clone(), longer], &ctx).is_err());
    draft.expires_at = Some(NOW + 3600);
    draft.issued_at = NOW - 20;
    assert!(
        GrantChain::verify(
            &[
                root,
                SignedDocument::sign(draft, &key(2)).unwrap().to_wire()
            ],
            &ctx
        )
        .is_err()
    );
}

#[test]
fn authenticated_caller_current_epochs_network_time_and_ancestor_revocation_are_required() {
    let root = sign(&claims(public(1), public(2)), &key(1));
    let leaf = sign(&child(&root, public(1), public(3)), &key(2));
    let wires = [root.clone(), leaf.clone()];
    let base = context(public(1), public(3));
    let mut contexts = Vec::new();
    let mut c = base.clone();
    c.principal = public(4);
    contexts.push(c);
    let mut c = base.clone();
    c.owner = public(4);
    contexts.push(c);
    let mut c = base.clone();
    c.domain = [99; 32];
    contexts.push(c);
    let mut c = base.clone();
    c.agent = [98; 32];
    contexts.push(c);
    let mut c = base.clone();
    c.service = [98; 32];
    contexts.push(c);
    let mut c = base.clone();
    c.ownership_epoch += 1;
    contexts.push(c);
    let mut c = base.clone();
    c.service_epoch += 1;
    contexts.push(c);
    let mut c = base.clone();
    c.subject_epochs.insert(public(2), 4);
    contexts.push(c);
    let mut c = base.clone();
    c.subject_epochs.remove(&public(3));
    contexts.push(c);
    let mut c = base.clone();
    c.now = NOW + 3600;
    contexts.push(c);
    let mut c = base.clone();
    c.now = NOW - 11;
    contexts.push(c); // within generic signature tolerance, still not active
    let mut c = base.clone();
    c.revoked.insert(id(&root));
    contexts.push(c);
    let mut c = base.clone();
    c.revoked.insert(id(&leaf));
    contexts.push(c);
    for denied in contexts {
        assert!(GrantChain::verify(&wires, &denied).is_err());
    }
    assert!(
        GrantChain::verify(&[leaf], &base).is_err(),
        "possessing a leaf is not proof of owner delegation"
    );
}

#[test]
fn public_review_publication_amendment_withdrawal_and_provider_reply_are_separate_rights() {
    for allowed in [
        Action::PublishReview,
        Action::AmendReview,
        Action::WithdrawReview,
        Action::ReplyReview,
    ] {
        let mut c = claims(public(1), public(2));
        c.actions = set([allowed]);
        c.resources = set(["review:order-42".into()]);
        let chain =
            GrantChain::verify(&[sign(&c, &key(1))], &context(public(1), public(2))).unwrap();
        for action in [
            Action::PublishReview,
            Action::AmendReview,
            Action::WithdrawReview,
            Action::ReplyReview,
        ] {
            let mut r = request(action, 0);
            r.resource = "review:order-42".into();
            assert_eq!(chain.authorize(&r).is_ok(), action == allowed);
        }
    }
}

#[test]
fn grant_consumer_requires_correct_kind_mandatory_expiry_and_finite_lifetime() {
    let c = claims(public(1), public(2));
    let valid = c.draft(DOMAIN, 5, NOW - 10, NOW - 10 + 30 * 86400).unwrap();
    let ctx = context(public(1), public(2));
    let wire = SignedDocument::sign(valid.clone(), &key(1))
        .unwrap()
        .to_wire();
    GrantChain::verify(&[wire], &ctx).unwrap();
    let mut no_expiry = valid.clone();
    no_expiry.expires_at = None;
    let mut too_long = valid.clone();
    too_long.expires_at = Some(NOW - 10 + 30 * 86400 + 1);
    let mut wrong_kind = valid;
    wrong_kind.kind = DocumentKind::Message;
    for invalid in [no_expiry, too_long, wrong_kind] {
        let wire = SignedDocument::sign(invalid, &key(1)).unwrap().to_wire();
        VerifiedDocument::decode(&wire, DOMAIN, NOW).unwrap(); // Generic wire rules intentionally accept these.
        assert!(GrantChain::verify(&[wire], &ctx).is_err());
    }
}

struct BudgetFixture {
    root: tempfile::TempDir,
    store: ProfileStore,
    wires: Vec<Vec<u8>>,
    ctx: AuthContext,
}
impl BudgetFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = ProfileStore::open(root.path().join("profile.db"), &[55; 32]).unwrap();
        let owner = store.identity().unwrap().public_key;
        let grant = store
            .sign_document(
                claims(owner, public(2))
                    .draft(DOMAIN, 5, NOW - 10, NOW + 3600)
                    .unwrap(),
            )
            .unwrap()
            .to_wire();
        let leaf = sign(&child(&grant, owner, public(3)), &key(2));
        Self {
            root,
            store,
            wires: vec![grant, leaf],
            ctx: context(owner, public(3)),
        }
    }
    fn chain(&self) -> GrantChain {
        GrantChain::verify(&self.wires, &self.ctx).unwrap()
    }
}

#[test]
fn sibling_runtimes_and_competing_reservations_share_atomic_ancestor_budget() {
    let mut f = BudgetFixture::new();
    let a = f.chain();
    let owner = f.ctx.owner;
    let leaf_b = sign(&child(&f.wires[0], owner, public(4)), &key(2));
    let b = GrantChain::verify(&[f.wires[0].clone(), leaf_b], &context(owner, public(4))).unwrap();
    let first = prepare_debit(&f.store, &a, "first", &request(Action::SpendPostage, 60)).unwrap();
    let competing =
        prepare_debit(&f.store, &b, "second", &request(Action::SpendPostage, 60)).unwrap();
    assert!(!first.already_committed);
    assert!(!competing.already_committed);
    f.store.commit_states(first.states).unwrap();
    assert!(
        f.store.commit_states(competing.states).is_err(),
        "stale shared counter cannot overcommit"
    );
    assert!(prepare_debit(&f.store, &b, "second", &request(Action::SpendPostage, 60)).is_err());
    let remaining =
        prepare_debit(&f.store, &b, "second", &request(Action::SpendPostage, 40)).unwrap();
    f.store.commit_states(remaining.states).unwrap();
    assert!(prepare_debit(&f.store, &a, "third", &request(Action::SpendPostage, 1)).is_err());
}

#[test]
fn restart_replays_exact_authorized_operation_without_second_charge_and_rejects_changed_retry() {
    let mut f = BudgetFixture::new();
    let mut leaf = child(&f.wires[0], f.ctx.owner, public(3));
    leaf.resources.insert("conversation:team".into());
    f.wires[1] = sign(&leaf, &key(2));
    let chain = f.chain();
    let debit = request(Action::SpendPostage, 60);
    let prepared = prepare_debit(&f.store, &chain, "durable-op", &debit).unwrap();
    f.store.commit_states(prepared.states).unwrap();
    drop(f.store);
    f.store = ProfileStore::open(f.root.path().join("profile.db"), &[55; 32]).unwrap();
    let replay = prepare_debit(&f.store, &chain, "durable-op", &debit).unwrap();
    assert!(replay.already_committed);
    assert!(replay.states.is_empty());
    // A smaller amount is allowed and fits the remaining 10, but cannot relabel this operation.
    let smaller = request(Action::SpendPostage, 1);
    prepare_debit(&f.store, &chain, "new-amount", &smaller).unwrap();
    assert!(matches!(
        prepare_debit(&f.store, &chain, "durable-op", &smaller),
        Err(CapabilityError::IdempotencyConflict)
    ));
    let zero = request(Action::SpendPostage, 0);
    let bound = prepare_debit(&f.store, &chain, "zero-binding", &zero).unwrap();
    f.store.commit_states(bound.states).unwrap();
    let mut other_resource = zero.clone();
    other_resource.resource = "conversation:team".into();
    let mut other_action = zero.clone();
    other_action.action = Action::ReadInbox;
    for (index, changed) in [other_resource, other_action].into_iter().enumerate() {
        prepare_debit(&f.store, &chain, &format!("new-binding-{index}"), &changed).unwrap();
        assert!(matches!(
            prepare_debit(&f.store, &chain, "zero-binding", &changed),
            Err(CapabilityError::IdempotencyConflict)
        ));
    }
    assert!(
        prepare_debit(&f.store, &chain, "zero-binding", &zero)
            .unwrap()
            .already_committed
    );
    assert!(
        prepare_debit(&f.store, &chain, "durable-op", &debit)
            .unwrap()
            .already_committed
    );
    let last = prepare_debit(
        &f.store,
        &chain,
        "new-op",
        &request(Action::SpendPostage, 10),
    )
    .unwrap();
    f.store.commit_states(last.states).unwrap();
    assert!(
        prepare_debit(
            &f.store,
            &chain,
            "over-leaf",
            &request(Action::SpendPostage, 1)
        )
        .is_err()
    );
}

#[test]
fn cancelled_prepare_and_failed_business_transaction_spend_nothing() {
    let mut f = BudgetFixture::new();
    let chain = f.chain();
    let debit = request(Action::SpendPostage, 60);
    let discarded = prepare_debit(&f.store, &chain, "cancelled", &debit).unwrap();
    drop(discarded);
    let mut transaction = prepare_debit(&f.store, &chain, "business", &debit)
        .unwrap()
        .states;
    transaction.push(StateChange {
        namespace: "business-action".into(),
        expected_revision: 0,
        bytes: b"accepted".to_vec(),
    });
    let db = rusqlite::Connection::open(f.root.path().join("profile.db")).unwrap();
    db.execute_batch(&format!("PRAGMA key = \"x'{}'\"; CREATE TRIGGER fail_business BEFORE INSERT ON states WHEN NEW.namespace='business-action' BEGIN SELECT RAISE(ABORT,'injected business failure'); END;",hex::encode([55;32]))).unwrap();
    assert!(f.store.commit_states(transaction).is_err());
    assert!(f.store.state("business-action").unwrap().is_none());
    db.execute_batch("DROP TRIGGER fail_business").unwrap();
    drop(db);
    let mut fresh = prepare_debit(&f.store, &chain, "business", &debit).unwrap();
    assert!(!fresh.already_committed);
    fresh.states.push(StateChange {
        namespace: "business-action".into(),
        expected_revision: 0,
        bytes: b"accepted".to_vec(),
    });
    f.store.commit_states(fresh.states).unwrap();
    drop(f.store);
    f.store = ProfileStore::open(f.root.path().join("profile.db"), &[55; 32]).unwrap();
    assert_eq!(
        f.store.state("business-action").unwrap().unwrap().bytes,
        b"accepted"
    );
    assert!(
        prepare_debit(&f.store, &chain, "business", &debit)
            .unwrap()
            .already_committed
    );
    let last = prepare_debit(
        &f.store,
        &chain,
        "after-rollback",
        &request(Action::SpendPostage, 10),
    )
    .unwrap();
    f.store.commit_states(last.states).unwrap();
    assert!(prepare_debit(&f.store, &chain, "over", &request(Action::SpendPostage, 1)).is_err());
}

#[test]
fn mismatched_profile_wrong_asset_and_zero_scope_do_not_create_budget_state() {
    let f = BudgetFixture::new();
    let chain = f.chain();
    let other = tempfile::tempdir().unwrap();
    let other_store = ProfileStore::open(other.path().join("other.db"), &[56; 32]).unwrap();
    assert!(
        prepare_debit(
            &other_store,
            &chain,
            "wrong-profile",
            &request(Action::SpendPostage, 1)
        )
        .is_err()
    );
    let mut wrong = request(Action::SpendPostage, 1);
    wrong.asset = "other:asset".into();
    assert!(prepare_debit(&f.store, &chain, "wrong-asset", &wrong).is_err());
    let mut c = claims(public(1), public(2));
    c.resources.clear();
    let empty = GrantChain::verify(&[sign(&c, &key(1))], &context(public(1), public(2))).unwrap();
    assert!(empty.authorize(&request(Action::ReadInbox, 0)).is_err());
    let fresh = prepare_debit(
        &f.store,
        &chain,
        "still-full",
        &request(Action::SpendPostage, 70),
    )
    .unwrap();
    assert!(!fresh.already_committed);
}
