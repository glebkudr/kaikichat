use super::*;

fn rows(root: &TempDir) -> Vec<(String, i64, Vec<u8>)> {
    let db = raw_encrypted_db(&root.path().join("profile.db"));
    let mut query = db
        .prepare("SELECT namespace,revision,bytes FROM states ORDER BY namespace")
        .unwrap();
    query
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn save(store: &mut ProfileStore, names: &[String]) {
    for chunk in names.chunks(16) {
        store
            .commit_states(
                chunk
                    .iter()
                    .map(|namespace| StateChange {
                        namespace: namespace.clone(),
                        expected_revision: 0,
                        bytes: [SECRET, namespace.as_bytes()].concat(),
                    })
                    .collect(),
            )
            .unwrap();
    }
}

#[test]
fn state_range_pages_resume_after_reopen_without_skipping_equal_deadlines_or_reading_future_work() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let prefix = "custody/outgoing-expiry/";
    let names: Vec<_> = (0..257)
        .map(|i| format!("{prefix}{:020}/{i:064x}", 1000 + i / 9))
        .collect();
    save(&mut store, &names);
    let through = names.last().unwrap().clone();
    let first = store
        .state_namespaces_between(prefix, &through, 19)
        .unwrap();
    assert_eq!(first, names[..19]);
    let after = first.last().unwrap().clone();
    let mut found = first;
    drop(store);

    let mut store = open(&root);
    // This API is a live ordered range, not a database snapshot. The caller's
    // persisted deadline/upper key keeps newly scheduled later work out of it.
    let future = format!("{prefix}{:020}/{:064x}", 2000, 257);
    let neighbors = vec![
        future.clone(),
        "custody/incoming-expiry/00000000000000001000/0".into(),
        "custody/outgoing-expiryx/00000000000000001000/0".into(),
    ];
    save(&mut store, &neighbors);
    let saved = rows(&root);
    let changes = store.change_count();
    let mut cursor = after;
    loop {
        let page = store
            .state_namespaces_between(&cursor, &through, 19)
            .unwrap();
        assert!(page.len() <= 19);
        if page.is_empty() {
            break;
        }
        assert!(page.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(page.first().unwrap() > &cursor);
        cursor = page.last().unwrap().clone();
        found.extend(page);
        assert!(found.len() <= names.len(), "continuation did not advance");
    }
    assert_eq!(found, names);
    assert_eq!(cursor, through);
    assert_eq!(store.change_count(), changes);
    assert_eq!(rows(&root), saved);
    assert_eq!(
        store
            .state_namespaces_between(&through, &future, 1)
            .unwrap(),
        vec![future.clone()]
    );
    for name in names.iter().chain(&neighbors) {
        let value = store.state(name).unwrap().unwrap();
        assert_eq!(value.revision, 1);
        assert_eq!(value.bytes, [SECRET, name.as_bytes()].concat());
    }
}

#[test]
fn state_range_is_exclusive_inclusive_literal_and_bounded_even_when_the_cursor_key_is_absent() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    let names: Vec<String> = [
        "custody/expiry/0001/a",
        "custody/expiry/0001/b",
        "custody/expiry/0001/c",
        "custody/expiry/0002/a",
        "custody/expiry/%_!/a",
        "custody/expiry/%_!/b",
        "custody/expiry/XYZ/a",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    save(&mut store, &names);
    let saved = rows(&root);
    assert_eq!(
        store
            .state_namespaces_between(&names[0], &names[3], 64)
            .unwrap(),
        names[1..4]
    );
    assert_eq!(
        store
            .state_namespaces_between("custody/expiry/0001/bb", &names[3], 64)
            .unwrap(),
        names[2..4]
    );
    assert_eq!(
        store
            .state_namespaces_between(&names[0], &names[3], 1)
            .unwrap(),
        names[1..2]
    );
    assert_eq!(
        store
            .state_namespaces_between("custody/expiry/%_!/", &names[5], 64)
            .unwrap(),
        names[4..6]
    );
    assert!(
        store
            .state_namespaces_between(&names[2], &names[2], 64)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .state_namespaces_between("absent/a", "absent/z", 64)
            .unwrap()
            .is_empty()
    );
    assert_eq!(rows(&root), saved);
}

#[test]
fn state_range_rejects_unbounded_or_reversed_requests_without_changing_encrypted_state() {
    let root = TempDir::new().unwrap();
    let mut store = open(&root);
    save(&mut store, &["custody/expiry/0001/a".into()]);
    let saved = rows(&root);
    let too_long = "x".repeat(257);
    for (after, through, limit) in [
        ("", "z", 1),
        ("a", "", 1),
        ("z", "a", 1),
        ("a", "z", 0),
        ("a", "z", 65),
        ("a\n", "z", 1),
        ("a", "z\0", 1),
        (too_long.as_str(), "z", 1),
        ("a", too_long.as_str(), 1),
    ] {
        assert!(matches!(
            store.state_namespaces_between(after, through, limit),
            Err(StoreError::InvalidInput)
        ));
    }
    assert_eq!(rows(&root), saved);
    drop(store);
    assert!(ProfileStore::open(root.path().join("profile.db"), &[0x24; 32]).is_err());
    let store = open(&root);
    assert_eq!(
        store
            .state_namespaces_between("custody/expiry/", "custody/expiry/z", 64)
            .unwrap(),
        vec!["custody/expiry/0001/a"]
    );
    assert_eq!(rows(&root), saved);
}
