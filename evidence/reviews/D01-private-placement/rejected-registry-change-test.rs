//! A fixed private ticket selects stable storage positions without exposing funding.
//! Uses genuine retained receipts and actual EIP-1186 registry membership evidence.
#![allow(clippy::unwrap_used)]
mod support;
use agentic_core::AppCore;
use agentic_l2_adapter::AssignmentMemberInput;
use agentic_postage_spend::{SpendCandidate, prepare_client};
use serde_json::Value;
use support::*;
use tempfile::TempDir;

fn metadata() -> Value {
    serde_json::from_str(include_str!("fixtures/private-custody.json")).unwrap()
}
fn candidate(core: &mut AppCore, v: &Value, which: usize) -> SpendCandidate {
    candidate_from_receipt(core, v, which, receipt(which))
}
fn candidate_from_receipt(
    core: &mut AppCore,
    v: &Value,
    which: usize,
    wire: &[u8],
) -> SpendCandidate {
    let now = v["checkedAt"].as_u64().unwrap();
    let permission = core
        .prepare_postage_client(
            hash(&v["finalizer"]["current"]["checkpointId"]),
            &raw(&v["finalizer"]["issuerProof"]),
            now,
        )
        .unwrap()
        .unwrap();
    let checked = context(core, v, which);
    prepare_client(&permission, core, checked, wire, now).unwrap()
}
fn ordinals(value: &Value) -> Vec<u64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect()
}

#[test]
fn genuine_competing_operations_share_private_placement_and_verify_actual_gapped_members() {
    let v = fixture();
    let m = metadata();
    let now = v["checkedAt"].as_u64().unwrap();
    let dir = TempDir::new().unwrap();
    let mut core = ready_client(&dir, &v);
    let a = candidate(&mut core, &v, 0);
    let b = candidate(&mut core, &v, 1);
    assert_ne!(a.operation(), b.operation());
    assert_eq!(a.nullifier(), b.nullifier());
    let expected = &m["expected"];
    let primary = ordinals(&expected["primaryOrdinals"]);
    let replacements = ordinals(&expected["replacementOrdinals"]);
    for candidate in [&a, &b] {
        let placement = candidate.custody_placement(&mut core, now).unwrap();
        assert_eq!(placement.assignment_id(), hash(&expected["assignmentId"]));
        assert_eq!(placement.primary_ordinals(), primary);
        assert_eq!(placement.replacement_ordinals(), replacements);
        assert_eq!(candidate.resources().target_replicas, 10);
        assert_eq!(candidate.resources().repair_allowance, 4);
        assert_eq!(
            placement.valid_until(),
            v["finalizer"]["current"]["expiresAt"].as_u64().unwrap()
        );
        assert_eq!(primary.len(), 10);
        assert_eq!(replacements.len(), 4);
        for (position, ordinal) in primary.iter().chain(&replacements).enumerate() {
            let member = m["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|member| member["ordinal"].as_u64() == Some(*ordinal))
                .unwrap();
            let proof = raw(&member["proof"]);
            let checked = placement
                .verify_member(
                    &mut core,
                    AssignmentMemberInput {
                        position: position as u16,
                        expected_checkpoint_id: hash(&v["finalizer"]["current"]["checkpointId"])
                            .into(),
                        proof: &proof,
                        node_key: hash(&member["nodeKey"]).into(),
                        salt: hash(&member["salt"]).into(),
                    },
                    now,
                )
                .unwrap();
            assert_eq!(checked.ordinal(), *ordinal);
            assert_eq!(
                u64::from(checked.index()),
                member["proof"]["index"].as_u64().unwrap()
            );
        }
    }
    assert!(core.custody_intents().unwrap().is_empty());
    assert!(core.finalizer_operators().unwrap().is_empty());
}

#[test]
fn actual_membership_at_another_slot_or_head_and_changed_opening_are_not_selected() {
    let v = fixture();
    let m = metadata();
    let now = v["checkedAt"].as_u64().unwrap();
    let dir = TempDir::new().unwrap();
    let mut core = ready_client(&dir, &v);
    let candidate = candidate(&mut core, &v, 0);
    let placement = candidate.custody_placement(&mut core, now).unwrap();
    let first = m["expected"]["primaryOrdinals"][0].as_u64().unwrap();
    let member = m["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["ordinal"].as_u64() == Some(first))
        .unwrap();
    let proof = raw(&member["proof"]);
    let head = hash(&v["finalizer"]["current"]["checkpointId"]);
    let key = hash(&member["nodeKey"]);
    let salt = hash(&member["salt"]);
    let verify =
        |core: &mut AppCore, position, expected: [u8; 32], key: [u8; 32], salt: [u8; 32]| {
            placement.verify_member(
                core,
                AssignmentMemberInput {
                    position,
                    expected_checkpoint_id: expected.into(),
                    proof: &proof,
                    node_key: key.into(),
                    salt: salt.into(),
                },
                now,
            )
        };
    assert!(verify(&mut core, 0, head, key, salt).is_ok());
    assert!(verify(&mut core, 1, head, key, salt).is_err());
    assert!(verify(&mut core, 14, head, key, salt).is_err());
    assert!(verify(&mut core, 0, [9; 32], key, salt).is_err());
    let mut changed = salt;
    changed[0] ^= 1;
    assert!(verify(&mut core, 0, head, key, changed).is_err());
    assert!(verify(&mut core, 0, head, key, salt).is_ok());
}

#[test]
fn expired_or_reconfigured_context_cannot_be_reused_for_private_placement() {
    let v = fixture();
    let now = v["checkedAt"].as_u64().unwrap();
    let dir = TempDir::new().unwrap();
    let mut core = ready_client(&dir, &v);
    let first = candidate(&mut core, &v, 0);
    let placement = first.custody_placement(&mut core, now).unwrap();
    let deadline = placement.valid_until();
    let m = metadata();
    let ordinal = placement.primary_ordinals()[0];
    let member = m["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["ordinal"].as_u64() == Some(ordinal))
        .unwrap();
    let proof = raw(&member["proof"]);
    let input = || AssignmentMemberInput {
        position: 0,
        expected_checkpoint_id: hash(&v["finalizer"]["current"]["checkpointId"]).into(),
        proof: &proof,
        node_key: hash(&member["nodeKey"]).into(),
        salt: hash(&member["salt"]).into(),
    };
    assert!(placement.verify_member(&mut core, input(), now).is_ok());
    assert!(
        placement
            .verify_member(&mut core, input(), deadline)
            .is_err()
    );
    assert!(placement.verify_member(&mut core, input(), now).is_err());
    assert!(first.custody_placement(&mut core, deadline).is_err());
    assert!(
        first.custody_placement(&mut core, now).is_err(),
        "clock rollback revived an expired context"
    );
    let dir = TempDir::new().unwrap();
    let mut core = ready_client(&dir, &v);
    let first = candidate(&mut core, &v, 0);
    let placement = first.custody_placement(&mut core, now).unwrap();
    assert!(placement.verify_member(&mut core, input(), now).is_ok());
    let mut registry = v["registryProfile"].clone();
    registry["codeHash"] = serde_json::json!(format!("0x{}", "43".repeat(32)));
    core.install_registry_profile(&raw(&registry), 1, now)
        .unwrap();
    assert!(placement.verify_member(&mut core, input(), now).is_err());
    assert!(first.custody_placement(&mut core, now).is_err());
}

#[test]
fn genuine_paid_classes_enforce_population_and_obligation_limits_without_fake_candidates() {
    let v: Value = serde_json::from_str(include_str!("fixtures/private-classes.json")).unwrap();
    let receipts: [&[u8]; 3] = [
        include_bytes!("fixtures/private-class-2.json"),
        include_bytes!("fixtures/private-class-3.json"),
        include_bytes!("fixtures/private-class-4.json"),
    ];
    let now = v["checkedAt"].as_u64().unwrap();
    let count = v["snapshot"]["count"].as_u64().unwrap();
    let max_duration = v["registryProfile"]["maxObligationSeconds"]
        .as_u64()
        .unwrap();
    assert_eq!((count, max_duration), (16, 3600));
    let dir = TempDir::new().unwrap();
    let mut core = ready_client(&dir, &v);
    for (index, wire) in receipts.iter().enumerate() {
        // Every candidate must first pass ordinary Core/context and genuine ZK
        // verification. A refusal here cannot substitute for placement refusal.
        let candidate = candidate_from_receipt(&mut core, &v, index, wire);
        assert_eq!(candidate.nullifier(), hash(&v["cases"][index]["nullifier"]));
        assert_eq!(candidate.journal(), bytes(&v["cases"][index]["journal"]));
        let r = candidate.resources();
        match index {
            0 => {
                assert_eq!(
                    (
                        r.class_id,
                        r.target_replicas,
                        r.repair_allowance,
                        r.retention_seconds
                    ),
                    (2, 16, 4, 60)
                );
                let placement = candidate.custody_placement(&mut core, now).unwrap();
                assert_eq!(placement.primary_ordinals().len(), 16);
                assert!(placement.replacement_ordinals().is_empty());
                let actual: std::collections::BTreeSet<_> =
                    placement.primary_ordinals().iter().copied().collect();
                assert_eq!(actual, (0..count).collect());
                for (position, ordinal) in placement.primary_ordinals().iter().enumerate() {
                    let member = v["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|m| m["ordinal"].as_u64() == Some(*ordinal))
                        .unwrap();
                    let proof = raw(&member["proof"]);
                    let checked = placement
                        .verify_member(
                            &mut core,
                            AssignmentMemberInput {
                                position: position as u16,
                                expected_checkpoint_id: hash(
                                    &v["finalizer"]["current"]["checkpointId"],
                                )
                                .into(),
                                proof: &proof,
                                node_key: hash(&member["nodeKey"]).into(),
                                salt: hash(&member["salt"]).into(),
                            },
                            now,
                        )
                        .unwrap();
                    assert_eq!(checked.ordinal(), *ordinal);
                }
            }
            1 => {
                assert_eq!(
                    (r.class_id, r.target_replicas, r.retention_seconds),
                    (3, 32, 60)
                );
                assert!(u64::from(r.target_replicas) > count);
                assert!(u64::from(r.retention_seconds) <= max_duration);
                assert!(
                    candidate.custody_placement(&mut core, now).is_err(),
                    "undersized population accepted"
                );
            }
            2 => {
                assert_eq!(
                    (r.class_id, r.target_replicas, r.retention_seconds),
                    (4, 3, 7200)
                );
                assert!(u64::from(r.target_replicas) <= count);
                assert!(u64::from(r.retention_seconds) > max_duration);
                assert!(
                    candidate.custody_placement(&mut core, now).is_err(),
                    "excessive storage obligation accepted"
                );
            }
            _ => unreachable!(),
        }
    }
    // Refusals do not poison the valid class or create owned funding/operator state.
    assert!(
        candidate_from_receipt(&mut core, &v, 0, receipts[0])
            .custody_placement(&mut core, now)
            .is_ok()
    );
    assert!(core.custody_intents().unwrap().is_empty());
    assert!(core.finalizer_operators().unwrap().is_empty());
}

#[test]
fn accepted_successor_revokes_an_existing_placement_before_its_original_deadline() {
    let v = fixture();
    let m = metadata();
    let now = v["checkedAt"].as_u64().unwrap();
    let dir = TempDir::new().unwrap();
    let mut core = ready_client(&dir, &v);
    let candidate = candidate(&mut core, &v, 0);
    let placement = candidate.custody_placement(&mut core, now).unwrap();
    let first = placement.primary_ordinals()[0];
    let member = m["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["ordinal"].as_u64() == Some(first))
        .unwrap();
    let proof = raw(&member["proof"]);
    let input = || AssignmentMemberInput {
        position: 0,
        expected_checkpoint_id: hash(&v["finalizer"]["current"]["checkpointId"]).into(),
        proof: &proof,
        node_key: hash(&member["nodeKey"]).into(),
        salt: hash(&member["salt"]).into(),
    };
    assert!(placement.verify_member(&mut core, input(), now).is_ok());
    let head = &v["laterFinalizer"]["current"];
    let later = head["issuedAt"].as_u64().unwrap();
    assert!(later > now && later < placement.valid_until());
    let revision = core.checkpoint_status(now).unwrap().unwrap().revision;
    core.accept_checkpoint(&bytes(&head["certificate"]), revision, later)
        .unwrap();
    assert_eq!(
        core.checkpoint_sync_anchor()
            .unwrap()
            .unwrap()
            .checkpoint_id,
        Some(hash(&head["checkpointId"]))
    );
    assert!(placement.verify_member(&mut core, input(), later).is_err());
    assert!(candidate.custody_placement(&mut core, later).is_err());
}
