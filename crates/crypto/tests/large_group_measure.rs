#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Sizes and timings of an MLS group of 2000 (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
//! phase 1). A measurement, not a check: run with
//! `cargo test --release -p agentic-crypto --test large_group_measure -- --ignored --nocapture`.
use agentic_crypto::{MlsClient, Prepared};
use std::time::Instant;

const GROUP: [u8; 32] = [0x47; 32];
const MEMBERS: usize = 2000;

fn accept<T>(client: &mut MlsClient, prepared: Prepared<T>) -> T {
    *client = MlsClient::from_state(prepared.next_state);
    prepared.value
}

fn id(i: usize) -> Vec<u8> {
    format!("ain1{:064x}", i).into_bytes()
}

#[test]
#[ignore = "measurement"]
fn measure_a_group_of_2000() {
    let started = Instant::now();
    let mut clients: Vec<MlsClient> = (0..MEMBERS)
        .map(|i| MlsClient::new(&id(i)).unwrap())
        .collect();
    let packages: Vec<Vec<u8>> = clients
        .iter_mut()
        .skip(1)
        .map(|c| {
            let p = c.last_resort_key_package().unwrap();
            accept(c, p)
        })
        .collect();
    println!(
        "clients and packages: {:?}, package {} B",
        started.elapsed(),
        packages[0].len()
    );
    let mut admin = clients.remove(0);
    let p = admin.create_group(GROUP).unwrap();
    accept(&mut admin, p);

    // Fill in batches of 1000, 998 and one of 1, keeping the last welcome.
    let mut welcome_one = vec![];
    for (n, batch) in [&packages[..1000], &packages[1000..1998], &packages[1998..]]
        .into_iter()
        .enumerate()
    {
        let t = Instant::now();
        let p = admin.commit_changes(GROUP, batch, &[], b"", None).unwrap();
        let made = accept(&mut admin, p);
        let p = admin.activate_pending_commit(GROUP).unwrap();
        accept(&mut admin, p);
        let welcome = made.welcome.unwrap();
        println!(
            "add batch {n} of {}: commit {} B, welcome {} B, {:?}",
            batch.len(),
            made.commit.len(),
            welcome.len(),
            t.elapsed()
        );
        if batch.len() == 1 {
            welcome_one = welcome;
        }
    }
    println!(
        "admin state at {MEMBERS}: {} B",
        admin.snapshot().as_bytes().len()
    );

    // The last invitee joins with the tree inside its Welcome.
    let mut member = clients.pop().unwrap();
    let t = Instant::now();
    let p = member.join(GROUP, &welcome_one).unwrap();
    accept(&mut member, p);
    println!(
        "join: {:?}, member state {} B",
        t.elapsed(),
        member.snapshot().as_bytes().len()
    );

    // A message each way.
    let t = Instant::now();
    let p = admin.encrypt(GROUP, b"hello", b"").unwrap();
    let wire = accept(&mut admin, p).wire;
    let encrypt = t.elapsed();
    let t = Instant::now();
    let p = member.decrypt(GROUP, &wire, b"").unwrap();
    accept(&mut member, p);
    println!(
        "message {} B: encrypt {encrypt:?}, decrypt {:?}",
        wire.len(),
        t.elapsed()
    );

    // One removal, then 30 %, then one more in the holed tree.
    let removals: [Vec<Vec<u8>>; 3] = [vec![id(1)], (2..602).map(id).collect(), vec![id(700)]];
    for (n, remove) in removals.iter().enumerate() {
        let t = Instant::now();
        let p = admin.commit_changes(GROUP, &[], remove, b"", None).unwrap();
        let made = accept(&mut admin, p);
        let p = admin.activate_pending_commit(GROUP).unwrap();
        accept(&mut admin, p);
        let commit = t.elapsed();
        let t = Instant::now();
        let p = member.apply_commit(GROUP, &made.commit).unwrap();
        accept(&mut member, p);
        println!(
            "remove {n} ({}): commit {} B, make {commit:?}, apply {:?}",
            remove.len(),
            made.commit.len(),
            t.elapsed()
        );
    }

    // Adding one more to the holed tree: its Welcome carries the tree.
    let mut late = MlsClient::new(&id(MEMBERS)).unwrap();
    let p = late.last_resort_key_package().unwrap();
    let package = accept(&mut late, p);
    let p = admin
        .commit_changes(GROUP, &[package], &[], b"", None)
        .unwrap();
    let made = accept(&mut admin, p);
    let p = admin.activate_pending_commit(GROUP).unwrap();
    accept(&mut admin, p);
    println!(
        "add to holed tree: commit {} B, welcome {} B",
        made.commit.len(),
        made.welcome.unwrap().len()
    );
    println!(
        "states after: admin {} B, member {} B; total {:?}",
        admin.snapshot().as_bytes().len(),
        member.snapshot().as_bytes().len(),
        started.elapsed()
    );
}
