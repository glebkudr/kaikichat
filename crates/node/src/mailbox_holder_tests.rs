//! Holder service of the mailbox swarm: what it receipts, what it refuses,
//! that one ticket never gets two receipted operations, cursor reads, and the
//! same over the wire between two real runtimes.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use agentic_mailbox_swarm::proof::SenderEquivocation;
use agentic_mailbox_swarm::stamp::{BookKey, operation};
use tempfile::TempDir;

const DOMAIN: [u8; 32] = NETWORK_DOMAIN;
const UNIT: [u8; 32] = [0x75; 32];
const BOOK: [u8; 32] = [0xb0; 32];
const MAILBOX: [u8; 32] = [0xa1; 32];
const OTHER_MAILBOX: [u8; 32] = [0xa2; 32];
/// The period of every service-level test clock (all below one day).
const P0: u64 = 0;

fn identity(seed: u8) -> identity::Keypair {
    identity::Keypair::ed25519_from_bytes([seed; 32]).unwrap()
}

struct Holder {
    _directory: TempDir,
    profile: std::path::PathBuf,
    service: Service,
}

fn holder() -> Holder {
    let directory = TempDir::new().unwrap();
    let profile = directory.path().join("profile.db");
    let service = Service::open(&profile, &[31; 32], &identity(7), DOMAIN).unwrap();
    Holder {
        _directory: directory,
        profile,
        service,
    }
}

fn reopen(holder: Holder) -> Holder {
    let Holder {
        _directory,
        profile,
        service,
    } = holder;
    drop(service);
    let mut service = Service::open(&profile, &[31; 32], &identity(7), DOMAIN).unwrap();
    service.set_unit(UNIT);
    service.learn_book(BOOK, terms(5_000)).unwrap();
    Holder {
        _directory,
        profile,
        service,
    }
}

fn book_key() -> BookKey {
    BookKey::from_bytes(&[3; 32]).unwrap()
}

fn terms(valid_until: u64) -> BookTerms {
    BookTerms {
        key: book_key().account(),
        count: 40,
        valid_until,
    }
}

fn stamped(mailbox: &[u8; 32], envelope: &[u8], index: u32) -> Stamp {
    Stamp::sign(
        &DOMAIN,
        BOOK,
        index,
        operation(mailbox, P0, envelope),
        &book_key(),
    )
}

/// A stamp for an entry of `period`.
fn stamped_in(mailbox: &[u8; 32], period: u64, envelope: &[u8], index: u32) -> Stamp {
    Stamp::sign(
        &DOMAIN,
        BOOK,
        index,
        operation(mailbox, period, envelope),
        &book_key(),
    )
}

fn ready() -> Holder {
    let mut h = holder();
    h.service.set_unit(UNIT);
    h.service.learn_book(BOOK, terms(5_000)).unwrap();
    h
}

#[test]
fn a_holder_receipts_a_stamped_envelope_as_its_unit() {
    let mut h = ready();
    let envelope = b"envelope one".to_vec();
    let stamp = stamped(&MAILBOX, &envelope, 0);
    let receipt = h
        .service
        .store(MAILBOX, P0, &envelope, &stamp, 1_000)
        .unwrap();
    assert_eq!(
        (
            receipt.mailbox,
            receipt.operation,
            receipt.ticket,
            receipt.holder,
            receipt.stored_at
        ),
        (
            MAILBOX,
            operation(&MAILBOX, P0, &envelope),
            stamp.ticket_id(&DOMAIN),
            UNIT,
            1_000
        )
    );
    assert_eq!(receipt.signer(&DOMAIN).unwrap(), h.service.account());
    // The receipt key belongs to the node identity: the same for that
    // identity anywhere, different for another node, and domain-separated
    // from the raw identity secret.
    let account = h.service.account();
    let h = reopen(h);
    assert_eq!(h.service.account(), account);
    for (seed, same) in [(7, true), (8, false)] {
        let directory = TempDir::new().unwrap();
        let other = Service::open(
            &directory.path().join("elsewhere.db"),
            &[32; 32],
            &identity(seed),
            DOMAIN,
        )
        .unwrap();
        assert_eq!(other.account() == account, same);
    }
    assert_ne!(HolderKey::from_bytes(&[7; 32]).unwrap().account(), account);
}

#[test]
fn a_holder_refuses_what_it_cannot_receipt_and_stores_none_of_it() {
    let envelope = b"envelope".to_vec();
    let stamp = stamped(&MAILBOX, &envelope, 0);
    let mut h = holder();
    h.service.learn_book(BOOK, terms(5_000)).unwrap();
    assert_eq!(
        h.service.store(MAILBOX, P0, &envelope, &stamp, 1_000),
        Err(Refusal::NoUnit)
    );
    let mut h = holder();
    h.service.set_unit(UNIT);
    assert_eq!(
        h.service.store(MAILBOX, P0, &envelope, &stamp, 1_000),
        Err(Refusal::UnknownBook)
    );
    let mut h = ready();
    let stranger = BookKey::from_bytes(&[4; 32]).unwrap();
    // Every refused stamp spends ticket 0 on some other operation, so a
    // holder that reserved the ticket before refusing would later answer
    // the genuine store with a conflict.
    let forged = Stamp::sign(
        &DOMAIN,
        BOOK,
        0,
        operation(&MAILBOX, P0, b"forged"),
        &stranger,
    );
    let mut unsigned = stamped(&MAILBOX, b"unsigned", 0);
    unsigned.signature[64] = 0;
    let big = vec![7; MAX_ENVELOPE + 1];
    let cases = [
        (
            b"forged".to_vec(),
            forged,
            1_000,
            Refusal::Stamp(StampError::Signer),
        ),
        (
            b"unsigned".to_vec(),
            unsigned,
            1_000,
            Refusal::Stamp(StampError::Signature),
        ),
        (
            b"outside".to_vec(),
            stamped(&MAILBOX, b"outside", 40),
            1_000,
            Refusal::Stamp(StampError::Index),
        ),
        (
            b"late".to_vec(),
            stamped(&MAILBOX, b"late", 0),
            5_000,
            Refusal::Stamp(StampError::Expired),
        ),
        (
            envelope.clone(),
            stamped(&MAILBOX, b"another", 0),
            1_000,
            Refusal::Operation,
        ),
        // Paid for the same bytes in another mailbox.
        (
            envelope.clone(),
            stamped(&OTHER_MAILBOX, &envelope, 0),
            1_000,
            Refusal::Operation,
        ),
        (
            big.clone(),
            stamped(&MAILBOX, &big, 0),
            1_000,
            Refusal::TooLarge,
        ),
    ];
    for (bytes, stamp, now, refusal) in cases {
        assert_eq!(
            h.service.store(MAILBOX, P0, &bytes, &stamp, now),
            Err(refusal)
        );
    }
    assert!(
        h.service
            .read(&MAILBOX, 0, MAX_PAGE)
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(
        h.service
            .store(MAILBOX, P0, &envelope, &stamp, 1_000)
            .is_ok()
    );
    // The largest allowed envelope is stored.
    let largest = vec![9; MAX_ENVELOPE];
    assert!(
        h.service
            .store(
                MAILBOX,
                P0,
                &largest,
                &stamped(&MAILBOX, &largest, 1),
                1_000
            )
            .is_ok()
    );
}

#[test]
fn a_holder_never_receipts_two_operations_for_one_ticket() {
    let mut h = ready();
    let first = b"first operation".to_vec();
    let second = b"second operation".to_vec();
    let receipt = h
        .service
        .store(MAILBOX, P0, &first, &stamped(&MAILBOX, &first, 0), 1_000)
        .unwrap();
    assert_eq!(
        h.service
            .store(MAILBOX, P0, &second, &stamped(&MAILBOX, &second, 0), 1_001),
        Err(Refusal::Conflict)
    );
    assert_eq!(
        h.service.store(
            OTHER_MAILBOX,
            P0,
            &first,
            &stamped(&OTHER_MAILBOX, &first, 0),
            1_001
        ),
        Err(Refusal::Conflict),
        "the same ticket spent on the same bytes in another mailbox"
    );
    // A forged stamp on the taken ticket learns nothing about it.
    let stranger = BookKey::from_bytes(&[4; 32]).unwrap();
    let forged = Stamp::sign(
        &DOMAIN,
        BOOK,
        0,
        operation(&MAILBOX, P0, &second),
        &stranger,
    );
    assert_eq!(
        h.service.store(MAILBOX, P0, &second, &forged, 1_001),
        Err(Refusal::Stamp(StampError::Signer))
    );
    // The same operation again is answered with the original receipt, once.
    assert_eq!(
        h.service
            .store(MAILBOX, P0, &first, &stamped(&MAILBOX, &first, 0), 2_000),
        Ok(receipt.clone())
    );
    assert_eq!(
        h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries.len(),
        1
    );
    // Durable across a restart.
    let mut h = reopen(h);
    assert_eq!(
        h.service
            .store(MAILBOX, P0, &second, &stamped(&MAILBOX, &second, 0), 3_000),
        Err(Refusal::Conflict)
    );
    assert_eq!(
        h.service
            .store(MAILBOX, P0, &first, &stamped(&MAILBOX, &first, 0), 3_000),
        Ok(receipt)
    );
    assert_eq!(
        h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries.len(),
        1
    );
}

#[test]
fn reading_pages_a_mailbox_in_arrival_order_by_cursor() {
    let mut h = ready();
    // Twenty entries interleaved with another mailbox's, crossing seq 9→10
    // and more than one page.
    let mut mine = Vec::new();
    for n in 0..20u32 {
        let envelope = vec![u8::try_from(n).unwrap(); 10 + n as usize];
        let stamp = stamped(&MAILBOX, &envelope, n);
        h.service
            .store(MAILBOX, P0, &envelope, &stamp, 1_000 + u64::from(n))
            .unwrap();
        mine.push((envelope, stamp, 1_000 + u64::from(n)));
        if n % 3 == 0 {
            let envelope = vec![0xee; 3 + n as usize];
            h.service
                .store(
                    OTHER_MAILBOX,
                    P0,
                    &envelope,
                    &stamped(&OTHER_MAILBOX, &envelope, 20 + n),
                    1_100,
                )
                .unwrap();
        }
    }
    for limit in [MAX_PAGE + 5, usize::MAX] {
        assert_eq!(
            h.service.read(&MAILBOX, 0, limit).unwrap().entries.len(),
            MAX_PAGE
        );
    }
    let empty = h.service.read(&MAILBOX, 3, 0).unwrap();
    assert!(empty.entries.is_empty());
    assert_eq!(empty.next, 3);
    let first = h.service.read(&MAILBOX, 0, 2).unwrap();
    assert_eq!(first.entries.len(), 2);
    assert_eq!(first.next, first.entries[1].seq);
    let middle = h.service.read(&MAILBOX, first.next, MAX_PAGE).unwrap();
    assert_eq!(middle.entries.len(), MAX_PAGE);
    let last = h.service.read(&MAILBOX, middle.next, MAX_PAGE).unwrap();
    assert_eq!(last.entries.len(), 2);
    let all: Vec<_> = first
        .entries
        .iter()
        .chain(&middle.entries)
        .chain(&last.entries)
        .collect();
    assert!(all.windows(2).all(|w| w[0].seq < w[1].seq));
    for (entry, (envelope, stamp, stored_at)) in all.iter().zip(&mine) {
        assert_eq!(
            (&entry.envelope, &entry.stamp, entry.stored_at),
            (envelope, stamp, *stored_at)
        );
    }
    let end = h.service.read(&MAILBOX, last.next, MAX_PAGE).unwrap();
    assert!(end.entries.is_empty());
    assert_eq!(end.next, last.next);
    let other = h.service.read(&OTHER_MAILBOX, 0, MAX_PAGE).unwrap();
    assert_eq!(other.entries.len(), 7);
    // Everything is still there after a restart.
    let h = reopen(h);
    let again = h.service.read(&MAILBOX, 0, 2).unwrap();
    assert_eq!(again.entries.iter().collect::<Vec<_>>(), all[..2]);
}

#[test]
fn every_refusal_has_its_own_wire_code() {
    let codes = [
        (Refusal::NoUnit, "no_unit"),
        (Refusal::UnknownBook, "unknown_book"),
        (Refusal::Stamp(StampError::Signature), "stamp_signature"),
        (Refusal::Stamp(StampError::Signer), "stamp_signer"),
        (Refusal::Stamp(StampError::Index), "stamp_index"),
        (Refusal::Stamp(StampError::Expired), "stamp_expired"),
        (Refusal::Operation, "operation"),
        (Refusal::TooLarge, "too_large"),
        (Refusal::Conflict, "conflict"),
        (Refusal::Period, "period"),
        (Refusal::Grant, "grant"),
        (Refusal::GrantPending, "grant_pending"),
        (Refusal::Blocked, "blocked"),
        (Refusal::Malformed, "malformed"),
        (Refusal::Storage, "storage"),
        (Refusal::AccessRequired, "access_required"),
        (Refusal::Pass, "bad_pass"),
        (Refusal::BookExpired, "book_expired"),
        (Refusal::UnknownUnit, "unknown_unit"),
    ];
    for (refusal, code) in codes {
        assert_eq!(refusal.code(), code);
    }
}

async fn exchange(
    client: &mut Runtime,
    holder: &mut Runtime,
    request: Request,
) -> std::result::Result<Response, String> {
    let peer = *holder.swarm.local_peer_id();
    let id = client.mailbox_request(peer, request);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(result) = client.mailbox_client.results.remove(&id) {
                return result;
            }
            tokio::select! {
                event = client.swarm.select_next_some() => client.event(event),
                event = holder.swarm.select_next_some() => holder.event(event),
            }
        }
    })
    .await
    .unwrap()
}

/// A gated holder (one that reads the chain) takes nothing but a pass from a
/// peer it does not know, then serves that peer; the same pass shown by
/// another peer is refused and that peer is kept out for a while.
#[tokio::test]
async fn a_gated_holder_serves_a_peer_once_it_showed_its_own_pass() {
    const CLIENT_BOOK: [u8; 32] = [0xbc; 32];
    let directories = [(); 3].map(|_| TempDir::new().unwrap());
    let mut client = test_support::runtime(directories[0].path());
    let mut stranger = test_support::runtime(directories[1].path());
    let mut holder = test_support::runtime(directories[2].path());
    holder.access_gate.set_enabled(true);
    holder.mailbox_holder.set_unit(UNIT);
    client.core.create_profile("Client").unwrap();
    let key = client.core.mailbox_book_account().unwrap();
    client
        .core
        .add_mailbox_book(CLIENT_BOOK, 10, u64::MAX)
        .unwrap();
    holder
        .mailbox_holder
        .learn_book(
            CLIENT_BOOK,
            BookTerms {
                key,
                count: 10,
                valid_until: u64::MAX,
            },
        )
        .unwrap();
    for peer in [&mut client, &mut stranger] {
        tokio::time::timeout(
            Duration::from_secs(10),
            test_support::connect(peer, &mut holder, false),
        )
        .await
        .unwrap();
    }
    let now = clock::wall().unwrap();
    let read = Request::Read {
        mailbox: MAILBOX.to_vec(),
        after: 0,
        limit: 16,
    };
    let refused = |code: &str| Response::Refused { code: code.into() };
    assert_eq!(
        exchange(&mut client, &mut holder, read.clone())
            .await
            .unwrap(),
        refused("access_required")
    );
    let access = client
        .core
        .mailbox_access(
            client.own_transport_key(),
            agentic_mailbox_swarm::address::period(now),
            now,
        )
        .unwrap()
        .unwrap();
    assert_eq!(access.grant, None);
    let show = Request::Access {
        credential: CredentialWire::Pass {
            pass: AccessPassWire::from(&access.pass),
            grant: None,
        },
    };
    let Response::Access { until } = exchange(&mut client, &mut holder, show.clone())
        .await
        .unwrap()
    else {
        panic!("access");
    };
    assert!(until > now);
    assert!(matches!(
        exchange(&mut client, &mut holder, read.clone())
            .await
            .unwrap(),
        Response::Page { .. }
    ));
    assert_eq!(
        exchange(&mut stranger, &mut holder, show).await.unwrap(),
        refused("bad_pass")
    );
    assert!(exchange(&mut stranger, &mut holder, read).await.is_err());
}

#[tokio::test]
async fn the_mailbox_protocol_stores_refuses_and_reads_between_two_runtimes() {
    let (client_directory, holder_directory) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut client = test_support::runtime(client_directory.path());
    let mut holder = test_support::runtime(holder_directory.path());
    holder.mailbox_holder.set_unit(UNIT);
    // The holder serves on its own wall clock; this book outlives the test.
    holder
        .mailbox_holder
        .learn_book(BOOK, terms(u64::MAX))
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(10),
        test_support::connect(&mut client, &mut holder, false),
    )
    .await
    .unwrap();
    // Entries of the holder's current period (its own wall clock).
    let live = agentic_mailbox_swarm::address::period(clock::wall().unwrap());
    let envelope = b"over the wire".to_vec();
    let stamp = stamped_in(&MAILBOX, live, &envelope, 1);
    let store = |envelope: &[u8], stamp: &Stamp| Request::Store {
        mailbox: MAILBOX.to_vec(),
        period: live,
        envelope: envelope.to_vec(),
        stamp: StampWire::from(stamp),
    };
    let Response::Stored { receipt } = exchange(&mut client, &mut holder, store(&envelope, &stamp))
        .await
        .unwrap()
    else {
        panic!("store");
    };
    let receipt = Receipt::try_from(&receipt).unwrap();
    assert_eq!(
        receipt.signer(&DOMAIN).unwrap(),
        holder.mailbox_holder.account()
    );
    assert_eq!(
        (receipt.mailbox, receipt.operation, receipt.holder),
        (MAILBOX, operation(&MAILBOX, live, &envelope), UNIT)
    );
    let refused = |code: &str| Response::Refused { code: code.into() };
    assert_eq!(
        exchange(&mut client, &mut holder, store(b"not paid for", &stamp))
            .await
            .unwrap(),
        refused("operation")
    );
    let double = b"double spend".to_vec();
    assert_eq!(
        exchange(
            &mut client,
            &mut holder,
            store(&double, &stamped_in(&MAILBOX, live, &double, 1))
        )
        .await
        .unwrap(),
        refused("conflict")
    );
    // A fresh entry of a period the holder no longer takes.
    let stale = b"stale".to_vec();
    assert_eq!(
        exchange(
            &mut client,
            &mut holder,
            Request::Store {
                mailbox: OTHER_MAILBOX.to_vec(),
                period: live - 2,
                envelope: stale.clone(),
                stamp: StampWire::from(&stamped_in(&OTHER_MAILBOX, live - 2, &stale, 2)),
            }
        )
        .await
        .unwrap(),
        refused("period")
    );
    // Malformed fields are refused as such; the holder keeps serving.
    let wire = StampWire::from(&stamp);
    let malformed = [
        Request::Store {
            mailbox: vec![0xa1; 31],
            period: live,
            envelope: envelope.clone(),
            stamp: wire.clone(),
        },
        Request::Store {
            mailbox: MAILBOX.to_vec(),
            period: live,
            envelope: envelope.clone(),
            stamp: StampWire {
                book: vec![0xb0; 31],
                ..wire.clone()
            },
        },
        Request::Store {
            mailbox: MAILBOX.to_vec(),
            period: live,
            envelope: envelope.clone(),
            stamp: StampWire {
                operation: vec![1; 31],
                ..wire.clone()
            },
        },
        Request::Store {
            mailbox: MAILBOX.to_vec(),
            period: live,
            envelope: envelope.clone(),
            stamp: StampWire {
                signature: wire.signature[..64].to_vec(),
                ..wire.clone()
            },
        },
        Request::Read {
            mailbox: vec![0xa1; 31],
            after: 0,
            limit: 10,
        },
    ];
    for request in malformed {
        assert_eq!(
            exchange(&mut client, &mut holder, request).await.unwrap(),
            refused("malformed")
        );
    }
    let read = Request::Read {
        mailbox: MAILBOX.to_vec(),
        after: 0,
        limit: 10,
    };
    let Response::Page {
        entries,
        next,
        grants,
    } = exchange(&mut client, &mut holder, read).await.unwrap()
    else {
        panic!("read");
    };
    // A bought book has no grant to pass on.
    assert!(grants.is_empty());
    assert_eq!(entries.len(), 1);
    assert_eq!(next, entries[0].seq);
    assert_eq!(entries[0].envelope, envelope);
    assert_eq!(entries[0].period, live);
    assert_eq!(Stamp::try_from(&entries[0].stamp).unwrap(), stamp);
}

// --- replication inside the swarm -------------------------------------------

#[test]
fn a_mailbox_summary_is_its_count_and_an_order_free_digest() {
    let envelopes: Vec<Vec<u8>> = (0..3u8).map(|n| vec![n; 5 + n as usize]).collect();
    let store_at = |h: &mut Holder, order: &[usize], now: u64| {
        for i in order {
            let e = &envelopes[*i];
            let index = u32::try_from(*i).unwrap();
            h.service
                .store(MAILBOX, P0, e, &stamped(&MAILBOX, e, index), now)
                .unwrap();
        }
    };
    let store = |h: &mut Holder, order: &[usize]| store_at(h, order, 1_000);
    let mut a = ready();
    assert_eq!(a.service.summary(&MAILBOX).unwrap(), Summary::default());
    store(&mut a, &[0, 1]);
    let two = a.service.summary(&MAILBOX).unwrap();
    assert_eq!(two.count, 2);
    store(&mut a, &[2]);
    let three = a.service.summary(&MAILBOX).unwrap();
    assert_eq!(three.count, 3);
    assert_ne!(three.digest, two.digest);
    // Another holder (other receipt key), another time and arrival order,
    // the same set: the same summary.
    let directory = TempDir::new().unwrap();
    let profile = directory.path().join("profile.db");
    let mut b = Holder {
        service: Service::open(&profile, &[31; 32], &identity(8), DOMAIN).unwrap(),
        _directory: directory,
        profile,
    };
    b.service.set_unit([0x76; 32]);
    b.service.learn_book(BOOK, terms(5_000)).unwrap();
    assert_ne!(b.service.account(), a.service.account());
    store_at(&mut b, &[2, 0, 1], 2_000);
    assert_eq!(b.service.summary(&MAILBOX).unwrap(), three);
    // The same count of other operations differs.
    let mut c = ready();
    store(&mut c, &[0, 1]);
    let other = b"other".to_vec();
    c.service
        .store(MAILBOX, P0, &other, &stamped(&MAILBOX, &other, 2), 1_000)
        .unwrap();
    let different = c.service.summary(&MAILBOX).unwrap();
    assert_eq!(different.count, 3);
    assert_ne!(different.digest, three.digest);
    // Repeats, refusals and other mailboxes leave it unchanged.
    store(&mut a, &[0]);
    a.service
        .store(
            OTHER_MAILBOX,
            P0,
            b"elsewhere",
            &stamped(&OTHER_MAILBOX, b"elsewhere", 9),
            1_000,
        )
        .unwrap();
    assert_eq!(
        a.service
            .store(MAILBOX, P0, b"x", &stamped(&MAILBOX, b"x", 0), 1_000),
        Err(Refusal::Conflict)
    );
    assert_eq!(a.service.summary(&MAILBOX).unwrap(), three);
    assert_eq!(a.service.summary(&OTHER_MAILBOX).unwrap().count, 1);
    let a = reopen(a);
    assert_eq!(a.service.summary(&MAILBOX).unwrap(), three);
}

#[test]
fn a_holder_lists_its_mailboxes_with_their_summaries_in_id_order() {
    let mut h = ready();
    // A second book pays for more mailboxes than the MAX_SUMMARIES clamp.
    const WIDE: [u8; 32] = [0xb1; 32];
    h.service
        .learn_book(
            WIDE,
            BookTerms {
                key: book_key().account(),
                count: 100,
                valid_until: 5_000,
            },
        )
        .unwrap();
    let wide = |mailbox: &[u8; 32], envelope: &[u8], index: u32| {
        Stamp::sign(
            &DOMAIN,
            WIDE,
            index,
            operation(mailbox, P0, envelope),
            &book_key(),
        )
    };
    // Stored out of id order; one mailbox holds two.
    let mailboxes: Vec<[u8; 32]> = (0..70u8).rev().map(|n| [n; 32]).collect();
    for (i, mailbox) in mailboxes.iter().enumerate() {
        let envelope = vec![7; 3];
        let index = u32::try_from(i).unwrap();
        h.service
            .store(
                *mailbox,
                P0,
                &envelope,
                &wide(mailbox, &envelope, index),
                1_000,
            )
            .unwrap();
    }
    h.service
        .store(
            [4; 32],
            P0,
            b"second",
            &wide(&[4; 32], b"second", 80),
            1_000,
        )
        .unwrap();
    let mut h = reopen(h);
    h.service
        .learn_book(
            WIDE,
            BookTerms {
                key: book_key().account(),
                count: 100,
                valid_until: 5_000,
            },
        )
        .unwrap();
    let mut listed = Vec::new();
    let mut after = None;
    for _ in 0..20 {
        let page = h.service.mailboxes(after.as_ref(), 8).unwrap();
        assert!(page.len() <= 8);
        let Some((last, _)) = page.last().copied() else {
            break;
        };
        listed.extend(page);
        after = Some(last);
    }
    let mut expected = mailboxes.clone();
    expected.sort();
    assert_eq!(listed.iter().map(|(m, _)| *m).collect::<Vec<_>>(), expected);
    for (mailbox, summary) in &listed {
        assert_eq!(*summary, h.service.summary(mailbox).unwrap());
    }
    assert_eq!(listed[4].1.count, 2);
    assert!(h.service.mailboxes(None, 0).unwrap().is_empty());
    let clamped = h.service.mailboxes(None, usize::MAX).unwrap();
    assert_eq!(clamped.len(), MAX_SUMMARIES);
    assert_eq!(clamped[..], listed[..MAX_SUMMARIES]);
    assert!(h.service.mailboxes(Some(&[69; 32]), 8).unwrap().is_empty());
}

#[test]
fn a_conflicting_stamp_is_kept_once_as_proof_against_its_book() {
    let mut h = ready();
    assert!(h.service.equivocations().unwrap().is_empty());
    let first = stamped(&MAILBOX, b"first", 0);
    let second = stamped(&MAILBOX, b"second", 0);
    h.service
        .store(MAILBOX, P0, b"first", &first, 1_000)
        .unwrap();
    // Slot 1 is spent before the book is proven.
    h.service
        .store(MAILBOX, P0, b"a", &stamped(&MAILBOX, b"a", 1), 1_000)
        .unwrap();
    assert_eq!(
        h.service.store(MAILBOX, P0, b"second", &second, 1_000),
        Err(Refusal::Conflict)
    );
    let proofs = h.service.equivocations().unwrap();
    assert_eq!(
        proofs,
        vec![SenderEquivocation {
            first: first.clone(),
            second: second.clone(),
        }]
    );
    assert_eq!(proofs[0].verify(&DOMAIN, &book_key().account()), Ok(()));
    // More conflicts on the slot add nothing; forged ones never count.
    let stranger = BookKey::from_bytes(&[4; 32]).unwrap();
    let forged = Stamp::sign(
        &DOMAIN,
        BOOK,
        0,
        operation(&MAILBOX, P0, b"forged"),
        &stranger,
    );
    assert_eq!(
        h.service.store(MAILBOX, P0, b"forged", &forged, 1_000),
        Err(Refusal::Stamp(StampError::Signer))
    );
    for envelope in [&b"third"[..], b"second"] {
        assert_eq!(
            h.service.store(
                MAILBOX,
                P0,
                envelope,
                &stamped(&MAILBOX, envelope, 0),
                1_000
            ),
            Err(Refusal::Conflict)
        );
    }
    assert_eq!(h.service.equivocations().unwrap().len(), 1);
    // Another slot is another proof; both survive a restart.
    assert_eq!(
        h.service.store(
            OTHER_MAILBOX,
            P0,
            b"a",
            &stamped(&OTHER_MAILBOX, b"a", 1),
            1_000
        ),
        Err(Refusal::Conflict)
    );
    let h = reopen(h);
    let proofs = h.service.equivocations().unwrap();
    assert_eq!(proofs.len(), 2);
    for proof in &proofs {
        assert_eq!(proof.verify(&DOMAIN, &book_key().account()), Ok(()));
    }
    // Proofs never replace what the holder stored.
    let kept: Vec<_> = h
        .service
        .read(&MAILBOX, 0, MAX_PAGE)
        .unwrap()
        .entries
        .into_iter()
        .map(|e| (e.envelope, e.stamp))
        .collect();
    assert_eq!(
        kept,
        vec![
            (b"first".to_vec(), first),
            (b"a".to_vec(), stamped(&MAILBOX, b"a", 1)),
        ]
    );
    assert!(
        h.service
            .read(&OTHER_MAILBOX, 0, MAX_PAGE)
            .unwrap()
            .entries
            .is_empty()
    );
}

#[tokio::test]
async fn summaries_are_answered_with_the_holders_own_view() {
    let (client_directory, holder_directory) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut client = test_support::runtime(client_directory.path());
    let mut holder = test_support::runtime(holder_directory.path());
    holder.mailbox_holder.set_unit(UNIT);
    holder
        .mailbox_holder
        .learn_book(BOOK, terms(u64::MAX))
        .unwrap();
    for (n, envelope) in [&b"one"[..], b"two"].iter().enumerate() {
        let index = u32::try_from(n).unwrap();
        holder
            .mailbox_holder
            .store(
                MAILBOX,
                P0,
                envelope,
                &stamped(&MAILBOX, envelope, index),
                1_000,
            )
            .unwrap();
    }
    let own = holder.mailbox_holder.summary(&MAILBOX).unwrap();
    tokio::time::timeout(
        Duration::from_secs(10),
        test_support::connect(&mut client, &mut holder, false),
    )
    .await
    .unwrap();
    let item = |mailbox: [u8; 32], summary: Summary| SummaryWire {
        mailbox: mailbox.to_vec(),
        count: summary.count,
        digest: summary.digest.to_vec(),
    };
    // Not in id order: answers follow the request.
    let asked = vec![
        item([0xa3; 32], Summary::default()),
        item(MAILBOX, own),
        item(
            OTHER_MAILBOX,
            Summary {
                count: 5,
                digest: [9; 32],
            },
        ),
    ];
    let answer = exchange(
        &mut client,
        &mut holder,
        Request::Summaries {
            items: asked.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        answer,
        Response::Summaries {
            items: vec![
                item([0xa3; 32], Summary::default()),
                item(MAILBOX, own),
                item(OTHER_MAILBOX, Summary::default()),
            ]
        }
    );
    let refused = Response::Refused {
        code: "malformed".into(),
    };
    let short = SummaryWire {
        mailbox: vec![0xa1; 31],
        ..asked[1].clone()
    };
    let bad_digest = SummaryWire {
        digest: vec![1; 31],
        ..asked[1].clone()
    };
    for items in [
        vec![short],
        vec![bad_digest],
        vec![asked[0].clone(); MAX_SUMMARIES + 1],
    ] {
        assert_eq!(
            exchange(&mut client, &mut holder, Request::Summaries { items })
                .await
                .unwrap(),
            refused
        );
    }
    assert_eq!(
        exchange(
            &mut client,
            &mut holder,
            Request::Summaries {
                items: vec![asked[0].clone(); MAX_SUMMARIES]
            }
        )
        .await
        .unwrap(),
        Response::Summaries {
            items: vec![asked[0].clone(); MAX_SUMMARIES]
        }
    );
}

// --- retention -----------------------------------------------------------------

const DAY: u64 = agentic_mailbox_swarm::address::PERIOD_SECONDS;

/// A book the holder read from the chain: kept across a restart, and until
/// every mailbox it could pay for has ended, so late copies are repaired.
#[test]
fn a_book_from_the_chain_outlives_a_restart_and_the_mailboxes_it_paid_for() {
    let end = 20 * DAY;
    let mut h = holder();
    h.service.set_unit(UNIT);
    h.service.learn_book(BOOK, terms(end)).unwrap();
    let Holder {
        _directory,
        profile,
        service,
    } = h;
    drop(service);
    let mut service = Service::open(&profile, &[31; 32], &identity(7), DOMAIN).unwrap();
    service.set_unit(UNIT);
    let mut h = Holder {
        _directory,
        profile,
        service,
    };
    h.service
        .store(
            MAILBOX,
            19,
            b"last",
            &stamped_in(&MAILBOX, 19, b"last", 0),
            19 * DAY + 5,
        )
        .unwrap();
    // After it ends, a copy paid in its time is still repaired.
    h.service.collect(end + DAY).unwrap();
    h.service
        .store_replica(
            OTHER_MAILBOX,
            19,
            b"copy",
            &stamped_in(&OTHER_MAILBOX, 19, b"copy", 1),
            end + DAY,
        )
        .unwrap();
    let after = [0xa3; 32];
    let later = |h: &mut Holder, at: u64| {
        h.service.store_replica(
            after,
            21,
            b"after",
            &stamped_in(&after, 21, b"after", 2),
            at,
        )
    };
    // A stamp of a period after its end is refused by the known book...
    assert_eq!(
        later(&mut h, end + DAY),
        Err(Refusal::Stamp(StampError::Expired))
    );
    // ...up to the end of the mailboxes of the period it ends in...
    let gone = agentic_mailbox_swarm::address::expires_at(20);
    h.service.collect(gone - 1).unwrap();
    assert_eq!(
        later(&mut h, gone - 1),
        Err(Refusal::Stamp(StampError::Expired))
    );
    // ...and is of an unknown book after that.
    h.service.collect(gone).unwrap();
    assert_eq!(later(&mut h, gone), Err(Refusal::UnknownBook));
}

/// A holder whose book is valid until `valid_until`.
fn ready_until(valid_until: u64) -> Holder {
    let mut h = ready();
    h.service.learn_book(BOOK, terms(valid_until)).unwrap();
    h
}

#[test]
fn a_holder_takes_fresh_entries_only_around_their_period() {
    let mut h = ready_until(u64::MAX);
    let p = 10;
    let now = p * DAY + 5;
    // Refused fresh entries reserve nothing: slot 0 stays free.
    for stale in [p - 2, p + 2] {
        let envelope = format!("stale {stale}").into_bytes();
        assert_eq!(
            h.service.store(
                MAILBOX,
                stale,
                &envelope,
                &stamped_in(&MAILBOX, stale, &envelope, 0),
                now
            ),
            Err(Refusal::Period),
            "{stale}"
        );
    }
    h.service
        .store(MAILBOX, p, b"now", &stamped_in(&MAILBOX, p, b"now", 0), now)
        .unwrap();
    for (n, writable) in [p - 1, p + 1].into_iter().enumerate() {
        let mailbox = [0xc0 + u8::try_from(n).unwrap(); 32];
        let index = u32::try_from(n).unwrap() + 1;
        h.service
            .store(
                mailbox,
                writable,
                b"edge",
                &stamped_in(&mailbox, writable, b"edge", index),
                now,
            )
            .unwrap();
        let entries = h.service.read(&mailbox, 0, MAX_PAGE).unwrap().entries;
        assert_eq!(entries[0].period, writable);
    }
    // One mailbox, one period: a stamp for another period is refused.
    assert_eq!(
        h.service.store(
            MAILBOX,
            p + 1,
            b"moved",
            &stamped_in(&MAILBOX, p + 1, b"moved", 5),
            now
        ),
        Err(Refusal::Period)
    );
    let entries = h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        (entries[0].period, &entries[0].envelope[..]),
        (p, &b"now"[..])
    );
    // Only the three accepted entries spent slots; the rule holds after a
    // restart too.
    assert_eq!(h.service.held_tickets().unwrap(), 3);
    let mut h = reopen(h);
    h.service.learn_book(BOOK, terms(u64::MAX)).unwrap();
    assert_eq!(
        h.service.store(
            MAILBOX,
            p + 1,
            b"moved again",
            &stamped_in(&MAILBOX, p + 1, b"moved again", 6),
            now
        ),
        Err(Refusal::Period)
    );
}

#[test]
fn an_expired_mailbox_is_collected_and_its_tickets_live_as_long_as_they_matter() {
    use agentic_mailbox_swarm::address::expires_at;
    let valid_until = 50 * DAY;
    let mut h = ready_until(valid_until);
    h.service
        .store(
            MAILBOX,
            10,
            b"kept",
            &stamped_in(&MAILBOX, 10, b"kept", 0),
            10 * DAY + 5,
        )
        .unwrap();
    h.service
        .store(
            OTHER_MAILBOX,
            30,
            b"later",
            &stamped_in(&OTHER_MAILBOX, 30, b"later", 1),
            30 * DAY + 5,
        )
        .unwrap();
    assert_eq!(h.service.held_tickets().unwrap(), 2);
    h.service.collect(expires_at(10) - 1).unwrap();
    assert_eq!(
        h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries.len(),
        1
    );
    // The mailbox expires: entries, summary and listing go.
    h.service.collect(expires_at(10)).unwrap();
    assert!(
        h.service
            .read(&MAILBOX, 0, MAX_PAGE)
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(h.service.summary(&MAILBOX).unwrap(), Summary::default());
    let listed: Vec<_> = h
        .service
        .mailboxes(None, MAX_SUMMARIES)
        .unwrap()
        .into_iter()
        .map(|(m, _)| m)
        .collect();
    assert_eq!(listed, vec![OTHER_MAILBOX]);
    // Its slot stays spent while the book can still sign: a re-spend in a
    // new mailbox is a conflict, also after a restart.
    let mut h = reopen(h);
    h.service.learn_book(BOOK, terms(valid_until)).unwrap();
    let again = [0xc9; 32];
    let spend = |h: &mut Holder| {
        h.service.store(
            again,
            45,
            b"again",
            &stamped_in(&again, 45, b"again", 0),
            45 * DAY + 5,
        )
    };
    assert_eq!(spend(&mut h), Err(Refusal::Conflict));
    assert_eq!(h.service.held_tickets().unwrap(), 2);
    h.service.collect(valid_until - 1).unwrap();
    assert_eq!(h.service.held_tickets().unwrap(), 2);
    // An expired mailbox is not brought back by a replica.
    assert_eq!(
        h.service.store_replica(
            MAILBOX,
            10,
            b"kept",
            &stamped_in(&MAILBOX, 10, b"kept", 0),
            45 * DAY
        ),
        Err(Refusal::Period)
    );
    // A ticket goes once both its book and its mailbox have ended.
    h.service.collect(valid_until).unwrap();
    assert_eq!(h.service.held_tickets().unwrap(), 1);
    assert_eq!(
        h.service
            .read(&OTHER_MAILBOX, 0, MAX_PAGE)
            .unwrap()
            .entries
            .len(),
        1
    );
    h.service.collect(expires_at(30)).unwrap();
    assert_eq!(h.service.held_tickets().unwrap(), 0);
    assert!(
        h.service
            .read(&OTHER_MAILBOX, 0, MAX_PAGE)
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(h.service.mailboxes(None, MAX_SUMMARIES).unwrap().is_empty());
}

#[test]
fn a_replica_is_judged_at_its_period_not_when_it_is_pulled() {
    use agentic_mailbox_swarm::address::expires_at;
    // The book ended when period 12 began; period 10 was paid in time.
    let mut h = ready_until(12 * DAY);
    let now = 20 * DAY + 5;
    let (envelope, stamp) = (
        b"paid in time".to_vec(),
        stamped_in(&MAILBOX, 10, b"paid in time", 0),
    );
    // Too old to be written fresh, still kept: taken as a replica.
    assert_eq!(
        h.service.store(MAILBOX, 10, &envelope, &stamp, now),
        Err(Refusal::Period)
    );
    let receipt = h
        .service
        .store_replica(MAILBOX, 10, &envelope, &stamp, now)
        .unwrap();
    assert_eq!(receipt.operation, operation(&MAILBOX, 10, &envelope));
    let entries = h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries;
    assert_eq!((entries.len(), entries[0].period), (1, 10));
    // A period that began after the book ended was never paid for, and a
    // replica cannot claim an earlier period than its stamp paid for.
    let late = [0xc1; 32];
    assert_eq!(
        h.service
            .store_replica(late, 10, b"late", &stamped_in(&late, 12, b"late", 1), now),
        Err(Refusal::Operation)
    );
    assert_eq!(
        h.service
            .store_replica(late, 12, b"late", &stamped_in(&late, 12, b"late", 1), now),
        Err(Refusal::Stamp(StampError::Expired))
    );
    // A mailbox past its retention is not taken back.
    let gone = [0xc2; 32];
    assert_eq!(
        h.service.store_replica(
            gone,
            10,
            b"gone",
            &stamped_in(&gone, 10, b"gone", 2),
            expires_at(10)
        ),
        Err(Refusal::Period)
    );
    // Replicas obey the same slot and mailbox rules.
    assert_eq!(
        h.service.store_replica(
            MAILBOX,
            10,
            b"other",
            &stamped_in(&MAILBOX, 10, b"other", 0),
            now
        ),
        Err(Refusal::Conflict)
    );
    assert_eq!(h.service.equivocations().unwrap().len(), 1);
    assert_eq!(
        h.service.store_replica(
            MAILBOX,
            11,
            b"moved",
            &stamped_in(&MAILBOX, 11, b"moved", 3),
            now
        ),
        Err(Refusal::Period)
    );
    let forged = Stamp::sign(
        &DOMAIN,
        BOOK,
        4,
        operation(&[0xc3; 32], 10, b"forged"),
        &BookKey::from_bytes(&[4; 32]).unwrap(),
    );
    assert_eq!(
        h.service
            .store_replica([0xc3; 32], 10, b"forged", &forged, now),
        Err(Refusal::Stamp(StampError::Signer))
    );
    assert_eq!(
        h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries.len(),
        1
    );
    assert_eq!(h.service.held_tickets().unwrap(), 1);
    // Kept from its own period, not from when it was pulled (day 20).
    h.service.collect(expires_at(10) - 1).unwrap();
    assert_eq!(
        h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries.len(),
        1
    );
    h.service.collect(expires_at(10)).unwrap();
    assert!(
        h.service
            .read(&MAILBOX, 0, MAX_PAGE)
            .unwrap()
            .entries
            .is_empty()
    );
}

// --- notary --------------------------------------------------------------------

use agentic_grant_book::{GrantBook, GrantEquivocation, GrantTerms, SecpKey};

fn server() -> SecpKey {
    SecpKey::from_secret(&[0x31; 32]).unwrap()
}

fn grant(book: [u8; 20], serial: u32, expiry: u64) -> GrantBook {
    GrantBook::issue(
        GrantTerms {
            domain: DOMAIN,
            book,
            day: 10,
            serial,
            count: 100,
            expiry,
        },
        &server(),
    )
}

#[test]
fn a_notary_keeps_the_first_stamp_of_a_slot_and_when_it_saw_it() {
    let mut h = ready_until(u64::MAX);
    let first = stamped_in(&MAILBOX, 10, b"one", 0);
    let key = first.ticket_id(&DOMAIN);
    assert_eq!(h.service.notary_record(&key).unwrap(), None);
    let noted = Notarized {
        first: Statement::Ticket(first.clone()),
        first_seen: 1_000,
    };
    assert_eq!(
        h.service
            .notarize(&Statement::Ticket(first.clone()), 1_000)
            .unwrap(),
        noted
    );
    // Repeats keep the first time.
    assert_eq!(
        h.service
            .notarize(&Statement::Ticket(first.clone()), 2_000)
            .unwrap(),
        noted
    );
    // Unverifiable statements take no slot.
    let stranger = BookKey::from_bytes(&[4; 32]).unwrap();
    let forged = Stamp::sign(&DOMAIN, BOOK, 1, operation(&MAILBOX, 10, b"x"), &stranger);
    assert_eq!(
        h.service.notarize(&Statement::Ticket(forged), 4_000),
        Err(Refusal::Stamp(StampError::Signer))
    );
    assert_eq!(
        h.service.notarize(
            &Statement::Ticket(stamped_in(&MAILBOX, 10, b"y", 40)),
            4_000
        ),
        Err(Refusal::Stamp(StampError::Index))
    );
    let unknown = Stamp::sign(
        &DOMAIN,
        [0xbb; 32],
        1,
        operation(&MAILBOX, 10, b"z"),
        &book_key(),
    );
    assert_eq!(
        h.service.notarize(&Statement::Ticket(unknown), 4_000),
        Err(Refusal::UnknownBook)
    );
    let genuine = stamped_in(&MAILBOX, 10, b"x", 1);
    assert_eq!(
        h.service
            .notarize(&Statement::Ticket(genuine.clone()), 5_000)
            .unwrap(),
        Notarized {
            first: Statement::Ticket(genuine),
            first_seen: 5_000,
        }
    );
    // A second operation on the slot, in another mailbox: the first stands,
    // and the pair is proof against the book.
    let second = stamped_in(&OTHER_MAILBOX, 11, b"two", 0);
    assert_eq!(second.ticket_id(&DOMAIN), key);
    assert_eq!(
        h.service
            .notarize(&Statement::Ticket(second.clone()), 3_000)
            .unwrap(),
        noted
    );
    assert_eq!(
        h.service.equivocations().unwrap(),
        vec![SenderEquivocation {
            first: first.clone(),
            second: second.clone(),
        }]
    );
    // One proof per slot, however often it is spent again.
    for again in [second, stamped_in(&MAILBOX, 12, b"three", 0)] {
        assert_eq!(
            h.service
                .notarize(&Statement::Ticket(again), 3_500)
                .unwrap(),
            noted
        );
    }
    assert_eq!(h.service.equivocations().unwrap().len(), 1);
    assert!(h.service.grant_equivocations().unwrap().is_empty());
    // Notarizing is not storing, and it is durable.
    assert!(
        h.service
            .read(&MAILBOX, 0, MAX_PAGE)
            .unwrap()
            .entries
            .is_empty()
    );
    let h = reopen(h);
    assert_eq!(h.service.notary_record(&key).unwrap(), Some(noted));
    // A slot spent in time may be registered after its book ended.
    let mut late = ready_until(2_000);
    let spent = stamped_in(&MAILBOX, 0, b"in time", 2);
    assert_eq!(
        late.service
            .notarize(&Statement::Ticket(spent.clone()), 3_000)
            .unwrap(),
        Notarized {
            first: Statement::Ticket(spent),
            first_seen: 3_000,
        }
    );
}

#[test]
fn a_notary_keeps_the_first_grant_of_a_serial() {
    let mut h = ready();
    let first = grant([1; 20], 3, 30 * DAY);
    let key = first.id();
    let noted = Notarized {
        first: Statement::Grant(first.clone()),
        first_seen: 10 * DAY + 5,
    };
    assert_eq!(
        h.service
            .notarize(&Statement::Grant(first.clone()), 10 * DAY + 5)
            .unwrap(),
        noted
    );
    // The same serial granted again to another book: the issuer
    // equivocated.
    let second = grant([2; 20], 3, 30 * DAY);
    assert_eq!(second.id(), key);
    assert_eq!(
        h.service
            .notarize(&Statement::Grant(second.clone()), 10 * DAY + 9)
            .unwrap(),
        noted
    );
    // Submitted again, it is still one proof, kept apart from stamp proofs.
    assert_eq!(
        h.service
            .notarize(&Statement::Grant(second.clone()), 10 * DAY + 20)
            .unwrap(),
        noted
    );
    let proofs = h.service.grant_equivocations().unwrap();
    assert_eq!(
        proofs,
        vec![GrantEquivocation {
            first: first.clone(),
            second
        }]
    );
    assert_eq!(proofs[0].verify(), Ok(server().account()));
    assert!(h.service.equivocations().unwrap().is_empty());
    // A tampered grant is refused and takes no serial.
    let mut tampered = grant([3; 20], 4, 30 * DAY);
    tampered.count = 101;
    assert_eq!(
        h.service
            .notarize(&Statement::Grant(tampered), 10 * DAY + 9),
        Err(Refusal::Grant)
    );
    let other = grant([3; 20], 4, 30 * DAY);
    assert_eq!(
        h.service
            .notarize(&Statement::Grant(other.clone()), 10 * DAY + 10)
            .unwrap()
            .first,
        Statement::Grant(other)
    );
    assert_eq!(h.service.notary_record(&key).unwrap(), Some(noted));
}

#[test]
fn notary_records_are_kept_while_they_can_matter() {
    // A slot's record lasts while its book can sign; a grant's until it
    // expires.
    let mut h = ready_until(50 * DAY);
    let stamp = stamped_in(&MAILBOX, 10, b"one", 0);
    let paid = grant([1; 20], 3, 40 * DAY);
    h.service
        .notarize(&Statement::Ticket(stamp.clone()), 10 * DAY)
        .unwrap();
    h.service
        .notarize(&Statement::Grant(paid.clone()), 10 * DAY)
        .unwrap();
    h.service.collect(40 * DAY - 1).unwrap();
    assert!(h.service.notary_record(&paid.id()).unwrap().is_some());
    h.service.collect(40 * DAY).unwrap();
    assert_eq!(h.service.notary_record(&paid.id()).unwrap(), None);
    let key = stamp.ticket_id(&DOMAIN);
    h.service.collect(50 * DAY - 1).unwrap();
    assert!(h.service.notary_record(&key).unwrap().is_some());
    h.service.collect(50 * DAY).unwrap();
    assert_eq!(h.service.notary_record(&key).unwrap(), None);
}

#[tokio::test]
async fn statements_are_notarized_in_batches_over_the_mailbox_protocol() {
    let (client_directory, holder_directory) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut client = test_support::runtime(client_directory.path());
    let mut holder = test_support::runtime(holder_directory.path());
    holder.mailbox_holder.set_unit(UNIT);
    holder
        .mailbox_holder
        .learn_book(BOOK, terms(u64::MAX))
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(10),
        test_support::connect(&mut client, &mut holder, false),
    )
    .await
    .unwrap();
    let first = stamped_in(&MAILBOX, 10, b"one", 0);
    let second = stamped_in(&MAILBOX, 10, b"two", 0);
    let other = stamped_in(&MAILBOX, 10, b"three", 1);
    let later = stamped_in(&MAILBOX, 10, b"four", 3);
    let paid = grant([1; 20], 3, u64::MAX);
    let mut bad = StampWire::from(&first);
    bad.book = vec![0xb0; 31];
    let stranger = BookKey::from_bytes(&[4; 32]).unwrap();
    let forged = Stamp::sign(&DOMAIN, BOOK, 2, operation(&MAILBOX, 10, b"x"), &stranger);
    let unknown = Stamp::sign(
        &DOMAIN,
        [0xbb; 32],
        1,
        operation(&MAILBOX, 10, b"z"),
        &book_key(),
    );
    let mut tampered = grant([3; 20], 4, u64::MAX);
    tampered.count = 101;
    let ticket = |stamp: &Stamp| StatementWire::Ticket {
        stamp: StampWire::from(stamp),
    };
    let ask = |statements: Vec<StatementWire>| Request::Notarize { statements };
    // One request carries many statements; each gets its own answer, in
    // order, and a bad one spoils none of the others. They are taken one
    // after another: once the book is proven to spend a slot twice, a new
    // slot of it is refused.
    let Response::Notarized { answers } = exchange(
        &mut client,
        &mut holder,
        ask(vec![
            ticket(&first),
            StatementWire::Ticket { stamp: bad },
            ticket(&forged),
            ticket(&other),
            ticket(&second),
            ticket(&unknown),
            StatementWire::Grant {
                grant: paid.clone(),
            },
            StatementWire::Grant { grant: tampered },
            ticket(&later),
        ]),
    )
    .await
    .unwrap() else {
        panic!("notarize");
    };
    let refused = |code: &str| AnswerWire::Refused { code: code.into() };
    let AnswerWire::Noted { first_seen, .. } = answers[0].clone() else {
        panic!("first");
    };
    assert!(first_seen > 0);
    let noted = |statement: StatementWire| AnswerWire::Noted {
        first: statement,
        first_seen,
    };
    assert_eq!(
        answers,
        vec![
            noted(ticket(&first)),
            refused("malformed"),
            refused("stamp_signer"),
            noted(ticket(&other)),
            // The slot's first stamp answers the second.
            noted(ticket(&first)),
            refused("unknown_book"),
            noted(StatementWire::Grant { grant: paid }),
            refused("grant"),
            refused("blocked"),
        ]
    );
    // The second spend of the slot was kept as proof.
    assert_eq!(holder.mailbox_holder.equivocations().unwrap().len(), 1);
    // A batch is never empty and never longer than its bound.
    for statements in [vec![], vec![ticket(&first); MAX_NOTARIZE + 1]] {
        assert_eq!(
            exchange(&mut client, &mut holder, ask(statements))
                .await
                .unwrap(),
            Response::Refused {
                code: "malformed".into()
            }
        );
    }
    let Response::Notarized { answers } = exchange(
        &mut client,
        &mut holder,
        ask(vec![ticket(&first); MAX_NOTARIZE]),
    )
    .await
    .unwrap() else {
        panic!("full batch");
    };
    assert_eq!(answers, vec![noted(ticket(&first)); MAX_NOTARIZE]);
}

// --- access passes -----------------------------------------------------------

use agentic_mailbox_swarm::access::AccessPass;

/// The transport key of the peer a pass is shown by.
const PEER_KEY: [u8; 32] = [0x9e; 32];

#[test]
fn a_pass_lets_its_own_peer_in_until_the_end_of_its_day_within_the_book() {
    let h = ready_until(u64::MAX);
    let now = 10 * DAY + 5;
    let t = terms(u64::MAX);
    let pass = AccessPass::sign(&DOMAIN, BOOK, PEER_KEY, 10, &book_key());
    // An hour past its day absorbs clock skew; the book's end comes first.
    assert_eq!(
        h.service.check_access(&pass, &PEER_KEY, &t, now),
        Ok(11 * DAY + 3_600)
    );
    let short = terms(10 * DAY + 100);
    assert_eq!(
        h.service.check_access(&pass, &PEER_KEY, &short, now),
        Ok(10 * DAY + 100)
    );
    // Shown by another peer than the one it names; a signature or day that
    // does not hold is refused the same way (`agentic_mailbox_swarm::access`).
    assert_eq!(
        h.service.check_access(&pass, &[0x9f; 32], &t, now),
        Err(Refusal::Pass)
    );
    let foreign = BookKey::from_bytes(&[4; 32]).unwrap();
    assert_eq!(
        h.service.check_access(
            &AccessPass::sign(&DOMAIN, BOOK, PEER_KEY, 10, &foreign),
            &PEER_KEY,
            &t,
            now
        ),
        Err(Refusal::Pass)
    );
    assert_eq!(
        h.service.check_access(&pass, &PEER_KEY, &terms(now), now),
        Err(Refusal::BookExpired)
    );
}

#[test]
fn a_proven_book_lets_nobody_in() {
    let mut h = ready_until(u64::MAX);
    let now = 10 * DAY + 5;
    let pass = AccessPass::sign(&DOMAIN, BOOK, PEER_KEY, 10, &book_key());
    let t = terms(u64::MAX);
    assert!(h.service.check_access(&pass, &PEER_KEY, &t, now).is_ok());
    let first = stamped_in(&MAILBOX, 10, b"one", 0);
    h.service.store(MAILBOX, 10, b"one", &first, now).unwrap();
    let second = stamped_in(&OTHER_MAILBOX, 10, b"two", 0);
    assert_eq!(
        h.service.store(OTHER_MAILBOX, 10, b"two", &second, now),
        Err(Refusal::Conflict)
    );
    assert_eq!(
        h.service.check_access(&pass, &PEER_KEY, &t, now),
        Err(Refusal::Blocked)
    );
}

// --- proofs and blocking -----------------------------------------------------

use agentic_mailbox_swarm::proof::HolderEquivocation;

#[test]
fn a_proven_book_is_refused_from_then_on() {
    let mut h = ready_until(u64::MAX);
    const OTHER_BOOK: [u8; 32] = [0xb2; 32];
    h.service.learn_book(OTHER_BOOK, terms(u64::MAX)).unwrap();
    let now = 10 * DAY + 5;
    let first = stamped_in(&MAILBOX, 10, b"one", 0);
    h.service.store(MAILBOX, 10, b"one", &first, now).unwrap();
    // A slot on record with this notary before any proof.
    let noted = stamped_in(&[0xc6; 32], 10, b"noted", 5);
    h.service
        .notarize(&Statement::Ticket(noted.clone()), now)
        .unwrap();
    assert!(h.service.blocked_books().unwrap().is_empty());
    assert_eq!(
        h.service.store(
            OTHER_MAILBOX,
            10,
            b"two",
            &stamped_in(&OTHER_MAILBOX, 10, b"two", 0),
            now
        ),
        Err(Refusal::Conflict)
    );
    assert_eq!(h.service.blocked_books().unwrap(), BTreeSet::from([BOOK]));
    // Nothing new of the book is taken: no fresh entry, no new notary key.
    let later = [0xc5; 32];
    let next = stamped_in(&later, 10, b"next", 1);
    assert_eq!(
        h.service.store(later, 10, b"next", &next, now),
        Err(Refusal::Blocked)
    );
    assert_eq!(
        h.service.notarize(&Statement::Ticket(next.clone()), now),
        Err(Refusal::Blocked)
    );
    // A notary still answers for a slot it already holds.
    assert_eq!(
        h.service
            .notarize(
                &Statement::Ticket(stamped_in(&[0xc6; 32], 10, b"other", 5)),
                now
            )
            .unwrap()
            .first,
        Statement::Ticket(noted)
    );
    // What it holds stays, and copies of what the book paid for are still
    // repaired: a verified replica is taken.
    assert_eq!(
        h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries.len(),
        1
    );
    let copy = [0xc7; 32];
    let paid = stamped_in(&copy, 10, b"copy", 2);
    h.service
        .store_replica(copy, 10, b"copy", &paid, now)
        .unwrap();
    // Only copies of periods written before the proof was known: a replica
    // of the next period still counts, a later one is new spending.
    let replicas = |h: &mut Holder| {
        let (next, after) = ([0xc8; 32], [0xc9; 32]);
        (
            h.service.store_replica(
                next,
                11,
                b"next day",
                &stamped_in(&next, 11, b"next day", 3),
                11 * DAY + 5,
            ),
            h.service.store_replica(
                after,
                12,
                b"day after",
                &stamped_in(&after, 12, b"day after", 4),
                12 * DAY + 5,
            ),
        )
    };
    let (next_day, day_after) = replicas(&mut h);
    assert!(next_day.is_ok(), "{next_day:?}");
    assert_eq!(day_after, Err(Refusal::Blocked));
    // Replicas still meet the slot rules after the proof.
    assert_eq!(
        h.service.store_replica(
            copy,
            10,
            b"other copy",
            &stamped_in(&copy, 10, b"other copy", 2),
            now
        ),
        Err(Refusal::Conflict)
    );
    let stranger = BookKey::from_bytes(&[4; 32]).unwrap();
    let forged = Stamp::sign(&DOMAIN, BOOK, 7, operation(&copy, 10, b"forged"), &stranger);
    assert_eq!(
        h.service.store_replica(copy, 10, b"forged", &forged, now),
        Err(Refusal::Stamp(StampError::Signer))
    );
    assert_eq!(
        h.service
            .proofs(0, MAX_PROOFS)
            .unwrap()
            .proofs
            .senders
            .len(),
        1
    );
    // Another book is untouched.
    let fine = Stamp::sign(
        &DOMAIN,
        OTHER_BOOK,
        0,
        operation(&later, 10, b"fine"),
        &book_key(),
    );
    h.service.store(later, 10, b"fine", &fine, now).unwrap();
    let mut h = reopen(h);
    h.service.learn_book(BOOK, terms(u64::MAX)).unwrap();
    assert_eq!(h.service.blocked_books().unwrap(), BTreeSet::from([BOOK]));
    assert_eq!(
        h.service.store(later, 10, b"next", &next, now),
        Err(Refusal::Blocked)
    );
    // When the proof was learned survives too.
    let after = [0xca; 32];
    assert_eq!(
        h.service.store_replica(
            after,
            12,
            b"still after",
            &stamped_in(&after, 12, b"still after", 8),
            12 * DAY + 5
        ),
        Err(Refusal::Blocked)
    );
    // Spending twice again later does not move that moment forward.
    let next = [0xc8; 32];
    assert_eq!(
        h.service.store_replica(
            next,
            11,
            b"again",
            &stamped_in(&next, 11, b"again", 3),
            13 * DAY + 5
        ),
        Err(Refusal::Conflict)
    );
    let later_still = [0xcb; 32];
    assert_eq!(
        h.service.store_replica(
            later_still,
            13,
            b"later still",
            &stamped_in(&later_still, 13, b"later still", 9),
            13 * DAY + 5
        ),
        Err(Refusal::Blocked)
    );
    assert_eq!(Refusal::Blocked.code(), "blocked");
}

#[test]
fn a_notary_conflict_blocks_the_book_and_is_passed_on() {
    let mut h = ready_until(u64::MAX);
    let now = 10 * DAY + 5;
    let first = stamped_in(&MAILBOX, 10, b"one", 0);
    let second = stamped_in(&OTHER_MAILBOX, 10, b"two", 0);
    h.service
        .notarize(&Statement::Ticket(first.clone()), now)
        .unwrap();
    h.service
        .notarize(&Statement::Ticket(second.clone()), now)
        .unwrap();
    assert_eq!(h.service.blocked_books().unwrap(), BTreeSet::from([BOOK]));
    assert_eq!(
        h.service.proofs(0, MAX_PROOFS).unwrap().proofs.senders,
        vec![SenderEquivocation { first, second }]
    );
}

fn holder_receipt(key: &HolderKey, ticket: [u8; 32], op: u8) -> Receipt {
    Receipt::sign(&DOMAIN, MAILBOX, [op; 32], ticket, UNIT, 1_000, key)
}

#[test]
fn proofs_are_accepted_once_and_only_when_they_verify() {
    let mut h = ready_until(u64::MAX);
    let sender = SenderEquivocation {
        first: stamped_in(&MAILBOX, 10, b"one", 3),
        second: stamped_in(&MAILBOX, 10, b"two", 3),
    };
    let stranger = BookKey::from_bytes(&[4; 32]).unwrap();
    let forged = SenderEquivocation {
        first: stamped_in(&MAILBOX, 10, b"one", 4),
        second: Stamp::sign(&DOMAIN, BOOK, 4, operation(&MAILBOX, 10, b"x"), &stranger),
    };
    let unknown = SenderEquivocation {
        first: Stamp::sign(
            &DOMAIN,
            [0xbb; 32],
            0,
            operation(&MAILBOX, 10, b"a"),
            &book_key(),
        ),
        second: Stamp::sign(
            &DOMAIN,
            [0xbb; 32],
            0,
            operation(&MAILBOX, 10, b"b"),
            &book_key(),
        ),
    };
    let lazy = HolderKey::from_bytes(&[0x51; 32]).unwrap();
    let holder = HolderEquivocation {
        first: holder_receipt(&lazy, [0x70; 32], 1),
        second: holder_receipt(&lazy, [0x70; 32], 2),
    };
    let same = HolderEquivocation {
        first: holder_receipt(&lazy, [0x71; 32], 1),
        second: holder_receipt(&lazy, [0x71; 32], 1),
    };
    let issuer = GrantEquivocation {
        first: grant([1; 20], 3, 30 * DAY),
        second: grant([2; 20], 3, 30 * DAY),
    };
    let not_issuer = GrantEquivocation {
        first: grant([1; 20], 5, 30 * DAY),
        second: grant([1; 20], 6, 30 * DAY),
    };
    // Public receipts and grants cannot frame anyone: the second half of
    // each of these is not what the accused signed.
    let honest = HolderKey::from_bytes(&[0x52; 32]).unwrap();
    let framed = HolderEquivocation {
        first: holder_receipt(&honest, [0x72; 32], 1),
        second: holder_receipt(&lazy, [0x72; 32], 2),
    };
    let mut altered = grant([2; 20], 7, 30 * DAY);
    altered.count = 50;
    let framed_issuer = GrantEquivocation {
        first: grant([1; 20], 7, 30 * DAY),
        second: altered,
    };
    // One proof per offender is enough: more against the same one are not
    // kept.
    let again_holder = HolderEquivocation {
        first: holder_receipt(&lazy, [0x73; 32], 1),
        second: holder_receipt(&lazy, [0x73; 32], 2),
    };
    let again_issuer = GrantEquivocation {
        first: grant([1; 20], 8, 30 * DAY),
        second: grant([2; 20], 8, 30 * DAY),
    };
    let again_sender = SenderEquivocation {
        first: stamped_in(&MAILBOX, 10, b"one", 6),
        second: stamped_in(&MAILBOX, 10, b"two", 6),
    };
    let offered = Proofs {
        senders: vec![sender.clone(), forged, unknown, again_sender],
        holders: vec![framed, holder.clone(), same, again_holder],
        grants: vec![framed_issuer, issuer.clone(), not_issuer, again_issuer],
    };
    let accepted = h.service.accept_proofs(&offered).unwrap();
    assert_eq!(
        accepted,
        Proofs {
            senders: vec![sender.clone()],
            holders: vec![holder.clone()],
            grants: vec![issuer.clone()],
        }
    );
    // Again: nothing is new.
    assert_eq!(
        h.service.accept_proofs(&offered).unwrap(),
        Proofs::default()
    );
    assert_eq!(h.service.blocked_books().unwrap(), BTreeSet::from([BOOK]));
    assert_eq!(
        h.service.blocked_holders().unwrap(),
        BTreeSet::from([lazy.account()])
    );
    assert!(
        !h.service
            .blocked_holders()
            .unwrap()
            .contains(&honest.account())
    );
    assert_eq!(
        h.service.blocked_issuers().unwrap(),
        BTreeSet::from([server().account()])
    );
    let h = reopen(h);
    let kept = h.service.proofs(0, 64).unwrap().proofs;
    assert_eq!(
        (kept.senders, kept.holders, kept.grants),
        (vec![sender], vec![holder], vec![issuer])
    );
}

#[tokio::test]
async fn proofs_are_listed_over_the_mailbox_protocol() {
    let (client_directory, holder_directory) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut client = test_support::runtime(client_directory.path());
    let mut holder = test_support::runtime(holder_directory.path());
    holder.mailbox_holder.set_unit(UNIT);
    holder
        .mailbox_holder
        .learn_book(BOOK, terms(u64::MAX))
        .unwrap();
    // Seventy holders, each proven once.
    let many: Vec<HolderEquivocation> = (1..=70u8)
        .map(|n| {
            let key = HolderKey::from_bytes(&[n; 32]).unwrap();
            HolderEquivocation {
                first: holder_receipt(&key, [n; 32], 1),
                second: holder_receipt(&key, [n; 32], 2),
            }
        })
        .collect();
    let sender = SenderEquivocation {
        first: stamped_in(&MAILBOX, 10, b"one", 3),
        second: stamped_in(&MAILBOX, 10, b"two", 3),
    };
    holder
        .mailbox_holder
        .accept_proofs(&Proofs {
            senders: vec![sender.clone()],
            holders: many.clone(),
            grants: vec![],
        })
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(10),
        test_support::connect(&mut client, &mut holder, false),
    )
    .await
    .unwrap();
    // Paged by the order the holder learned them: everything arrives once.
    let mut after = 0;
    let mut listed = Proofs::default();
    let mut pages = 0;
    loop {
        let Response::Proofs { proofs, next } =
            exchange(&mut client, &mut holder, Request::Proofs { after })
                .await
                .unwrap()
        else {
            panic!("proofs");
        };
        pages += 1;
        let page = Proofs::try_from(&proofs).unwrap();
        assert!(page.senders.len() + page.holders.len() + page.grants.len() <= MAX_PROOFS);
        listed.senders.extend(page.senders);
        listed.holders.extend(page.holders);
        listed.grants.extend(page.grants);
        match next {
            Some(next) => after = next,
            None => break,
        }
        assert!(pages < 10);
    }
    assert!(pages >= 2);
    assert_eq!(listed.senders, vec![sender]);
    assert_eq!(listed.holders, many);
    // A cursor past the end is an empty last page.
    let Response::Proofs { proofs, next } =
        exchange(&mut client, &mut holder, Request::Proofs { after: 1_000 })
            .await
            .unwrap()
    else {
        panic!("proofs");
    };
    assert_eq!(Proofs::try_from(&proofs).unwrap(), Proofs::default());
    assert_eq!(next, None);
}

// --- grant books -----------------------------------------------------------------

/// What the chain's `GrantIssuer` answered on `today` about `server` on each
/// of `days`: active on `from ≤ day < until`, 300 coins a day, books of 100,
/// 30 days.
fn read_rules(
    service: &mut Service,
    server: &SecpKey,
    (from, until): (u64, u64),
    days: std::ops::RangeInclusive<u64>,
    today: u64,
) {
    for day in days {
        service.learn_grant_day(
            server.account(),
            day,
            crate::runtime::chain::GrantDay {
                active: from <= day && day < until,
                cap_coins: 300,
                book_size: 100,
                max_validity_days: 30,
                today,
            },
        );
    }
}

fn granted(
    by: &SecpKey,
    domain: [u8; 32],
    book: [u8; 20],
    day: u64,
    serial: u32,
    count: u32,
    expiry: u64,
) -> GrantBook {
    GrantBook::issue(
        GrantTerms {
            domain,
            book,
            day,
            serial,
            count,
            expiry,
        },
        by,
    )
}

#[test]
fn a_grant_is_checked_against_the_rules_of_its_day() {
    let mut h = ready();
    // The issuer may sign on days 10 and 11 only.
    read_rules(&mut h.service, &server(), (10, 12), 0..=13, 12);
    let book = BookKey::from_bytes(&[0x41; 32]).unwrap().account();
    let now = 12 * DAY + 5;
    let s = server();
    let ok = granted(&s, DOMAIN, book, 11, 2, 100, 25 * DAY);
    assert_eq!(h.service.check_grant(&ok, now), Ok(()));
    // Each day has the cap the chain gives for it: day 10's is smaller.
    h.service.learn_grant_day(
        s.account(),
        10,
        crate::runtime::chain::GrantDay {
            active: true,
            cap_coins: 200,
            book_size: 100,
            max_validity_days: 30,
            today: 12,
        },
    );
    assert_eq!(
        h.service
            .check_grant(&granted(&s, DOMAIN, book, 10, 2, 100, 25 * DAY), now),
        Err(Refusal::Grant)
    );
    assert_eq!(h.service.check_grant(&ok, now), Ok(()));
    // An issuer whose rules for the day were not read yet is neither
    // taken nor refused.
    let unread = SecpKey::from_secret(&[0x34; 32]).unwrap();
    assert_eq!(
        h.service
            .check_grant(&granted(&unread, DOMAIN, book, 11, 0, 100, 25 * DAY), now),
        Err(Refusal::GrantPending)
    );
    let mut altered = ok.clone();
    altered.count = 99;
    // The chain knows no such issuer: inactive every day.
    let stranger = SecpKey::from_secret(&[0x32; 32]).unwrap();
    read_rules(&mut h.service, &stranger, (0, 0), 0..=13, 12);
    for (label, bad) in [
        (
            "beyond the daily cap",
            granted(&s, DOMAIN, book, 11, 3, 100, 25 * DAY),
        ),
        (
            "before the issuer's days",
            granted(&s, DOMAIN, book, 9, 0, 100, 25 * DAY),
        ),
        (
            "after the issuer's days",
            granted(&s, DOMAIN, book, 12, 0, 100, 25 * DAY),
        ),
        (
            "an unknown issuer",
            granted(&stranger, DOMAIN, book, 11, 0, 100, 25 * DAY),
        ),
        (
            "another book size",
            granted(&s, DOMAIN, book, 11, 0, 99, 25 * DAY),
        ),
        (
            "valid too long",
            granted(&s, DOMAIN, book, 11, 0, 100, (11 + 1 + 30) * DAY + 1),
        ),
        ("expired", granted(&s, DOMAIN, book, 11, 0, 100, now)),
        (
            "another network",
            granted(&s, [0x12; 32], book, 11, 0, 100, 25 * DAY),
        ),
        ("altered after signing", altered),
    ] {
        assert_eq!(
            h.service.check_grant(&bad, now),
            Err(Refusal::Grant),
            "{label}"
        );
    }
    // A grant dated ahead is refused even from an issuer active that day.
    read_rules(&mut h.service, &server(), (10, 14), 0..=13, 12);
    assert_eq!(
        h.service
            .check_grant(&granted(&s, DOMAIN, book, 13, 0, 100, 25 * DAY), now),
        Err(Refusal::Grant)
    );
    // An issuer proven to grant one serial of day 10 twice grants nothing
    // from day 11 on; what it granted up to that day stands.
    let proof = GrantEquivocation {
        first: granted(&s, DOMAIN, book, 10, 1, 100, 25 * DAY),
        second: granted(&s, DOMAIN, [7; 20], 10, 1, 100, 25 * DAY),
    };
    h.service
        .accept_proofs(&Proofs {
            grants: vec![proof],
            ..Proofs::default()
        })
        .unwrap();
    assert_eq!(h.service.check_grant(&ok, now), Err(Refusal::Blocked));
    let before = granted(&s, DOMAIN, book, 10, 0, 100, 25 * DAY);
    assert_eq!(h.service.check_grant(&before, now), Ok(()));
    // Another issuer is not affected.
    let other = SecpKey::from_secret(&[0x33; 32]).unwrap();
    read_rules(&mut h.service, &other, (10, 14), 0..=13, 12);
    assert_eq!(
        h.service
            .check_grant(&granted(&other, DOMAIN, book, 11, 0, 100, 25 * DAY), now),
        Ok(())
    );
}

#[test]
fn a_grant_book_is_learned_only_when_first_seen_on_its_day() {
    let mut h = ready();
    read_rules(&mut h.service, &server(), (0, u64::MAX), 0..=11, 11);
    let key = BookKey::from_bytes(&[0x41; 32]).unwrap();
    let grant = granted(&server(), DOMAIN, key.account(), 10, 0, 100, 25 * DAY);
    let book = grant.id();
    let now = 11 * DAY + 3_600;
    let paid = |envelope: &[u8], index: u32| {
        Stamp::sign(
            &DOMAIN,
            book,
            index,
            operation(&MAILBOX, 10, envelope),
            &key,
        )
    };
    // First seen past its day (beyond the tolerance): a grant issued
    // after the fact, never learned.
    assert_eq!(
        h.service.learn_grant(&grant, 11 * DAY + 3_600, now),
        Err(Refusal::Grant)
    );
    assert_eq!(
        h.service
            .store(MAILBOX, 10, b"early", &paid(b"early", 0), now),
        Err(Refusal::UnknownBook)
    );
    // First seen on its day, within the tolerance on either side: its
    // key's stamps pay, up to its size.
    for first_seen in [10 * DAY - 3_599, 11 * DAY + 3_599] {
        h.service.learn_grant(&grant, first_seen, now).unwrap();
    }
    h.service
        .store(MAILBOX, 10, b"paid", &paid(b"paid", 0), now)
        .unwrap();
    assert_eq!(
        h.service
            .store(MAILBOX, 10, b"beyond", &paid(b"beyond", 100), now),
        Err(Refusal::Stamp(StampError::Index))
    );
    // An invalid grant is refused whatever the notaries say.
    let over = granted(&server(), DOMAIN, key.account(), 10, 3, 100, 25 * DAY);
    assert_eq!(
        h.service.learn_grant(&over, 10 * DAY + 7, now),
        Err(Refusal::Grant)
    );
    // Learned books survive a restart.
    let mut h = reopen(h);
    h.service
        .store(MAILBOX, 10, b"again", &paid(b"again", 1), now)
        .unwrap();
    assert_eq!(h.service.granted(&book).unwrap(), Some(grant.clone()));
    // The book lasts as long as its grant.
    let at = |period: u64, envelope: &[u8], index: u32| {
        Stamp::sign(
            &DOMAIN,
            book,
            index,
            operation(&OTHER_MAILBOX, period, envelope),
            &key,
        )
    };
    h.service
        .store(
            OTHER_MAILBOX,
            24,
            b"last",
            &at(24, b"last", 2),
            25 * DAY - 1,
        )
        .unwrap();
    let late = [0xa3; 32];
    assert_eq!(
        h.service.store(
            late,
            25,
            b"over",
            &Stamp::sign(&DOMAIN, book, 3, operation(&late, 25, b"over"), &key),
            25 * DAY + 5
        ),
        Err(Refusal::Stamp(StampError::Expired))
    );
}

#[tokio::test]
async fn grants_are_offered_over_the_mailbox_protocol() {
    let (client_directory, holder_directory) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut client = test_support::runtime(client_directory.path());
    let mut holder = test_support::runtime(holder_directory.path());
    holder.mailbox_holder.set_unit(UNIT);
    let today = clock::wall().unwrap() / DAY;
    read_rules(
        &mut holder.mailbox_holder,
        &server(),
        (0, u64::MAX),
        today - 2..=today,
        today,
    );
    // One notary, which does not answer yet.
    holder.mailbox_client.learn_holder(
        [0x76; 32],
        mailbox_client::Holder {
            peer: PeerId::random(),
            addresses: vec![],
            account: [0x76; 20],
        },
    );
    tokio::time::timeout(
        Duration::from_secs(10),
        test_support::connect(&mut client, &mut holder, false),
    )
    .await
    .unwrap();
    let today = agentic_mailbox_swarm::address::period(clock::wall().unwrap());
    let book = BookKey::from_bytes(&[0x41; 32]).unwrap().account();
    let good = granted(&server(), DOMAIN, book, today, 0, 100, (today + 20) * DAY);
    let bad = granted(&server(), DOMAIN, book, today, 3, 100, (today + 20) * DAY);
    let offer = |grant: GrantBook| Request::LearnGrant { grant };
    assert_eq!(
        exchange(&mut client, &mut holder, offer(bad))
            .await
            .unwrap(),
        Response::Refused {
            code: "grant".into()
        }
    );
    // A valid grant waits for its notaries.
    assert_eq!(
        exchange(&mut client, &mut holder, offer(good.clone()))
            .await
            .unwrap(),
        Response::Refused {
            code: "grant_pending".into()
        }
    );
    // Once learned, it is acknowledged at once.
    let now = clock::wall().unwrap();
    holder.mailbox_holder.learn_grant(&good, now, now).unwrap();
    assert_eq!(
        exchange(&mut client, &mut holder, offer(good))
            .await
            .unwrap(),
        Response::Learned
    );
    // An issuer proven to grant a serial of yesterday twice grants nothing
    // today.
    let proof = GrantEquivocation {
        first: granted(
            &server(),
            DOMAIN,
            book,
            today - 1,
            0,
            100,
            (today + 20) * DAY,
        ),
        second: granted(
            &server(),
            DOMAIN,
            [7; 20],
            today - 1,
            0,
            100,
            (today + 20) * DAY,
        ),
    };
    holder
        .mailbox_holder
        .accept_proofs(&Proofs {
            grants: vec![proof],
            ..Proofs::default()
        })
        .unwrap();
    let blocked = granted(&server(), DOMAIN, book, today, 1, 100, (today + 20) * DAY);
    assert_eq!(
        exchange(&mut client, &mut holder, offer(blocked))
            .await
            .unwrap(),
        Response::Refused {
            code: "blocked".into()
        }
    );
}

#[test]
fn a_grant_is_decided_by_most_of_its_notaries_at_least_four() {
    use super::GrantVerdict::{Learn, Refuse, Retry};
    use super::grant_verdict as verdict;
    // (in time, late, outstanding, unreachable, conflict)
    for (answers, decided) in [
        ((4, 0, 0, 0, false), Some(Learn)),
        ((10, 0, 0, 0, false), Some(Learn)),
        ((4, 3, 0, 0, false), Some(Learn)),
        // Early: even if every outstanding notary said late.
        ((6, 3, 1, 0, false), Some(Learn)),
        ((6, 1, 0, 2, false), Some(Learn)),
        ((3, 0, 0, 0, false), Some(Refuse)),
        ((4, 4, 0, 0, false), Some(Refuse)),
        ((1, 6, 3, 0, false), Some(Refuse)),
        ((2, 0, 1, 0, false), Some(Refuse)),
        ((0, 0, 0, 0, false), Some(Refuse)),
        // Late even if every unreachable notary had said in time.
        ((2, 5, 0, 3, false), Some(Refuse)),
        ((1, 6, 2, 1, false), Some(Refuse)),
        // Another grant first on record for its serial.
        ((9, 0, 0, 0, true), Some(Refuse)),
        // Unreachable notaries could still tip it: ask again later.
        ((3, 0, 0, 1, false), Some(Retry)),
        ((5, 1, 0, 4, false), Some(Retry)),
        ((5, 0, 0, 5, false), Some(Retry)),
        ((0, 0, 0, 10, false), Some(Retry)),
        ((5, 4, 1, 0, false), None),
        ((0, 0, 10, 0, false), None),
        ((3, 0, 1, 0, false), None),
        ((3, 0, 1, 2, false), None),
    ] {
        let (in_time, late, outstanding, unreachable, conflict) = answers;
        assert_eq!(
            verdict(in_time, late, outstanding, unreachable, conflict),
            decided,
            "{answers:?}"
        );
    }
}

#[test]
fn an_issuer_is_blocked_after_the_earliest_day_proven_against_it() {
    let mut h = ready();
    read_rules(&mut h.service, &server(), (0, u64::MAX), 0..=20, 20);
    let s = server();
    let book = BookKey::from_bytes(&[0x41; 32]).unwrap().account();
    let twice = |day: u64| GrantEquivocation {
        first: granted(&s, DOMAIN, book, day, 0, 100, 35 * DAY),
        second: granted(&s, DOMAIN, [7; 20], day, 0, 100, 35 * DAY),
    };
    let on = |day: u64| granted(&s, DOMAIN, book, day, 1, 100, 35 * DAY);
    let now = 20 * DAY;
    let accept = |h: &mut Holder, proof: GrantEquivocation| {
        h.service
            .accept_proofs(&Proofs {
                grants: vec![proof],
                ..Proofs::default()
            })
            .unwrap()
            .grants
    };
    assert_eq!(accept(&mut h, twice(12)), vec![twice(12)]);
    assert_eq!(h.service.check_grant(&on(12), now), Ok(()));
    assert_eq!(h.service.check_grant(&on(13), now), Err(Refusal::Blocked));
    // An earlier day proven later moves the block back, and is passed on
    // so every node ends at the same day.
    assert_eq!(accept(&mut h, twice(9)), vec![twice(9)]);
    assert_eq!(h.service.check_grant(&on(9), now), Ok(()));
    assert_eq!(h.service.check_grant(&on(10), now), Err(Refusal::Blocked));
    // A later day changes nothing and is not passed on.
    assert!(accept(&mut h, twice(11)).is_empty());
    assert_eq!(h.service.check_grant(&on(10), now), Err(Refusal::Blocked));
    assert_eq!(h.service.grant_equivocations().unwrap(), vec![twice(9)]);
    let logged = h.service.proofs(0, MAX_PROOFS).unwrap().proofs.grants;
    assert_eq!(logged, vec![twice(12), twice(9)]);
    // A notary that sees a serial of an even earlier day granted twice
    // keeps that one the same way.
    let first = granted(&s, DOMAIN, book, 7, 0, 100, 35 * DAY);
    let second = granted(&s, DOMAIN, [7; 20], 7, 0, 100, 35 * DAY);
    for grant in [&first, &second] {
        h.service
            .notarize(&Statement::Grant(grant.clone()), now)
            .unwrap();
    }
    assert_eq!(h.service.check_grant(&on(8), now), Err(Refusal::Blocked));
    let earliest = GrantEquivocation { first, second };
    assert_eq!(
        h.service.grant_equivocations().unwrap(),
        vec![earliest.clone()]
    );
    // It is passed on too.
    assert_eq!(
        h.service.proofs(0, MAX_PROOFS).unwrap().proofs.grants,
        vec![twice(12), twice(9), earliest]
    );
    // It holds after a restart.
    let mut h = reopen(h);
    read_rules(&mut h.service, &server(), (0, u64::MAX), 0..=20, 20);
    assert_eq!(h.service.check_grant(&on(7), now), Ok(()));
    assert_eq!(h.service.check_grant(&on(8), now), Err(Refusal::Blocked));
}

#[tokio::test]
async fn a_page_carries_the_grants_of_its_books_once() {
    let (client_directory, holder_directory) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut client = test_support::runtime(client_directory.path());
    let mut holder = test_support::runtime(holder_directory.path());
    holder.mailbox_holder.set_unit(UNIT);
    holder
        .mailbox_holder
        .learn_book(BOOK, terms(u64::MAX))
        .unwrap();
    let today = clock::wall().unwrap() / DAY;
    read_rules(
        &mut holder.mailbox_holder,
        &server(),
        (0, u64::MAX),
        today - 2..=today,
        today,
    );
    tokio::time::timeout(
        Duration::from_secs(10),
        test_support::connect(&mut client, &mut holder, false),
    )
    .await
    .unwrap();
    let now = clock::wall().unwrap();
    let today = agentic_mailbox_swarm::address::period(now);
    let key = BookKey::from_bytes(&[0x41; 32]).unwrap();
    let grant = granted(
        &server(),
        DOMAIN,
        key.account(),
        today,
        0,
        100,
        (today + 20) * DAY,
    );
    holder.mailbox_holder.learn_grant(&grant, now, now).unwrap();
    let other_key = BookKey::from_bytes(&[0x42; 32]).unwrap();
    let other = granted(
        &server(),
        DOMAIN,
        other_key.account(),
        today,
        1,
        100,
        (today + 20) * DAY,
    );
    holder.mailbox_holder.learn_grant(&other, now, now).unwrap();
    for (grant, key, index, envelope) in [
        (&grant, &key, 0, b"one"),
        (&other, &other_key, 0, b"oth"),
        (&grant, &key, 1, b"two"),
    ] {
        let stamp = Stamp::sign(
            &DOMAIN,
            grant.id(),
            index,
            operation(&MAILBOX, today, envelope),
            key,
        );
        holder
            .mailbox_holder
            .store(MAILBOX, today, envelope, &stamp, now)
            .unwrap();
    }
    let bought = stamped_in(&MAILBOX, today, b"bought", 0);
    holder
        .mailbox_holder
        .store(MAILBOX, today, b"bought", &bought, now)
        .unwrap();
    let read = |after: u64, limit: u16| Request::Read {
        mailbox: MAILBOX.to_vec(),
        after,
        limit,
    };
    let Response::Page {
        entries, grants, ..
    } = exchange(&mut client, &mut holder, read(0, 10))
        .await
        .unwrap()
    else {
        panic!("read");
    };
    // Each grant once, in the order its book first appears.
    assert_eq!(entries.len(), 4);
    assert_eq!(grants, vec![grant, other]);
    // Only the grants of the page's own entries.
    let Response::Page {
        entries, grants, ..
    } = exchange(&mut client, &mut holder, read(3, 10))
        .await
        .unwrap()
    else {
        panic!("read");
    };
    assert_eq!(entries.len(), 1);
    assert!(grants.is_empty());
}

/// A group commit claim (spec/groups-v1.md) is recorded like any statement:
/// the first at its key wins, a rival claim is a race, not an equivocation,
/// and only the owner's roster lets a claimer in.
#[test]
fn a_notary_records_a_groups_first_commit_claim_under_its_owners_roster() {
    use agentic_protocol::group::{CommitClaim, Roster, group_ref};
    use agentic_protocol::{DocumentDraft, DocumentKind, SignedDocument};
    use ed25519_dalek::SigningKey;
    let mut h = holder();
    let now = 1_000;
    let owner = SigningKey::from_bytes(&[41; 32]);
    let admin = SigningKey::from_bytes(&[42; 32]);
    let member = SigningKey::from_bytes(&[43; 32]);
    let root = |k: &SigningKey| k.verifying_key().to_bytes();
    let group_id = [44; 32];
    let document = |kind, body: Vec<u8>, key: &SigningKey| {
        SignedDocument::sign(
            DocumentDraft {
                domain: DOMAIN,
                kind,
                authority_epoch: 0,
                issued_at: now,
                expires_at: None,
                body,
                extensions: std::collections::BTreeMap::new(),
            },
            key,
        )
        .unwrap()
        .to_wire()
    };
    let roster = document(
        DocumentKind::GroupRoster,
        Roster {
            group: group_ref(&DOMAIN, &root(&owner), &group_id),
            version: 1,
            admins: vec![root(&admin)],
            access: agentic_protocol::group::Access::Private,
            kind: agentic_protocol::group::GroupKind::Group,
        }
        .encode(),
        &owner,
    );
    let claim = |commit: u8, round: u32, key: &SigningKey| {
        Statement::Commit(document(
            DocumentKind::GroupCommit,
            CommitClaim {
                owner: root(&owner),
                group_id,
                epoch: 3,
                round,
                commit: [commit; 32],
                roster: roster.clone(),
            }
            .encode(),
            key,
        ))
    };
    let first = claim(1, 0, &owner);
    assert_eq!(h.service.notarize(&first, now).unwrap().first, first);
    // The admin's rival claim at the same epoch and round meets the first.
    assert_eq!(
        h.service.notarize(&claim(2, 0, &admin), now).unwrap().first,
        first
    );
    assert!(h.service.equivocations().unwrap().is_empty());
    // A plain member takes no epoch, not even with a roster of its own
    // naming itself.
    assert!(h.service.notarize(&claim(3, 0, &member), now).is_err());
    let own_roster = document(
        DocumentKind::GroupRoster,
        Roster {
            group: group_ref(&DOMAIN, &root(&owner), &group_id),
            version: 2,
            admins: vec![root(&member)],
            access: agentic_protocol::group::Access::Private,
            kind: agentic_protocol::group::GroupKind::Group,
        }
        .encode(),
        &member,
    );
    let forged = Statement::Commit(document(
        DocumentKind::GroupCommit,
        CommitClaim {
            owner: root(&owner),
            group_id,
            epoch: 3,
            round: 0,
            commit: [4; 32],
            roster: own_roster,
        }
        .encode(),
        &member,
    ));
    assert!(h.service.notarize(&forged, now).is_err());
    // Another round is another key.
    let later = claim(2, 1, &admin);
    assert_eq!(h.service.notarize(&later, now).unwrap().first, later);
    // Kept across a restart, and for longer than the epoch's mailbox lives.
    let h = reopen(h);
    let mut h = h;
    assert_eq!(
        h.service.notarize(&claim(1, 1, &owner), now).unwrap().first,
        later
    );
    let later_on = now + 32 * 86_400;
    h.service.collect(later_on).unwrap();
    assert_eq!(
        h.service
            .notarize(&claim(2, 0, &admin), later_on)
            .unwrap()
            .first,
        first
    );
}

// --- operator payouts (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md) -------------

use crate::runtime::chain::TicketClaim;
use agentic_mailbox_swarm::stamp::{named_operation, swarm_digest};

/// Ten units a sender names, this holder's third unless `without`.
fn listed(without: bool) -> Vec<[u8; 32]> {
    let mut units: Vec<[u8; 32]> = (0..10u8).map(|n| [0x40 + n; 32]).collect();
    if !without {
        units[2] = UNIT;
    }
    units
}

/// A stamp of `book` naming `holders` for `envelope` in `mailbox`, `period`.
fn named_stamp(
    book: [u8; 32],
    key: &BookKey,
    mailbox: &[u8; 32],
    period: u64,
    envelope: &[u8],
    index: u32,
    holders: &[[u8; 32]],
) -> Stamp {
    let mut stamp = Stamp::sign(
        &DOMAIN,
        book,
        index,
        named_operation(mailbox, period, &swarm_digest(holders), envelope),
        key,
    );
    stamp.holders = Some(holders.to_vec());
    stamp
}

/// A holder keeps the ticket of every paid stamp that names its unit, with
/// all the contract needs to pay it: the stamp, where it names the unit, and
/// the envelope's hash. One per slot, across a restart, fresh or copied.
#[test]
fn a_paid_stamp_naming_this_unit_is_kept_as_a_ticket_to_claim() {
    let mut h = ready();
    let holders = listed(false);
    let envelope = b"paid".to_vec();
    let stamp = named_stamp(BOOK, &book_key(), &MAILBOX, P0, &envelope, 3, &holders);
    h.service
        .store(MAILBOX, P0, &envelope, &stamp, 1_000)
        .unwrap();
    let mut list = [[0; 32]; 10];
    list.copy_from_slice(&holders);
    let expected = TicketClaim {
        book: BOOK,
        index: 3,
        mailbox: MAILBOX,
        period: P0,
        envelope: alloy_primitives::keccak256(&envelope).0,
        holders: list,
        position: 2,
        signature: stamp.signature,
    };
    let tickets = h.service.tickets().unwrap();
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].claim, expected);
    assert_eq!(tickets[0].valid_until, 5_000);
    assert_eq!(tickets[0].state, TicketState::Drawing);
    // The same stamp stored again is the same ticket.
    h.service
        .store(MAILBOX, P0, &envelope, &stamp, 1_001)
        .unwrap();
    assert_eq!(
        h.service.held(),
        Held {
            messages: 1,
            paid: 1
        }
    );
    let mut h = reopen(h);
    assert_eq!(h.service.tickets().unwrap(), tickets);
    assert_eq!(
        h.service.held(),
        Held {
            messages: 1,
            paid: 1
        }
    );
    // A copy pulled from another holder names this unit just the same.
    let copy = named_stamp(BOOK, &book_key(), &MAILBOX, P0, b"copy", 4, &holders);
    h.service
        .store_replica(MAILBOX, P0, b"copy", &copy, 1_002)
        .unwrap();
    assert_eq!(h.service.tickets().unwrap().len(), 2);
}

/// Stored, but no ticket: a stamp that names nobody, a list without this
/// unit or naming it twice (the contract pays neither), and a stamp of a
/// granted book, which pays holders nothing.
#[test]
fn only_paid_stamps_naming_this_unit_once_are_tickets() {
    let mut h = ready_until(u64::MAX);
    let key = book_key();
    let now = 11 * DAY + 3_600;
    let first = Stamp::sign(&DOMAIN, BOOK, 0, operation(&MAILBOX, 10, b"v1"), &key);
    let mut twice = listed(false);
    twice[7] = UNIT;
    for (label, envelope, stamp) in [
        ("names nobody", b"v1".as_slice(), first),
        (
            "without this unit",
            b"others",
            named_stamp(BOOK, &key, &MAILBOX, 10, b"others", 1, &listed(true)),
        ),
        (
            "this unit twice",
            b"twice",
            named_stamp(BOOK, &key, &MAILBOX, 10, b"twice", 2, &twice),
        ),
    ] {
        h.service
            .store(MAILBOX, 10, envelope, &stamp, now)
            .unwrap_or_else(|refusal| panic!("{label}: {refusal:?}"));
    }
    read_rules(&mut h.service, &server(), (10, 12), 0..=13, 12);
    let granted_key = BookKey::from_bytes(&[0x42; 32]).unwrap();
    let grant = granted(
        &server(),
        DOMAIN,
        granted_key.account(),
        10,
        0,
        100,
        25 * DAY,
    );
    h.service.learn_grant(&grant, 10 * DAY + 5, now).unwrap();
    let free = named_stamp(
        grant.id(),
        &granted_key,
        &MAILBOX,
        10,
        b"free",
        0,
        &listed(false),
    );
    h.service.store(MAILBOX, 10, b"free", &free, now).unwrap();
    assert!(h.service.tickets().unwrap().is_empty());
    assert_eq!(
        h.service.held(),
        Held {
            messages: 4,
            paid: 3
        }
    );
}

/// Draw terms of these tests: books last `validity` from their purchase,
/// tickets 360 days.
fn draw_terms(validity: u64, threshold: [u8; 32]) -> DrawTerms {
    DrawTerms {
        validity,
        lifetime: 360 * DAY,
        threshold,
    }
}

/// A ticket is drawn by the seed of its book's purchase day: it waits
/// while that seed is unknown, stays to be claimed if it won and is dropped
/// if it lost.
#[test]
fn tickets_are_drawn_by_the_seed_of_their_books_purchase_day() {
    let draw = |threshold: [u8; 32], seeds: &[(u64, [u8; 32])]| {
        let mut h = ready();
        for index in 0..2 {
            let envelope = [b'e', u8::try_from(index).unwrap()];
            let stamp = named_stamp(
                BOOK,
                &book_key(),
                &MAILBOX,
                P0,
                &envelope,
                index,
                &listed(false),
            );
            h.service
                .store(MAILBOX, P0, &envelope, &stamp, 1_000)
                .unwrap();
        }
        // The book ends at 5000 and lasts 4000: bought on day 0.
        h.service
            .draw(
                &draw_terms(4_000, threshold),
                &seeds.iter().copied().collect(),
            )
            .unwrap();
        h.service
            .tickets()
            .unwrap()
            .into_iter()
            .map(|t| t.state)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        draw([0xff; 32], &[(1, [0x5e; 32])]),
        [TicketState::Drawing, TicketState::Drawing]
    );
    assert_eq!(
        draw([0xff; 32], &[(0, [0x5e; 32])]),
        [TicketState::Won, TicketState::Won]
    );
    assert_eq!(draw([0; 32], &[(0, [0x5e; 32])]), Vec::<TicketState>::new());
}

/// A won ticket outlives its mailbox and its book: it lasts 360 days from
/// the purchase. The operator sees what ends within 30 days and in how many
/// days, rounded up; an ended ticket is dropped.
#[test]
fn a_won_ticket_lasts_360_days_from_its_purchase_and_shows_when_it_ends() {
    // Bought on day 10: the book ends on day 40, its tickets on day 370.
    let mut h = ready_until(40 * DAY);
    let now = 10 * DAY + 3_600;
    let stamp = named_stamp(BOOK, &book_key(), &MAILBOX, 10, b"won", 0, &listed(false));
    h.service.store(MAILBOX, 10, b"won", &stamp, now).unwrap();
    let terms = draw_terms(30 * DAY, [0xff; 32]);
    h.service
        .draw(&terms, &[(10, [0x5e; 32])].into_iter().collect())
        .unwrap();
    // The mailbox and the book are long gone.
    h.service.collect(100 * DAY).unwrap();
    assert_eq!(held_count(&h, &MAILBOX), 0);
    assert_eq!(h.service.tickets().unwrap().len(), 1);
    assert_eq!(h.service.ending(300 * DAY, &terms), None);
    assert_eq!(
        h.service.ending(345 * DAY + 3_600, &terms),
        Some(Ending {
            tickets: 1,
            in_days: 25
        })
    );
    h.service.forget_ended(370 * DAY - 1, &terms).unwrap();
    assert_eq!(h.service.tickets().unwrap().len(), 1);
    h.service.forget_ended(370 * DAY, &terms).unwrap();
    assert!(h.service.tickets().unwrap().is_empty());
}

fn held_count(h: &Holder, mailbox: &[u8; 32]) -> usize {
    h.service.read(mailbox, 0, MAX_PAGE).unwrap().entries.len()
}

/// Entries kept, and stamps sent, before stamps named holders still read:
/// their stamp names nobody. A named stamp keeps its list over the wire.
#[test]
fn stamps_in_the_form_before_named_holders_still_read() {
    let stamp = stamped(&MAILBOX, b"old", 1);
    let old = json!({
        "book": stamp.book.to_vec(),
        "index": 1,
        "operation": stamp.operation.to_vec(),
        "signature": stamp.signature.to_vec(),
    });
    let wire: StampWire = serde_json::from_value(old).unwrap();
    assert_eq!(Stamp::try_from(&wire).unwrap(), stamp);
    let named = named_stamp(BOOK, &book_key(), &MAILBOX, P0, b"new", 2, &listed(false));
    let wire = serde_json::to_value(StampWire::from(&named)).unwrap();
    let back: StampWire = serde_json::from_value(wire).unwrap();
    assert_eq!(Stamp::try_from(&back).unwrap(), named);
}

// --- revoked grants (Docs/V1_IDENTITY_PENALTIES_2026_09_30.md) -----------------

use agentic_grant_book::GrantRevocation;

/// A holder that read the test issuer's rules for days 0 to 11, and the
/// book key of its grants.
fn granting() -> (Holder, BookKey) {
    let mut h = ready_until(u64::MAX);
    read_rules(&mut h.service, &server(), (0, u64::MAX), 0..=11, 11);
    (h, BookKey::from_bytes(&[0x41; 32]).unwrap())
}

/// A stamp of `grant`'s book for `envelope` in `mailbox` and `period`.
fn spent(
    grant: &GrantBook,
    key: &BookKey,
    mailbox: &[u8; 32],
    period: u64,
    envelope: &[u8],
    index: u32,
) -> Stamp {
    Stamp::sign(
        &DOMAIN,
        grant.id(),
        index,
        operation(mailbox, period, envelope),
        key,
    )
}

#[test]
fn a_revoked_grant_pays_for_nothing_new_while_its_copies_are_repaired() {
    let (mut h, key) = granting();
    let grant = granted(&server(), DOMAIN, key.account(), 10, 0, 100, 25 * DAY);
    let neighbour = granted(&server(), DOMAIN, key.account(), 10, 1, 100, 25 * DAY);
    let now = 10 * DAY + 60;
    h.service.learn_grant(&grant, now, now).unwrap();
    h.service.learn_grant(&neighbour, now, now).unwrap();
    let before = spent(&grant, &key, &MAILBOX, 10, b"before", 0);
    h.service
        .store(MAILBOX, 10, b"before", &before, now)
        .unwrap();
    // Only the issuer revokes its grant.
    let stranger = SecpKey::from_secret(&[0x32; 32]).unwrap();
    let forged = GrantRevocation::issue(&grant, now, &stranger);
    assert_eq!(h.service.learn_revocation(&forged, now), Ok(false));
    let still = spent(&grant, &key, &MAILBOX, 10, b"still", 1);
    h.service.store(MAILBOX, 10, b"still", &still, now).unwrap();
    // The issuer's revocation: the grant pays for nothing new from then on,
    let at = now + 60;
    let revocation = GrantRevocation::issue(&grant, at, &server());
    assert_eq!(h.service.learn_revocation(&revocation, at), Ok(true));
    assert_eq!(h.service.learn_revocation(&revocation, at), Ok(false));
    let after = spent(&grant, &key, &MAILBOX, 10, b"after", 2);
    assert_eq!(
        h.service.store(MAILBOX, 10, b"after", &after, at),
        Err(Refusal::Blocked)
    );
    assert_eq!(
        h.service.blocked_books().unwrap(),
        BTreeSet::from([grant.id()])
    );
    // but what it paid for stays, and copies of that are still repaired.
    assert_eq!(
        h.service.read(&MAILBOX, 0, MAX_PAGE).unwrap().entries.len(),
        2
    );
    let copy = [0xc7; 32];
    let paid = spent(&grant, &key, &copy, 10, b"copy", 3);
    h.service
        .store_replica(copy, 10, b"copy", &paid, at)
        .unwrap();
    // A replica two periods later is new spending, not a copy.
    let late = [0xc8; 32];
    let spending = spent(&grant, &key, &late, 12, b"late", 5);
    assert_eq!(
        h.service
            .store_replica(late, 12, b"late", &spending, 12 * DAY + 5),
        Err(Refusal::Blocked)
    );
    // Another grant of the issuer's day still pays.
    let fine = spent(&neighbour, &key, &OTHER_MAILBOX, 10, b"fine", 0);
    h.service
        .store(OTHER_MAILBOX, 10, b"fine", &fine, at)
        .unwrap();
    // The revocation outlives a restart.
    let mut h = reopen(h);
    let later = spent(&grant, &key, &MAILBOX, 10, b"later", 4);
    assert_eq!(
        h.service.store(MAILBOX, 10, b"later", &later, at),
        Err(Refusal::Blocked)
    );
}

#[test]
fn a_revocation_read_before_its_grant_blocks_the_grant_once_learned() {
    let (mut h, key) = granting();
    let grant = granted(&server(), DOMAIN, key.account(), 10, 0, 100, 25 * DAY);
    let never = granted(&server(), DOMAIN, key.account(), 10, 1, 100, 20 * DAY);
    let now = 10 * DAY + 60;
    for revoked in [&grant, &never] {
        let revocation = GrantRevocation::issue(revoked, now, &server());
        assert_eq!(h.service.learn_revocation(&revocation, now), Ok(true));
    }
    assert!(h.service.blocked_books().unwrap().is_empty());
    // Kept across a restart, it blocks the grant as soon as it is learned:
    // replicas of what it paid for before are taken, nothing fresh.
    let mut h = reopen(h);
    read_rules(&mut h.service, &server(), (0, u64::MAX), 0..=11, 11);
    let later = now + 3_600;
    h.service.learn_grant(&grant, now, later).unwrap();
    assert_eq!(
        h.service.blocked_books().unwrap(),
        BTreeSet::from([grant.id()])
    );
    let fresh = spent(&grant, &key, &MAILBOX, 10, b"fresh", 0);
    assert_eq!(
        h.service.store(MAILBOX, 10, b"fresh", &fresh, later),
        Err(Refusal::Blocked)
    );
    let copy = spent(&grant, &key, &OTHER_MAILBOX, 10, b"copy", 1);
    h.service
        .store_replica(OTHER_MAILBOX, 10, b"copy", &copy, later)
        .unwrap();
    // A revocation of a grant never shown here goes when the grant ends.
    assert_eq!(h.service.info()["pendingRevocations"], 1);
    h.service.collect(20 * DAY).unwrap();
    assert_eq!(h.service.info()["pendingRevocations"], 0);
}

#[test]
fn a_double_spent_grant_is_queued_once_for_its_identity_server() {
    let (mut h, key) = granting();
    let grant = granted(&server(), DOMAIN, key.account(), 10, 0, 100, 25 * DAY);
    let other = granted(&server(), DOMAIN, key.account(), 10, 1, 100, 20 * DAY);
    let now = 10 * DAY + 60;
    h.service.learn_grant(&grant, now, now).unwrap();
    h.service.learn_grant(&other, now, now).unwrap();
    let first = spent(&grant, &key, &MAILBOX, 10, b"one", 0);
    let second = spent(&grant, &key, &OTHER_MAILBOX, 10, b"two", 0);
    h.service.store(MAILBOX, 10, b"one", &first, now).unwrap();
    assert_eq!(
        h.service.store(OTHER_MAILBOX, 10, b"two", &second, now),
        Err(Refusal::Conflict)
    );
    // A bought book spent twice is nobody's identity.
    let (a, b) = ([0xc1; 32], [0xc2; 32]);
    h.service
        .store(a, 10, b"bought", &stamped_in(&a, 10, b"bought", 0), now)
        .unwrap();
    assert_eq!(
        h.service
            .store(b, 10, b"twice", &stamped_in(&b, 10, b"twice", 0), now),
        Err(Refusal::Conflict)
    );
    // Another slot of the grant proven spent twice adds no second report.
    let (e, f) = ([0xc5; 32], [0xc6; 32]);
    h.service
        .accept_proofs(&Proofs {
            senders: vec![SenderEquivocation {
                first: spent(&grant, &key, &e, 10, b"e", 3),
                second: spent(&grant, &key, &f, 10, b"f", 3),
            }],
            ..Proofs::default()
        })
        .unwrap();
    assert_eq!(
        h.service.reports().unwrap(),
        vec![Report {
            grant: grant.clone(),
            first,
            second,
        }]
    );
    // A proof learned from another node is queued the same way.
    let (c, d) = ([0xc3; 32], [0xc4; 32]);
    let gossiped = SenderEquivocation {
        first: spent(&other, &key, &c, 10, b"x", 7),
        second: spent(&other, &key, &d, 10, b"y", 7),
    };
    h.service
        .accept_proofs(&Proofs {
            senders: vec![gossiped.clone()],
            ..Proofs::default()
        })
        .unwrap();
    assert_eq!(h.service.reports().unwrap().len(), 2);
    // The queue outlives a restart until the server answered.
    let mut h = reopen(h);
    h.service.reported(&grant.id()).unwrap();
    assert_eq!(
        h.service.reports().unwrap(),
        vec![Report {
            grant: other,
            first: gossiped.first,
            second: gossiped.second,
        }]
    );
    // Unreported, it goes when its grant ends.
    h.service.collect(20 * DAY).unwrap();
    assert!(h.service.reports().unwrap().is_empty());
}
