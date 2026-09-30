//! Real canonical two-chain registries and public-test registrar keys, no mocked authority.
use super::route::{registrar_fixture as registrar, restored, roster};
use super::*;
const TRANSPORT_FLOORS: &str = "l2/finalizer/transport-floors";

fn check_binding(
    core: &mut AppCore,
    c: &Case,
    wire: &[u8],
    expected: &[u8],
    peer: [u8; 32],
    at: u64,
    compact: bool,
) -> Result<agentic_core::CheckedFinalizerPeer, CoreError> {
    if compact {
        core.verify_finalizer_binding(wire, expected, peer, at)
    } else {
        core.verify_finalizer_peer(
            &agentic_core::FinalizerPresentation {
                roster: roster(c, false),
                binding: format!("0x{}", hex::encode(wire)),
            },
            expected,
            peer,
            at,
        )
    }
}

#[test]
fn binding_floor_rejects_live_replay_after_restart_across_compact_and_full_proofs() {
    for chain in 0..2 {
        for compact in [true, false] {
            let c = Case::new(chain, 4);
            let provider_dir = TempDir::new().unwrap();
            let mut provider = ready(&c, &provider_dir);
            let receiver_dir = TempDir::new().unwrap();
            let mut receiver = local_roster(&c, &receiver_dir);
            let key = public(&c);
            let old_peer = transport(chain);
            let new_peer = transport(10 + chain);
            let third_peer = transport(20 + chain);
            let old = provider
                .serve_finalizer_bindings(old_peer, c.now)
                .unwrap()
                .remove(0);
            check_binding(&mut receiver, &c, &old, &key, old_peer, c.now, compact).unwrap();
            let new = provider
                .serve_finalizer_bindings(new_peer, c.now + 1)
                .unwrap()
                .remove(0);
            let conflicting = provider
                .serve_finalizer_bindings(third_peer, c.now + 1)
                .unwrap()
                .remove(0);
            assert_ne!(new, conflicting);
            let checked =
                check_binding(&mut receiver, &c, &new, &key, new_peer, c.now + 1, compact).unwrap();
            assert_eq!(checked.route().transport_key(), new_peer);
            let saved = rows(&receiver_dir);
            drop(receiver);
            receiver = reopen(&receiver_dir);
            for reader in [true, false] {
                // The old signature is still live; only the persisted observation rejects it.
                agentic_finalizer::routing::verify_transport_hint(
                    &old,
                    hash(&c.e["policy"]["network"]),
                    c.now + 1,
                )
                .unwrap();
                assert!(
                    check_binding(&mut receiver, &c, &old, &key, old_peer, c.now + 1, reader)
                        .is_err(),
                    "a cold receiver accepted a superseded transport"
                );
                assert!(
                    check_binding(
                        &mut receiver,
                        &c,
                        &conflicting,
                        &key,
                        third_peer,
                        c.now + 1,
                        reader
                    )
                    .is_err(),
                    "equal issuance cannot replace a different binding"
                );
                assert_eq!(
                    check_binding(&mut receiver, &c, &new, &key, new_peer, c.now + 1, reader)
                        .unwrap()
                        .route()
                        .transport_key(),
                    new_peer
                );
            }
            assert_eq!(
                rows(&receiver_dir),
                saved,
                "duplicate and replay must not rewrite the floor at the same observed time"
            );
            let latest = provider
                .serve_finalizer_bindings(third_peer, c.now + 2)
                .unwrap()
                .remove(0);
            check_binding(
                &mut receiver,
                &c,
                &latest,
                &key,
                third_peer,
                c.now + 2,
                !compact,
            )
            .unwrap();
            drop(receiver);
            receiver = reopen(&receiver_dir);
            assert!(
                check_binding(&mut receiver, &c, &new, &key, new_peer, c.now + 2, compact).is_err()
            );
            assert_eq!(
                check_binding(
                    &mut receiver,
                    &c,
                    &latest,
                    &key,
                    third_peer,
                    c.now + 2,
                    compact
                )
                .unwrap()
                .route()
                .transport_key(),
                third_peer
            );
        }
    }
}

#[test]
fn unauthenticated_newer_claim_cannot_poison_a_working_binding_floor() {
    let c = Case::new(0, 4);
    let provider_dir = TempDir::new().unwrap();
    let mut provider = ready(&c, &provider_dir);
    let old = provider
        .serve_finalizer_bindings(transport(0), c.now)
        .unwrap()
        .remove(0);
    let new = provider
        .serve_finalizer_bindings(transport(10), c.now + 1)
        .unwrap()
        .remove(0);
    let mut corrupt = new.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    for compact in [true, false] {
        let receiver_dir = TempDir::new().unwrap();
        let mut receiver = local_roster(&c, &receiver_dir);
        let key = public(&c);
        check_binding(&mut receiver, &c, &old, &key, transport(0), c.now, compact).unwrap();
        for (wire, expected, peer) in [
            (corrupt.as_slice(), key.clone(), transport(10)),
            (new.as_slice(), key.clone(), transport(20)),
            (
                new.as_slice(),
                bytes(&c.members()[1]["compressedKey"]),
                transport(10),
            ),
        ] {
            assert!(
                check_binding(&mut receiver, &c, wire, &expected, peer, c.now + 1, compact)
                    .is_err()
            );
            check_binding(
                &mut receiver,
                &c,
                &old,
                &key,
                transport(0),
                c.now + 1,
                !compact,
            )
            .unwrap();
        }
        drop(receiver);
        receiver = reopen(&receiver_dir);
        check_binding(
            &mut receiver,
            &c,
            &old,
            &key,
            transport(0),
            c.now + 1,
            compact,
        )
        .unwrap();
        check_binding(
            &mut receiver,
            &c,
            &new,
            &key,
            transport(10),
            c.now + 1,
            compact,
        )
        .unwrap();
        assert!(
            check_binding(
                &mut receiver,
                &c,
                &old,
                &key,
                transport(0),
                c.now + 1,
                !compact
            )
            .is_err()
        );
    }
}

#[test]
fn binding_floor_survives_a_real_checkpoint_and_roster_renewal_before_cold_reopen() {
    for chain in 0..2 {
        for compact in [true, false] {
            let c = Case::new(chain, 4);
            let provider_dir = TempDir::new().unwrap();
            let mut provider = ready(&c, &provider_dir);
            let receiver_dir = TempDir::new().unwrap();
            let mut receiver = local_roster(&c, &receiver_dir);
            let key = public(&c);
            let old = provider
                .serve_finalizer_bindings(transport(chain), c.now)
                .unwrap()
                .remove(0);
            let latest = provider
                .serve_finalizer_bindings(transport(chain + 10), c.now + 1)
                .unwrap()
                .remove(0);
            check_binding(
                &mut receiver,
                &c,
                &old,
                &key,
                transport(chain),
                c.now,
                compact,
            )
            .unwrap();
            check_binding(
                &mut receiver,
                &c,
                &latest,
                &key,
                transport(chain + 10),
                c.now + 1,
                compact,
            )
            .unwrap();
            c.head(&mut receiver, true);
            let renewed = roster(&c, true);
            assert_ne!(renewed.checkpoint_id, roster(&c, false).checkpoint_id);
            let published = receiver
                .publish_finalizer_roster(&renewed, 1, c.now + 1)
                .unwrap();
            assert_eq!(published.committee_id, c.e["committeeId"]);
            assert_eq!(renewed.epoch, roster(&c, false).epoch);
            drop(receiver);
            receiver = reopen(&receiver_dir);
            let at = c.now + 2;
            agentic_finalizer::routing::verify_transport_hint(
                &old,
                hash(&c.e["policy"]["network"]),
                at,
            )
            .unwrap();
            assert!(
                receiver
                    .verify_finalizer_binding(&old, &key, transport(chain), at)
                    .is_err()
            );
            assert_eq!(
                receiver
                    .verify_finalizer_binding(&latest, &key, transport(chain + 10), at)
                    .unwrap()
                    .route()
                    .transport_key(),
                transport(chain + 10)
            );
            // Both full presentations carry the new proven roster. Refusing the old
            // binding cannot be explained by an obsolete checkpoint in the carrier.
            let old_presentation = agentic_core::FinalizerPresentation {
                roster: renewed.clone(),
                binding: format!("0x{}", hex::encode(&old)),
            };
            let current_presentation = agentic_core::FinalizerPresentation {
                roster: renewed,
                binding: format!("0x{}", hex::encode(&latest)),
            };
            assert!(
                receiver
                    .verify_finalizer_peer(&old_presentation, &key, transport(chain), at)
                    .is_err()
            );
            assert_eq!(
                receiver
                    .verify_finalizer_peer(&current_presentation, &key, transport(chain + 10), at)
                    .unwrap()
                    .route()
                    .transport_key(),
                transport(chain + 10)
            );
        }
    }
}

#[test]
fn binding_floor_and_checkpoint_commit_atomically_before_releasing_verified_transport() {
    for compact in [true, false] {
        for existing in [false, true] {
            for failed_namespace in [TRANSPORT_FLOORS, "l2/checkpoint"] {
                let c = Case::new(0, 4);
                let provider_dir = TempDir::new().unwrap();
                let mut provider = ready(&c, &provider_dir);
                let receiver_dir = TempDir::new().unwrap();
                let mut receiver = local_roster(&c, &receiver_dir);
                let key = public(&c);
                let old = provider
                    .serve_finalizer_bindings(transport(0), c.now)
                    .unwrap()
                    .remove(0);
                let new = provider
                    .serve_finalizer_bindings(transport(10), c.now + 1)
                    .unwrap()
                    .remove(0);
                if existing {
                    check_binding(&mut receiver, &c, &old, &key, transport(0), c.now, compact)
                        .unwrap();
                }
                let saved = rows(&receiver_dir);
                let db = sql(&receiver_dir);
                let verb = if existing || failed_namespace == "l2/checkpoint" {
                    "UPDATE"
                } else {
                    "INSERT"
                };
                db.execute_batch(&format!("CREATE TRIGGER fail_binding BEFORE {verb} ON states WHEN NEW.namespace='{failed_namespace}' BEGIN SELECT RAISE(ABORT,'disk full'); END;")).unwrap();
                assert!(
                    matches!(
                        check_binding(
                            &mut receiver,
                            &c,
                            &new,
                            &key,
                            transport(10),
                            c.now + 1,
                            compact
                        ),
                        Err(CoreError::Store(_))
                    ),
                    "verified transport escaped a failed durable boundary"
                );
                assert_eq!(
                    rows(&receiver_dir),
                    saved,
                    "clock and binding floor must commit in one transaction"
                );
                db.execute_batch("DROP TRIGGER fail_binding;").unwrap();
                drop(receiver);
                receiver = reopen(&receiver_dir);
                // Failed verification did not poison the receiver's version barrier.
                check_binding(
                    &mut receiver,
                    &c,
                    &old,
                    &key,
                    transport(0),
                    c.now + 1,
                    !compact,
                )
                .unwrap();
                check_binding(
                    &mut receiver,
                    &c,
                    &new,
                    &key,
                    transport(10),
                    c.now + 1,
                    compact,
                )
                .unwrap();
                assert!(rows(&receiver_dir).contains_key(TRANSPORT_FLOORS));
                drop(receiver);
                receiver = reopen(&receiver_dir);
                assert!(
                    check_binding(
                        &mut receiver,
                        &c,
                        &old,
                        &key,
                        transport(0),
                        c.now + 1,
                        !compact
                    )
                    .is_err()
                );
                check_binding(
                    &mut receiver,
                    &c,
                    &new,
                    &key,
                    transport(10),
                    c.now + 1,
                    !compact,
                )
                .unwrap();
            }
        }
    }
}

// Independent fixture authoring: shorten only the deadline in a genuine selected
// operator's compact record and sign with its public-test registrar scalar.
fn shorter_binding(provider_dir: &TempDir, original: &[u8], expiry: u64) -> Vec<u8> {
    use p256::ecdsa::{SigningKey, signature::Signer};
    let saved: Value = serde_json::from_slice(&rows(provider_dir)[KEYS_STATE].1).unwrap();
    let seed: Vec<u8> =
        serde_json::from_value(saved["entries"]["selected"]["seed"].clone()).unwrap();
    let mut envelope = minicbor::Decoder::new(original);
    assert_eq!(envelope.array().unwrap(), Some(2));
    let mut d = minicbor::Decoder::new(envelope.bytes().unwrap());
    assert_eq!(d.array().unwrap(), Some(8));
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(8)
        .unwrap()
        .str(d.str().unwrap())
        .unwrap()
        .bytes(d.bytes().unwrap())
        .unwrap()
        .bytes(d.bytes().unwrap())
        .unwrap()
        .u64(d.u64().unwrap())
        .unwrap()
        .bytes(d.bytes().unwrap())
        .unwrap()
        .bytes(d.bytes().unwrap())
        .unwrap()
        .u64(d.u64().unwrap())
        .unwrap();
    assert!(expiry < d.u64().unwrap());
    e.u64(expiry).unwrap();
    let body = e.into_writer();
    let namespace = b"ain-finalizer-route-v1";
    let mut preimage = vec![namespace.len() as u8];
    preimage.extend_from_slice(namespace);
    preimage.extend_from_slice(&body);
    let signature: Signature = SigningKey::from_slice(&seed).unwrap().sign(&preimage);
    let signature = signature.normalize_s();
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(2)
        .unwrap()
        .bytes(&body)
        .unwrap()
        .bytes(&signature.to_bytes())
        .unwrap();
    e.into_writer()
}

#[test]
fn expired_newer_binding_cannot_revive_a_still_live_old_transport_after_restart() {
    let c = Case::new(0, 4);
    let provider_dir = TempDir::new().unwrap();
    let mut provider = ready(&c, &provider_dir);
    let old = provider
        .serve_finalizer_bindings(transport(0), c.now)
        .unwrap()
        .remove(0);
    let new = provider
        .serve_finalizer_bindings(transport(10), c.now + 1)
        .unwrap()
        .remove(0);
    let short = shorter_binding(&provider_dir, &new, c.now + 3);
    let network = hash(&c.e["policy"]["network"]);
    let hint =
        agentic_finalizer::routing::verify_transport_hint(&short, network, c.now + 1).unwrap();
    assert_eq!(hint.expires_at(), c.now + 3);
    for compact in [true, false] {
        let receiver_dir = TempDir::new().unwrap();
        let mut receiver = local_roster(&c, &receiver_dir);
        let key = public(&c);
        check_binding(&mut receiver, &c, &old, &key, transport(0), c.now, compact).unwrap();
        check_binding(
            &mut receiver,
            &c,
            &short,
            &key,
            transport(10),
            c.now + 1,
            !compact,
        )
        .unwrap();
        drop(receiver);
        receiver = reopen(&receiver_dir);
        assert!(
            check_binding(
                &mut receiver,
                &c,
                &short,
                &key,
                transport(10),
                c.now + 4,
                compact
            )
            .is_err()
        );
        agentic_finalizer::routing::verify_transport_hint(&old, network, c.now + 4).unwrap();
        assert!(
            check_binding(
                &mut receiver,
                &c,
                &old,
                &key,
                transport(0),
                c.now + 4,
                compact
            )
            .is_err(),
            "pruning an expired newer binding revived an older live route"
        );
        assert!(matches!(
            check_binding(
                &mut receiver,
                &c,
                &old,
                &key,
                transport(0),
                c.now + 2,
                !compact
            ),
            Err(CoreError::CheckpointClockRollback)
        ));
        let fresh = provider
            .serve_finalizer_bindings(transport(20), c.now + 5)
            .unwrap()
            .remove(0);
        check_binding(
            &mut receiver,
            &c,
            &fresh,
            &key,
            transport(20),
            c.now + 5,
            compact,
        )
        .unwrap();
    }
}
fn transport(chain: usize) -> [u8; 32] {
    ed25519_dalek::SigningKey::from_bytes(&[71 + chain as u8; 32])
        .verifying_key()
        .to_bytes()
}
fn public(c: &Case) -> Vec<u8> {
    bytes(&c.members()[0]["compressedKey"])
}
fn ready(c: &Case, dir: &TempDir) -> AppCore {
    let mut core = c.ready(dir);
    restored(c, dir, true);
    core.publish_finalizer_roster(&roster(c, false), 0, c.now)
        .unwrap();
    core.set_finalizer_operator_enabled("selected", true, 1, c.now)
        .unwrap();
    core
}
fn local_roster(c: &Case, dir: &TempDir) -> AppCore {
    let mut core = c.ready(dir);
    core.publish_finalizer_roster(&roster(c, false), 0, c.now)
        .unwrap();
    core
}
#[test]
fn compact_announcement_lists_only_enabled_local_selected_keys_and_survives_restart() {
    for chain in 0..2 {
        let c = Case::new(chain, 4);
        let dir = TempDir::new().unwrap();
        let mut core = profile(&dir);
        assert!(
            core.serve_finalizer_bindings(transport(chain), c.now)
                .unwrap()
                .is_empty()
        );
        c.trust(&mut core);
        c.install(&mut core, c.now);
        c.head(&mut core, false);
        // Restore three enrolled selected scalars and one genuinely unselected scalar
        // using the existing test registrar. The local roster remains proven, unchanged.
        let selected = c.members();
        let extra = c.v["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| !selected.contains(m))
            .unwrap();
        let mut entries = serde_json::Map::new();
        let db = sql(&dir);
        for (i, member) in selected
            .iter()
            .take(3)
            .chain(std::iter::once(extra))
            .enumerate()
        {
            db.execute("DELETE FROM states WHERE namespace=?1", [KEYS_STATE])
                .unwrap();
            registrar::restore(&db, &c.v, &c.e, member);
            let raw: Vec<u8> = db
                .query_row(
                    "SELECT bytes FROM states WHERE namespace=?1",
                    [KEYS_STATE],
                    |r| r.get(0),
                )
                .unwrap();
            let mut saved: Value = serde_json::from_slice(&raw).unwrap();
            entries.insert(format!("key-{i}"), saved["entries"]["selected"].clone());
            if i == 3 {
                saved["entries"] = Value::Object(entries.clone());
                db.execute(
                    "UPDATE states SET bytes=?1 WHERE namespace=?2",
                    rusqlite::params![serde_json::to_vec(&saved).unwrap(), KEYS_STATE],
                )
                .unwrap();
            }
        }
        // key-2 stays disabled; key-3 is enabled but is not in the selected roster.
        for (i, revision) in [(0, 1), (1, 2), (3, 3)] {
            core.set_finalizer_operator_enabled(&format!("key-{i}"), true, revision, c.now)
                .unwrap();
        }
        assert!(
            core.serve_finalizer_bindings(transport(chain), c.now)
                .unwrap()
                .is_empty()
        );
        core.publish_finalizer_roster(&roster(&c, false), 0, c.now)
            .unwrap();
        let wires = core
            .serve_finalizer_bindings(transport(chain), c.now)
            .unwrap();
        assert_eq!(wires.len(), 2);
        assert!(wires.iter().all(|w| w.len() <= 512));
        let consumer_dir = TempDir::new().unwrap();
        let mut consumer = local_roster(&c, &consumer_dir);
        assert!(consumer.finalizer_operators().unwrap().is_empty());
        let mut actual = BTreeSet::new();
        for w in &wires {
            let hint = agentic_finalizer::routing::verify_transport_hint(
                w,
                hash(&c.e["policy"]["network"]),
                c.now,
            )
            .unwrap();
            let checked = consumer
                .verify_finalizer_binding(w, hint.public_key(), transport(chain), c.now)
                .unwrap();
            actual.insert(checked.route().public_key().to_vec());
            assert_eq!(
                checked.committee().committee().id(),
                hash(&c.e["committeeId"])
            );
            assert_eq!(checked.committee().committee().quorum(), 3);
        }
        assert_eq!(
            actual,
            selected
                .iter()
                .take(2)
                .map(|m| bytes(&m["compressedKey"]))
                .collect()
        );
        let references: Vec<Value> = serde_json::from_str(include_str!(
            "../../../finalizer/tests/fixtures/p256-route.json"
        ))
        .unwrap();
        assert!(
            wires.contains(&bytes(&references[chain]["wire"])),
            "existing independent OpenSSL wire stays byte-identical"
        );
        drop(core);
        drop(consumer);
        let mut core = reopen(&dir);
        let mut consumer = reopen(&consumer_dir);
        assert_eq!(
            core.serve_finalizer_bindings(transport(chain), c.now)
                .unwrap(),
            wires
        );
        consumer
            .verify_finalizer_binding(
                &bytes(&references[chain]["wire"]),
                &public(&c),
                transport(chain),
                c.now,
            )
            .unwrap();
        core.set_finalizer_operator_enabled("key-0", false, 4, c.now)
            .unwrap();
        let remaining = core
            .serve_finalizer_bindings(transport(chain), c.now)
            .unwrap();
        assert_eq!(remaining.len(), 1);
        let hint = agentic_finalizer::routing::verify_transport_hint(
            &remaining[0],
            hash(&c.e["policy"]["network"]),
            c.now,
        )
        .unwrap();
        assert_eq!(hint.public_key(), bytes(&selected[1]["compressedKey"]));
    }
}
#[test]
fn compact_binding_requires_local_verified_roster_expected_key_and_actual_transport() {
    for chain in 0..2 {
        let c = Case::new(chain, 4);
        let dir = TempDir::new().unwrap();
        let mut provider = ready(&c, &dir);
        let good = provider
            .serve_finalizer_bindings(transport(chain), c.now)
            .unwrap()
            .remove(0);
        let other_dir = TempDir::new().unwrap();
        let mut other = c.ready(&other_dir);
        assert!(
            other
                .verify_finalizer_binding(&good, &public(&c), transport(chain), c.now)
                .is_err()
        );
        other
            .publish_finalizer_roster(&roster(&c, false), 0, c.now)
            .unwrap();
        assert!(
            other
                .verify_finalizer_binding(
                    &good,
                    &bytes(&c.members()[1]["compressedKey"]),
                    transport(chain),
                    c.now
                )
                .is_err()
        );
        let other_transport = ed25519_dalek::SigningKey::from_bytes(&[99; 32])
            .verifying_key()
            .to_bytes();
        assert!(
            other
                .verify_finalizer_binding(&good, &public(&c), other_transport, c.now)
                .is_err()
        );
        let mut bad = good.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(
            other
                .verify_finalizer_binding(&bad, &public(&c), transport(chain), c.now)
                .is_err()
        );
        let different = Case::new(chain, 7);
        let different_dir = TempDir::new().unwrap();
        let mut wrong_committee = local_roster(&different, &different_dir);
        assert!(
            wrong_committee
                .verify_finalizer_binding(&good, &public(&c), transport(chain), c.now)
                .is_err()
        );
        let checked = other
            .verify_finalizer_binding(&good, &public(&c), transport(chain), c.now)
            .unwrap();
        assert_eq!(
            checked.route().expires_at(),
            c.v["current"]["expiresAt"].as_u64().unwrap()
        );
        assert!(
            other
                .serve_finalizer_bindings(transport(chain), c.now)
                .unwrap()
                .is_empty()
        );
    }
}
#[test]
fn refreshed_local_roster_reuses_short_binding_but_never_extends_it_or_revives_after_clock_rollback()
 {
    let c = Case::new(0, 4);
    let dir = TempDir::new().unwrap();
    let mut core = ready(&c, &dir);
    let old = core
        .serve_finalizer_bindings(transport(0), c.now)
        .unwrap()
        .remove(0);
    c.head(&mut core, true);
    let time = c.v["renewed"]["issuedAt"].as_u64().unwrap();
    assert!(
        core.serve_finalizer_bindings(transport(0), time)
            .unwrap()
            .is_empty()
    );
    assert!(
        core.verify_finalizer_binding(&old, &public(&c), transport(0), time)
            .is_err()
    );
    core.publish_finalizer_roster(&roster(&c, true), 1, time)
        .unwrap();
    let old_expiry = c.v["current"]["expiresAt"].as_u64().unwrap();
    assert_eq!(
        core.verify_finalizer_binding(&old, &public(&c), transport(0), time)
            .unwrap()
            .route()
            .expires_at(),
        old_expiry
    );
    let fresh = core
        .serve_finalizer_bindings(transport(0), time)
        .unwrap()
        .remove(0);
    assert_eq!(
        core.verify_finalizer_binding(&fresh, &public(&c), transport(0), time)
            .unwrap()
            .route()
            .expires_at(),
        time + 60
    );
    assert!(
        core.verify_finalizer_binding(&old, &public(&c), transport(0), old_expiry)
            .is_err()
    );
    assert!(
        core.verify_finalizer_binding(&fresh, &public(&c), transport(0), old_expiry)
            .is_ok()
    );
    drop(core);
    let mut core = reopen(&dir);
    assert!(matches!(
        core.serve_finalizer_bindings(transport(0), time),
        Err(CoreError::CheckpointClockRollback)
    ));
    assert!(matches!(
        core.verify_finalizer_binding(&old, &public(&c), transport(0), time),
        Err(CoreError::CheckpointClockRollback)
    ));
    let expiry = c.v["renewed"]["expiresAt"].as_u64().unwrap();
    assert!(
        core.serve_finalizer_bindings(transport(0), expiry)
            .unwrap()
            .is_empty()
    );
    assert!(
        core.verify_finalizer_binding(&fresh, &public(&c), transport(0), expiry)
            .is_err()
    );
}
#[test]
fn compact_announcement_and_checked_route_do_not_escape_failed_durable_clock_commit() {
    let c = Case::new(0, 4);
    let dir = TempDir::new().unwrap();
    let mut core = ready(&c, &dir);
    let binding = core
        .serve_finalizer_bindings(transport(0), c.now)
        .unwrap()
        .remove(0);
    let original = rows(&dir);
    let db = sql(&dir);
    db.execute_batch("CREATE TRIGGER fail BEFORE UPDATE ON states WHEN NEW.namespace='l2/checkpoint' BEGIN SELECT RAISE(ABORT,'disk full'); END;").unwrap();
    assert!(matches!(
        core.serve_finalizer_bindings(transport(0), c.now + 1),
        Err(CoreError::Store(_))
    ));
    assert!(matches!(
        core.verify_finalizer_binding(&binding, &public(&c), transport(0), c.now + 1),
        Err(CoreError::Store(_))
    ));
    assert_eq!(rows(&dir), original);
    db.execute_batch("DROP TRIGGER fail;").unwrap();
    assert_eq!(
        core.serve_finalizer_bindings(transport(0), c.now + 1)
            .unwrap()
            .len(),
        1
    );
    assert!(
        core.verify_finalizer_binding(&binding, &public(&c), transport(0), c.now + 1)
            .is_ok()
    );
    let original: Vec<u8> = db
        .query_row(
            "SELECT bytes FROM states WHERE namespace='l2/finalizer/roster'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut corrupt: Value = serde_json::from_slice(&original).unwrap();
    let byte = corrupt["roster"]["members"][0]["qx"][0].as_u64().unwrap();
    corrupt["roster"]["members"][0]["qx"][0] = json!(byte ^ 1);
    db.execute(
        "UPDATE states SET bytes=?1 WHERE namespace='l2/finalizer/roster'",
        [serde_json::to_vec(&corrupt).unwrap()],
    )
    .unwrap();
    assert!(
        core.serve_finalizer_bindings(transport(0), c.now + 1)
            .is_err()
    );
    assert!(
        core.verify_finalizer_binding(&binding, &public(&c), transport(0), c.now + 1)
            .is_err()
    );
    db.execute(
        "UPDATE states SET bytes=?1 WHERE namespace='l2/finalizer/roster'",
        [original],
    )
    .unwrap();
    assert_eq!(
        core.serve_finalizer_bindings(transport(0), c.now + 1)
            .unwrap()
            .len(),
        1
    );
    assert!(
        core.verify_finalizer_binding(&binding, &public(&c), transport(0), c.now + 1)
            .is_ok()
    );
}
