use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn key_package_record(records: &BTreeMap<Vec<u8>, Vec<u8>>, wire: &[u8]) -> Vec<u8> {
    // Identify the stored bundle by its actual public TLS KeyPackage, not by
    // whichever record the join delta happens to delete.
    let matching: Vec<_> = records
        .iter()
        .filter(|(key, _)| key.starts_with(b"KeyPackage"))
        .filter(|(key, bytes)| {
            let mut decoder = minicbor::Decoder::new(bytes);
            assert_eq!(decoder.array().unwrap(), Some(2));
            assert_eq!(decoder.bytes().unwrap(), key.as_slice());
            let bundle: KeyPackageBundle =
                serde_json::from_slice(decoder.bytes().unwrap()).unwrap();
            assert_eq!(decoder.position(), bytes.len());
            bundle.key_package().tls_serialize_detached().unwrap() == wire
        })
        .map(|(key, _)| key.clone())
        .collect();
    assert_eq!(matching.len(), 1, "exact KeyPackage bundle must exist");
    matching[0].clone()
}

#[test]
fn staged_record_delta_restores_exact_state_and_removes_consumed_key_package() {
    let alice = MlsClient::new(b"alice-records").unwrap();
    let bob = MlsClient::new(b"bob-records").unwrap();
    let unused = bob.key_package().unwrap();
    let bob = MlsClient::from_state(unused.next_state);
    let package = bob.key_package().unwrap();
    let bob = MlsClient::from_state(package.next_state);
    let created = alice.create_group(GROUP).unwrap();
    let alice = MlsClient::from_state(created.next_state);
    let added = alice
        .add_members(GROUP, std::slice::from_ref(&package.value))
        .unwrap();
    let pending = MlsClient::from_state(added.next_state);
    let activated = pending.activate_pending_commit(GROUP).unwrap();
    let alice = MlsClient::from_state(activated.next_state);
    let before = bob.snapshot().as_bytes().to_vec();
    let mut records: BTreeMap<Vec<u8>, Vec<u8>> = bob
        .snapshot()
        .record_changes(None)
        .map(|(key, bytes)| (key.to_vec(), bytes.unwrap().to_vec()))
        .collect();
    let consumed_key = key_package_record(&records, &package.value);
    let unused_key = key_package_record(&records, &unused.value);
    assert_ne!(consumed_key, unused_key);
    let unused_bytes = records[&unused_key].clone();
    let joined = bob.join(GROUP, &added.value.welcome).unwrap();
    let changes: Vec<_> = joined
        .next_state
        .record_changes(Some(bob.snapshot()))
        .collect();
    assert_eq!(
        changes
            .iter()
            .filter(|(key, bytes)| key.starts_with(b"KeyPackage") && bytes.is_none())
            .map(|(key, _)| key.to_vec())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([consumed_key.clone()]),
        "only the KeyPackage supplied to add_members may be deleted"
    );
    for (key, bytes) in changes {
        if let Some(bytes) = bytes {
            records.insert(key.to_vec(), bytes.to_vec());
        } else {
            assert!(records.remove(key).is_some());
        }
    }
    assert!(!records.contains_key(&consumed_key));
    assert_eq!(records.get(&unused_key), Some(&unused_bytes));
    assert_eq!(bob.snapshot().as_bytes(), before);
    let restored = MlsClient::restore_records(joined.next_state.record_prefix(), records).unwrap();
    assert_eq!(restored.snapshot().as_bytes(), joined.next_state.as_bytes());
    let sent = alice.encrypt(GROUP, TEXT, AAD).unwrap();
    assert_eq!(
        restored
            .decrypt(GROUP, &sent.value.wire, AAD)
            .unwrap()
            .value
            .plaintext,
        TEXT
    );
    let stats = sent.next_state.persistence_stats(Some(alice.snapshot()));
    assert!(stats.changed_records > 0);
    assert!(stats.changed_records < stats.total_records);
    assert!(stats.written_bytes < stats.snapshot_bytes);
}
