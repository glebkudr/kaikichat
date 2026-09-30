//! Core side of the mailbox swarm: both ends of a conversation derive the
//! same rotating mailbox per direction, each conversation has its own stamp
//! book, and stamps spend its slots durably, once per operation.
use super::*;
use agentic_core::{CoreError, DirectPayment, MailboxBook, SwarmDelivery, SwarmPending};
use agentic_crypto::CryptoError;
use agentic_crypto::mailbox::MailboxError;
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, period};
use agentic_mailbox_swarm::receipt::{HolderKey, Receipt};
use agentic_mailbox_swarm::select::{Member, QUORUM, SWARM_SIZE, rendezvous};
use agentic_mailbox_swarm::stamp::{BookTerms, book_id, named_operation, operation, swarm_digest};

struct Pair {
    alice_root: TempDir,
    alice: AppCore,
    bob_root: TempDir,
    bob: AppCore,
    conversation: String,
}

fn pair() -> Pair {
    let (alice_root, bob_root) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut alice = profile(&alice_root, "Alice");
    let mut bob = profile(&bob_root, "Bob");
    let conversation = connect(&mut alice, &mut bob);
    Pair {
        alice_root,
        alice,
        bob_root,
        bob,
        conversation,
    }
}

#[test]
fn both_ends_derive_the_same_mailbox_per_direction_and_period() {
    let p = pair();
    let c = &p.conversation;
    let to_bob = p.alice.swarm_mailbox(c, false, NOW).unwrap();
    assert_eq!(p.bob.swarm_mailbox(c, true, NOW).unwrap(), to_bob);
    let to_alice = p.bob.swarm_mailbox(c, false, NOW).unwrap();
    assert_eq!(p.alice.swarm_mailbox(c, true, NOW).unwrap(), to_alice);
    assert_ne!(to_bob, to_alice);
    let start = NOW / PERIOD_SECONDS * PERIOD_SECONDS;
    assert_eq!(p.alice.swarm_mailbox(c, false, start).unwrap(), to_bob);
    assert_eq!(
        p.alice
            .swarm_mailbox(c, false, start + PERIOD_SECONDS - 1)
            .unwrap(),
        to_bob
    );
    let next = p
        .alice
        .swarm_mailbox(c, false, start + PERIOD_SECONDS)
        .unwrap();
    assert_ne!(next, to_bob);
    assert_eq!(
        p.bob
            .swarm_mailbox(c, true, start + PERIOD_SECONDS)
            .unwrap(),
        next
    );
    assert!(matches!(
        p.alice.swarm_mailbox(&"ab".repeat(32), false, NOW),
        Err(CoreError::UnknownConversation)
    ));
}

#[test]
fn a_profile_has_one_book_key_for_every_conversation() {
    let mut p = pair();
    assert!(p.alice.mailbox_books().unwrap().is_empty());
    let account = p.alice.mailbox_book_account().unwrap();
    assert_eq!(p.alice.mailbox_book_account().unwrap(), account);
    assert_ne!(p.bob.mailbox_book_account().unwrap(), account);
    drop(p.alice);
    let mut alice = core(&p.alice_root, DOMAIN);
    assert_eq!(alice.mailbox_book_account().unwrap(), account);
    assert!(alice.mailbox_books().unwrap().is_empty());
}

#[test]
fn stamps_spend_one_slot_per_operation_durably_until_the_books_end() {
    let mut p = pair();
    let c = p.conversation.clone();
    let mailbox = p.alice.swarm_mailbox(&c, false, NOW).unwrap();
    assert!(matches!(
        p.alice.stamp_mailbox(&mailbox, period(NOW), b"a", NOW),
        Err(CoreError::MailboxBookMissing)
    ));
    let account = p.alice.mailbox_book_account().unwrap();
    let added = p.alice.add_mailbox_book(BOOK, 3, NOW + 100).unwrap();
    assert_eq!(
        added,
        MailboxBook {
            book: BOOK,
            count: 3,
            valid_until: NOW + 100,
            used: 0,
        }
    );
    let terms = BookTerms {
        key: account,
        count: 3,
        valid_until: NOW + 100,
    };
    let a = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    assert_eq!(
        (a.book, a.index, a.operation),
        (BOOK, 0, operation(&mailbox, period(NOW), b"a"))
    );
    assert_eq!(a.verify(&DOMAIN, &terms, NOW), Ok(()));
    let b = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"b", NOW)
        .unwrap();
    assert_eq!(b.index, 1);
    assert_eq!(b.verify(&DOMAIN, &terms, NOW), Ok(()));
    assert_eq!(
        p.alice
            .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
            .unwrap(),
        a
    );
    assert_eq!(p.alice.mailbox_books().unwrap()[0].used, 2);
    // The same bytes for another mailbox are another operation.
    let next = p
        .alice
        .swarm_mailbox(&c, false, NOW + PERIOD_SECONDS)
        .unwrap();
    // Durable: a restart neither forgets nor reuses a slot.
    drop(p.alice);
    let mut alice = core(&p.alice_root, DOMAIN);
    assert_eq!(
        alice
            .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
            .unwrap(),
        a
    );
    let other = alice
        .stamp_mailbox(&next, period(NOW) + 1, b"a", NOW)
        .unwrap();
    assert_eq!(other.index, 2);
    assert_eq!(other.operation, operation(&next, period(NOW) + 1, b"a"));
    assert!(matches!(
        alice.stamp_mailbox(&mailbox, period(NOW), b"d", NOW),
        Err(CoreError::MailboxBookExhausted)
    ));
    // A spent operation is still stamped after exhaustion, so its message
    // can reach the holders that missed it, up to the last valid second.
    assert_eq!(
        alice
            .stamp_mailbox(&next, period(NOW) + 1, b"a", NOW + 99)
            .unwrap(),
        other
    );
    assert!(matches!(
        alice.stamp_mailbox(&mailbox, period(NOW), b"a", NOW + 100),
        Err(CoreError::MailboxBookExpired)
    ));
    assert!(matches!(
        alice.stamp_mailbox(&mailbox, period(NOW), b"e", NOW + 100),
        Err(CoreError::MailboxBookExpired)
    ));
    assert_eq!(alice.mailbox_books().unwrap()[0].used, 3);
}

#[test]
fn every_conversation_spends_the_profile_books_soonest_ending_first() {
    let mut p = pair();
    let carol_root = TempDir::new().unwrap();
    let mut carol = profile(&carol_root, "Carol");
    let to_bob = p.alice.swarm_mailbox(&p.conversation, false, NOW).unwrap();
    let carol_chat = connect(&mut p.alice, &mut carol);
    let to_carol = p.alice.swarm_mailbox(&carol_chat, false, NOW).unwrap();
    // Ids, insertion order and end dates all disagree: `soon` ends first
    // but within a day, `mid` is the soonest to last beyond a day.
    let (late, soon, mid) = ([0xb3; 32], [0xb2; 32], [0xb1; 32]);
    p.alice.mailbox_book_account().unwrap();
    p.alice.add_mailbox_book(late, 2, NOW + MONTH).unwrap();
    p.alice.add_mailbox_book(soon, 2, NOW + 100).unwrap();
    p.alice
        .add_mailbox_book(mid, 2, NOW + 2 * PERIOD_SECONDS)
        .unwrap();
    let ops: [(&[u8; 32], &[u8]); 6] = [
        (&to_bob, b"1"),
        (&to_carol, b"2"),
        (&to_carol, b"3"),
        (&to_bob, b"4"),
        (&to_bob, b"5"),
        (&to_carol, b"6"),
    ];
    let stamp = |core: &mut AppCore, (mailbox, bytes): (&[u8; 32], &[u8]), at: u64| {
        core.stamp_mailbox(mailbox, period(NOW), bytes, at)
    };
    let spent: Vec<_> = ops
        .iter()
        .map(|op| {
            let s = stamp(&mut p.alice, *op, NOW).unwrap();
            (s.book, s.index)
        })
        .collect();
    // Both conversations draw from the same books: the soonest ending of
    // those lasting another day first, a book ending within a day last.
    assert_eq!(
        spent,
        [
            (mid, 0),
            (mid, 1),
            (late, 0),
            (late, 1),
            (soon, 0),
            (soon, 1)
        ]
    );
    assert!(matches!(
        p.alice.stamp_mailbox(&to_bob, period(NOW), b"7", NOW),
        Err(CoreError::MailboxBookExhausted)
    ));
    let used = |core: &AppCore| {
        core.mailbox_books()
            .unwrap()
            .iter()
            .map(|b| (b.book, b.used))
            .collect::<Vec<_>>()
    };
    assert_eq!(used(&p.alice), [(soon, 2), (mid, 2), (late, 2)]);
    // A spent operation keeps its book and slot even when a better book
    // appears.
    let fresh = [0xb4; 32];
    p.alice
        .add_mailbox_book(fresh, 5, NOW + 3 * PERIOD_SECONDS)
        .unwrap();
    for (n, op) in ops.iter().enumerate() {
        let again = stamp(&mut p.alice, *op, NOW).unwrap();
        assert_eq!((again.book, again.index), spent[n], "op {n}");
    }
    // Once its book ended, a spent operation is not moved to another book.
    let ended = [0xb5; 32];
    p.alice.add_mailbox_book(ended, 5, NOW + 150).unwrap();
    assert!(matches!(
        stamp(&mut p.alice, ops[4], NOW + 200),
        Err(CoreError::MailboxBookExpired)
    ));
    // A new operation goes to the book lasting another day.
    let next = p
        .alice
        .stamp_mailbox(&to_bob, period(NOW + 200), b"8", NOW + 200)
        .unwrap();
    assert_eq!((next.book, next.index), (fresh, 0));
    assert_eq!(
        used(&p.alice),
        [(soon, 2), (ended, 0), (mid, 2), (fresh, 1), (late, 2)]
    );
    // All of it survives a restart.
    drop(p.alice);
    let mut alice = core(&p.alice_root, DOMAIN);
    assert_eq!(
        used(&alice),
        [(soon, 2), (ended, 0), (mid, 2), (fresh, 1), (late, 2)]
    );
    assert!(matches!(
        alice.add_mailbox_book(fresh, 6, NOW + 3 * PERIOD_SECONDS),
        Err(CoreError::InvalidInput)
    ));
    assert_eq!(
        stamp(&mut alice, ops[2], NOW + 200)
            .map(|s| (s.book, s.index))
            .unwrap(),
        (late, 0)
    );
}

#[test]
fn equal_books_are_spent_in_id_order_and_ended_ones_never() {
    let mut p = pair();
    let mailbox = p.alice.swarm_mailbox(&p.conversation, false, NOW).unwrap();
    // A book may be recorded before the key is first asked for.
    p.alice
        .add_mailbox_book([0xc2; 32], 1, NOW + MONTH)
        .unwrap();
    p.alice
        .add_mailbox_book([0xc1; 32], 1, NOW + MONTH)
        .unwrap();
    let key = p.alice.mailbox_book_account().unwrap();
    let first = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    assert_eq!((first.book, first.index), ([0xc1; 32], 0));
    let terms = BookTerms {
        key,
        count: 1,
        valid_until: NOW + MONTH,
    };
    assert_eq!(first.verify(&DOMAIN, &terms, NOW), Ok(()));
    let second = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"c", NOW)
        .unwrap();
    assert_eq!((second.book, second.index), ([0xc2; 32], 0));
    // With only short books left, an ended one is skipped even with a free
    // slot, for the one still valid.
    p.alice.add_mailbox_book([0xc3; 32], 1, NOW + 150).unwrap();
    p.alice.add_mailbox_book([0xc4; 32], 1, NOW + 500).unwrap();
    let short = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW + 200), b"d", NOW + 200)
        .unwrap();
    assert_eq!((short.book, short.index), ([0xc4; 32], 0));
    // Every book ended, one with free slots: ended, not exhausted.
    let later = NOW + MONTH;
    assert!(matches!(
        p.alice.stamp_mailbox(&mailbox, period(later), b"b", later),
        Err(CoreError::MailboxBookExpired)
    ));
}

#[test]
fn a_book_lasting_exactly_another_day_counts_as_lasting() {
    let mut p = pair();
    let mailbox = p.alice.swarm_mailbox(&p.conversation, false, NOW).unwrap();
    p.alice.mailbox_book_account().unwrap();
    // Ends a second too early; has the smaller id and ends first.
    p.alice
        .add_mailbox_book([0xd1; 32], 1, NOW + PERIOD_SECONDS - 1)
        .unwrap();
    p.alice
        .add_mailbox_book([0xd2; 32], 1, NOW + PERIOD_SECONDS)
        .unwrap();
    let stamp = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    assert_eq!((stamp.book, stamp.index), ([0xd2; 32], 0));
    let then = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"b", NOW)
        .unwrap();
    assert_eq!((then.book, then.index), ([0xd1; 32], 0));
}

#[test]
fn a_grant_funds_the_profile_book_it_names() {
    use agentic_grant_book::{GrantBook, GrantTerms, SecpKey};
    let mut p = pair();
    let server = SecpKey::from_secret(&[0x31; 32]).unwrap();
    let account = p.alice.mailbox_book_account().unwrap();
    let grant = |book: [u8; 20], serial: u32| {
        GrantBook::issue(
            GrantTerms {
                domain: DOMAIN,
                book,
                day: period(NOW),
                serial,
                count: 100,
                expiry: NOW + MONTH,
            },
            &server,
        )
    };
    let mine = grant(account, 1);
    let added = p.alice.add_mailbox_grant(&mine).unwrap();
    assert_eq!(
        added,
        MailboxBook {
            book: mine.id(),
            count: 100,
            valid_until: NOW + MONTH,
            used: 0,
        }
    );
    assert_eq!(p.alice.mailbox_books().unwrap(), vec![added.clone()]);
    assert_eq!(
        p.alice.mailbox_book_grant(&mine.id()).unwrap(),
        Some(mine.clone())
    );
    // It pays like any book: the stamp is the profile key's, on the grant's id.
    let mailbox = p.alice.swarm_mailbox(&p.conversation, false, NOW).unwrap();
    let stamp = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    assert_eq!((stamp.book, stamp.index), (mine.id(), 0));
    let terms = BookTerms {
        key: account,
        count: 100,
        valid_until: NOW + MONTH,
    };
    assert_eq!(stamp.verify(&DOMAIN, &terms, NOW), Ok(()));
    // Only a genuine grant of this network naming this profile's key;
    // adding it again is a no-op keeping its spent slots.
    let before = persisted_state(&p.alice_root);
    let mut forged = grant(account, 2);
    forged.count = 1_000;
    let elsewhere = GrantBook::issue(
        GrantTerms {
            domain: [0x12; 32],
            book: account,
            day: period(NOW),
            serial: 4,
            count: 100,
            expiry: NOW + MONTH,
        },
        &server,
    );
    // The same serial again with other terms: a book's terms never change.
    let reissued = GrantBook::issue(
        GrantTerms {
            domain: DOMAIN,
            book: account,
            day: period(NOW),
            serial: 1,
            count: 100,
            expiry: NOW + MONTH - 1,
        },
        &server,
    );
    assert_eq!(reissued.id(), mine.id());
    for bad in [grant([9; 20], 3), forged, elsewhere, reissued.clone()] {
        assert!(matches!(
            p.alice.add_mailbox_grant(&bad),
            Err(CoreError::InvalidInput)
        ));
    }
    assert_eq!(persisted_state(&p.alice_root), before);
    assert_eq!(p.alice.add_mailbox_grant(&mine).unwrap().used, 1);
    // A bought book has no grant to show.
    p.alice.add_mailbox_book(BOOK, 10, NOW + MONTH).unwrap();
    assert_eq!(p.alice.mailbox_book_grant(&BOOK).unwrap(), None);
    drop(p.alice);
    let alice = core(&p.alice_root, DOMAIN);
    assert_eq!(alice.mailbox_book_grant(&mine.id()).unwrap(), Some(mine));
}

/// Access by book (Docs/V1_DISCOVERY_2026_09_27.md): the profile shows
/// holders a pass signed by its book key for the book that lasts longest,
/// with the grant when that book is one. No active book, no pass.
#[test]
fn a_pass_names_the_longest_lasting_active_book_and_carries_its_grant() {
    use agentic_grant_book::{GrantBook, GrantTerms, SecpKey};
    use agentic_mailbox_swarm::access::AccessPass;
    const PEER: [u8; 32] = [0x9e; 32];
    let mut p = pair();
    let day = period(NOW);
    let account = p.alice.mailbox_book_account().unwrap();
    assert!(p.alice.mailbox_access(PEER, day, NOW).unwrap().is_none());
    // A spent-out book still lets its owner in, and outlasts the other.
    p.alice
        .add_mailbox_book([0xc1; 32], 0, NOW + MONTH)
        .unwrap();
    p.alice.add_mailbox_book([0xc2; 32], 5, NOW + 1).unwrap();
    let access = p.alice.mailbox_access(PEER, day, NOW).unwrap().unwrap();
    assert_eq!(
        access.pass,
        AccessPass {
            book: [0xc1; 32],
            peer: PEER,
            day,
            ..access.pass.clone()
        }
    );
    assert_eq!(access.pass.signer(&DOMAIN).unwrap(), account);
    assert_eq!(access.grant, None);
    let server = SecpKey::from_secret(&[0x31; 32]).unwrap();
    let grant = GrantBook::issue(
        GrantTerms {
            domain: DOMAIN,
            book: account,
            day,
            serial: 1,
            count: 100,
            expiry: NOW + 2 * MONTH,
        },
        &server,
    );
    p.alice.add_mailbox_grant(&grant).unwrap();
    let access = p.alice.mailbox_access(PEER, day, NOW).unwrap().unwrap();
    assert_eq!(access.pass.book, grant.id());
    assert_eq!(access.grant, Some(grant));
    // Past every book's end there is nothing to show.
    let later = NOW + 2 * MONTH;
    assert!(
        p.alice
            .mailbox_access(PEER, period(later), later)
            .unwrap()
            .is_none()
    );
}

/// Discovery (spec/discovery-v1.md) is paid with stamps for operations of
/// its own: one slot per operation, the same slot for the same operation
/// again, never a slot of an ended book; what the profile sends the service
/// is signed by its root key.
#[test]
fn discovery_stamps_spend_a_slot_per_operation_and_documents_carry_the_root_key() {
    use agentic_mailbox_swarm::discover::{card_operation, lookup_operation};
    use agentic_protocol::directory::Request as Signed;
    let mut p = pair();
    let account = p.alice.mailbox_book_account().unwrap();
    assert!(matches!(
        p.alice.stamp_operation([1; 32], NOW),
        Err(CoreError::MailboxBookMissing)
    ));
    p.alice
        .add_mailbox_book([0xc1; 32], 3, NOW + MONTH)
        .unwrap();
    let lookup = lookup_operation(&DOMAIN, "google", &[7; 32], 1);
    let first = p.alice.stamp_operation(lookup, NOW).unwrap();
    assert_eq!((first.book, first.operation), ([0xc1; 32], lookup));
    assert_eq!(first.signer(&DOMAIN).unwrap(), account);
    // A retry spends nothing more; another operation takes the next slot.
    assert_eq!(p.alice.stamp_operation(lookup, NOW).unwrap(), first);
    let card = card_operation(&DOMAIN, &[9; 32], 0);
    assert_eq!(
        p.alice.stamp_operation(card, NOW).unwrap().index,
        first.index + 1
    );
    assert_eq!(p.alice.mailbox_books().unwrap()[0].used, 2);
    // The last slot, then nothing: the book is spent out.
    p.alice
        .stamp_operation(card_operation(&DOMAIN, &[9; 32], 1), NOW)
        .unwrap();
    assert!(matches!(
        p.alice
            .stamp_operation(card_operation(&DOMAIN, &[9; 32], 2), NOW),
        Err(CoreError::MailboxBookExhausted)
    ));
    // A book that has ended pays for nothing, even with slots left.
    p.alice.add_mailbox_book([0xc2; 32], 10, NOW + 10).unwrap();
    let later = NOW + MONTH;
    assert!(matches!(
        p.alice
            .stamp_operation(card_operation(&DOMAIN, &[9; 32], 3), later),
        Err(CoreError::MailboxBookExpired)
    ));
    // A consent to bind, signed by the profile's root key.
    let body = Signed::Link {
        kind: "github".into(),
        service: "https://directory.example".into(),
    }
    .encode();
    let wire = p.alice.directory_document(body.clone(), NOW).unwrap();
    let document = VerifiedDocument::decode(&wire, DOMAIN, NOW).unwrap();
    assert_eq!(document.kind(), DocumentKind::Directory);
    assert_eq!(document.body(), body.as_slice());
    assert_eq!(
        agentic_protocol::network_id(document.author()),
        p.alice.snapshot().unwrap().identity.unwrap().network_id
    );
}

/// `coins buy`: a purchase request is a book of the profile's key under a
/// fresh salt, one unpaid request at a time. It funds sending only once the
/// chain confirmed it for that key, however late it is paid.
#[test]
fn a_purchase_waits_for_its_confirmation_and_funds_the_profile_key() {
    let mut p = pair();
    let account = p.alice.mailbox_book_account().unwrap();
    let request = p.alice.start_mailbox_purchase(NOW).unwrap();
    assert_eq!(request.key, account);
    assert_eq!(request.book, book_id(&DOMAIN, &account, &request.salt));
    assert_eq!(request.created_at, NOW);
    // Asking again before paying gives the same request.
    assert_eq!(p.alice.start_mailbox_purchase(NOW + 1).unwrap(), request);
    // Pending across a restart, and not a book yet.
    drop(p.alice);
    let mut alice = core(&p.alice_root, DOMAIN);
    assert_eq!(alice.mailbox_purchases().unwrap(), vec![request.clone()]);
    assert!(alice.mailbox_books().unwrap().is_empty());
    // Only a pending purchase of the profile's key is confirmed.
    let before = persisted_state(&p.alice_root);
    for (book, key) in [(BOOK, account), (request.book, [9; 20])] {
        assert!(matches!(
            alice.confirm_mailbox_purchase(book, key, 100, NOW + MONTH),
            Err(CoreError::InvalidInput)
        ));
    }
    assert_eq!(persisted_state(&p.alice_root), before);
    // Paid late, weeks after the request, it still funds the profile.
    let late = NOW + 3 * 7 * PERIOD_SECONDS;
    assert_eq!(alice.mailbox_purchases().unwrap(), vec![request.clone()]);
    let bought = alice
        .confirm_mailbox_purchase(request.book, account, 100, late + MONTH)
        .unwrap();
    assert_eq!(
        bought,
        MailboxBook {
            book: request.book,
            count: 100,
            valid_until: late + MONTH,
            used: 0,
        }
    );
    assert_eq!(alice.mailbox_books().unwrap(), vec![bought.clone()]);
    assert!(alice.mailbox_purchases().unwrap().is_empty());
    // Seeing the confirmation again changes nothing.
    assert_eq!(
        alice
            .confirm_mailbox_purchase(request.book, account, 100, late + MONTH)
            .unwrap(),
        bought
    );
    // Once paid, the next request is another book.
    let next = alice.start_mailbox_purchase(late + 1).unwrap();
    assert_ne!(next.book, request.book);
    assert_eq!(next.book, book_id(&DOMAIN, &account, &next.salt));
    assert_eq!(alice.mailbox_purchases().unwrap(), vec![next]);
}

/// `coins claim`: the request asking the identity server for a grant is
/// signed by the profile's book key and kept, with the server's claim, until
/// the claim ends, so a restart never loses a grant already signed in for.
#[test]
fn a_claim_is_signed_by_the_book_key_and_kept_until_it_ends() {
    let mut p = pair();
    let account = p.alice.mailbox_book_account().unwrap();
    let claim = p.alice.start_mailbox_claim(NOW).unwrap();
    assert!(claim.request.verify().is_ok());
    assert_eq!(
        (
            claim.request.domain,
            claim.request.book,
            claim.request.created_at
        ),
        (DOMAIN, account, NOW)
    );
    assert_eq!(
        (&claim.claim_id, &claim.login_url, claim.expires_at),
        (&None, &None, None)
    );
    // Asked again while it lasts, the same request: the server answers a
    // repeated request with the same claim.
    assert_eq!(p.alice.start_mailbox_claim(NOW + 5).unwrap(), claim);
    let opened = p
        .alice
        .open_mailbox_claim("c1", "https://id.test/v1/claims/c1/login", NOW + 900)
        .unwrap();
    assert_eq!(opened.request, claim.request);
    assert_eq!(
        (
            opened.claim_id.as_deref(),
            opened.login_url.as_deref(),
            opened.expires_at
        ),
        (
            Some("c1"),
            Some("https://id.test/v1/claims/c1/login"),
            Some(NOW + 900)
        )
    );
    drop(p.alice);
    let mut alice = core(&p.alice_root, DOMAIN);
    assert_eq!(alice.mailbox_claim().unwrap(), Some(opened.clone()));
    assert_eq!(alice.start_mailbox_claim(NOW + 10).unwrap(), opened);
    alice.finish_mailbox_claim().unwrap();
    assert_eq!(alice.mailbox_claim().unwrap(), None);
    let next = alice.start_mailbox_claim(NOW + 20).unwrap();
    assert_ne!(next.request.nonce, claim.request.nonce);
    assert_eq!(next.claim_id, None);
}

/// Direct delivery pays with the stamped envelope the sender stores in the
/// swarm: the recipient opens it from its author's node and answers with a
/// receipt, once per message.
#[test]
fn a_stamped_envelope_is_received_from_its_author_with_a_receipt() {
    let mut p = pair();
    fund(&mut p.alice);
    let sent = p
        .alice
        .send_message(&p.conversation, "directly", "d1", NOW)
        .unwrap();
    let d = p.alice.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    let record = p.alice.create_node_record("peer-A", vec![], NOW).unwrap();
    // Only from its author's node, and only the envelope as sealed.
    let carol_root = TempDir::new().unwrap();
    let carol = profile(&carol_root, "Carol");
    let stranger = carol.create_node_record("peer-C", vec![], NOW).unwrap();
    let before = persisted_state(&p.bob_root);
    assert!(
        p.bob
            .receive_stamped_from(
                &d.conversation_id,
                d.period,
                &d.envelope,
                &stranger,
                "peer-C",
                DirectPayment::Checked,
                NOW
            )
            .is_err()
    );
    let mut altered = d.envelope.clone();
    altered[40] ^= 1;
    assert!(
        p.bob
            .receive_stamped_from(
                &d.conversation_id,
                d.period,
                &altered,
                &record,
                "peer-A",
                DirectPayment::Checked,
                NOW
            )
            .is_err()
    );
    assert_eq!(persisted_state(&p.bob_root), before);
    assert!(texts(&p.bob).is_empty());
    let outcome = p
        .bob
        .receive_stamped_from(
            &d.conversation_id,
            d.period,
            &d.envelope,
            &record,
            "peer-A",
            DirectPayment::Checked,
            NOW,
        )
        .unwrap();
    assert!(outcome.reply.is_some());
    assert_eq!(texts(&p.bob), ["directly"]);
    // The swarm copy or a retry is the same message; the receipt is sent again.
    let again = p
        .bob
        .receive_stamped_from(
            &d.conversation_id,
            d.period,
            &d.envelope,
            &record,
            "peer-A",
            DirectPayment::Checked,
            NOW,
        )
        .unwrap();
    assert!(again.reply.is_some());
    assert_eq!(texts(&p.bob), ["directly"]);
    // The receipt reaches Alice unpaid: it is a control message.
    let bob_record = p.bob.create_node_record("peer-B", vec![], NOW).unwrap();
    p.alice
        .receive_control_from(&outcome.reply.unwrap(), &bob_record, "peer-B", NOW)
        .unwrap();
}

/// Bob takes Alice's message directly, its payment checked or not.
fn take_directly(p: &mut Pair, text: &str, operation: &str, paid: DirectPayment) -> SwarmDelivery {
    let sent = p
        .alice
        .send_message(&p.conversation, text, operation, NOW)
        .unwrap();
    let d = p.alice.prepare_swarm_delivery(&sent.id, NOW).unwrap();
    let record = p.alice.create_node_record("peer-A", vec![], NOW).unwrap();
    p.bob
        .receive_stamped_from(
            &d.conversation_id,
            d.period,
            &d.envelope,
            &record,
            "peer-A",
            paid,
            NOW,
        )
        .unwrap();
    d
}

/// A message taken directly while its stamp could not be checked is shown
/// with low trust wherever it is read: the snapshot, the window's history,
/// the owner's inbox and an agent's, also after a restart. A checked
/// message is not, even if a copy of it arrives unchecked later.
#[test]
fn a_message_taken_without_checking_its_stamp_is_shown_with_low_trust() {
    let mut p = pair();
    fund(&mut p.alice);
    take_directly(&mut p, "offline", "o1", DirectPayment::Unchecked);
    let online = take_directly(&mut p, "online", "o2", DirectPayment::Checked);
    let record = p.alice.create_node_record("peer-A", vec![], NOW).unwrap();
    p.bob
        .receive_stamped_from(
            &online.conversation_id,
            online.period,
            &online.envelope,
            &record,
            "peer-A",
            DirectPayment::Unchecked,
            NOW,
        )
        .unwrap();
    let marks = |bob: &AppCore| -> Vec<(String, bool)> {
        bob.snapshot().unwrap().conversations[0]
            .messages
            .iter()
            .map(|m| (m.text.clone(), m.low_trust))
            .collect()
    };
    let expected = [("offline".to_owned(), true), ("online".to_owned(), false)];
    assert_eq!(marks(&p.bob), expected);
    let history = p.bob.conversation_history(&p.conversation, None).unwrap();
    assert_eq!(
        history
            .messages
            .iter()
            .map(|m| (m.text.clone(), m.low_trust))
            .collect::<Vec<_>>(),
        expected
    );
    let inbox = p
        .bob
        .owner_inbox_poll(&p.conversation, 10, 60, NOW)
        .unwrap();
    assert_eq!(
        inbox
            .items
            .iter()
            .map(|i| (i.text.clone(), i.low_trust))
            .collect::<Vec<_>>(),
        expected
    );
    // An agent reads the mark where it is set; a checked item has none.
    let key = runtime_key(72);
    let grant = grant_runtime(&mut p.bob, &key, &p.conversation, &[Action::ReadInbox]);
    let page = agent_inbox::call_at(
        &mut p.bob,
        &key,
        &grant,
        "inbox_poll",
        agent_inbox::poll(&p.conversation, "low-trust", 10, 4096, 30),
        1,
        NOW,
    )
    .unwrap();
    let items = page["items"].as_array().unwrap();
    assert_eq!(
        items
            .iter()
            .map(|i| (i["text"].as_str().unwrap(), i.get("lowTrust").cloned()))
            .collect::<Vec<_>>(),
        [("offline", Some(json!(true))), ("online", None)]
    );
    drop(p.bob);
    let bob = core(&p.bob_root, DOMAIN);
    assert_eq!(marks(&bob), expected);
}

/// A node that takes payment receives unpaid only control messages: a
/// Welcome of a new contact and receipts, never an application message.
#[test]
fn a_node_taking_payment_refuses_an_unpaid_application_message() {
    let mut p = pair();
    let sent = p
        .alice
        .send_message(&p.conversation, "without a stamp", "u1", NOW)
        .unwrap();
    let wire = p
        .alice
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|item| item.message_id == sent.id)
        .unwrap()
        .wire;
    let record = p.alice.create_node_record("peer-A", vec![], NOW).unwrap();
    let before = persisted_state(&p.bob_root);
    assert!(matches!(
        p.bob.receive_control_from(&wire, &record, "peer-A", NOW),
        Err(CoreError::PaymentRequired)
    ));
    assert_eq!(persisted_state(&p.bob_root), before);
    assert!(texts(&p.bob).is_empty());
    // A new contact's Welcome goes through unpaid.
    let carol_root = TempDir::new().unwrap();
    let mut carol = profile(&carol_root, "Carol");
    let invite = p.bob.create_invitation(NOW, vec![]).unwrap();
    let with_bob = carol.add_contact("Bob", &invite, NOW).unwrap().id;
    let welcome = carol.outbox(100).unwrap().remove(0).wire;
    let carol_record = carol.create_node_record("peer-C", vec![], NOW).unwrap();
    p.bob
        .receive_control_from(&welcome, &carol_record, "peer-C", NOW)
        .unwrap();
    assert!(
        p.bob
            .snapshot()
            .unwrap()
            .conversations
            .iter()
            .any(|c| c.id == with_bob)
    );
}

#[test]
fn a_book_is_added_once_with_terms_that_never_change() {
    let mut p = pair();
    let mailbox = p.alice.swarm_mailbox(&p.conversation, false, NOW).unwrap();
    p.alice.mailbox_book_account().unwrap();
    p.alice.add_mailbox_book(BOOK, 5, NOW + 100).unwrap();
    p.alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    // Adding it again keeps its spent slots.
    let again = p.alice.add_mailbox_book(BOOK, 5, NOW + 100).unwrap();
    assert_eq!(again.used, 1);
    for (count, valid_until) in [(6, NOW + 100), (5, NOW + 101)] {
        let before = persisted_state(&p.alice_root);
        assert!(matches!(
            p.alice.add_mailbox_book(BOOK, count, valid_until),
            Err(CoreError::InvalidInput)
        ));
        assert_eq!(persisted_state(&p.alice_root), before);
    }
    assert_eq!(
        p.alice
            .stamp_mailbox(&mailbox, period(NOW), b"b", NOW)
            .unwrap()
            .index,
        1
    );
}

#[test]
fn a_slot_is_spent_only_on_a_period_holders_still_take() {
    let mut p = pair();
    let c = p.conversation.clone();
    let mailbox = p.alice.swarm_mailbox(&c, false, NOW).unwrap();
    p.alice.mailbox_book_account().unwrap();
    p.alice.add_mailbox_book(BOOK, 10, NOW + MONTH).unwrap();
    let now = period(NOW);
    for stale in [now - 2, now + 2] {
        let before = persisted_state(&p.alice_root);
        assert!(matches!(
            p.alice.stamp_mailbox(&mailbox, stale, b"a", NOW),
            Err(CoreError::InvalidInput)
        ));
        assert_eq!(persisted_state(&p.alice_root), before);
    }
    for (n, writable) in [now - 1, now, now + 1].into_iter().enumerate() {
        let stamp = p
            .alice
            .stamp_mailbox(&mailbox, writable, b"a", NOW)
            .unwrap();
        assert_eq!(stamp.index, u32::try_from(n).unwrap());
        assert_eq!(stamp.operation, operation(&mailbox, writable, b"a"));
    }
}

#[test]
fn a_conversation_starts_when_its_welcome_was_issued() {
    let (ar, br) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    let c = alice.add_contact("Bob", &invite, NOW + 10).unwrap().id;
    assert_eq!(alice.conversation_started_at(&c).unwrap(), NOW + 10);
    let welcome = alice.outbox(1).unwrap().remove(0);
    // The joiner starts when the Welcome was issued, not when it arrived.
    let reply = bob
        .receive(&welcome.wire, NOW + 500)
        .unwrap()
        .reply
        .unwrap();
    assert_eq!(bob.conversation_started_at(&c).unwrap(), NOW + 10);
    // Neither the joiner's answer nor later messages move it, on either end.
    alice.receive(&reply, NOW + 2_000).unwrap();
    assert_eq!(alice.conversation_started_at(&c).unwrap(), NOW + 10);
    alice.send_message(&c, "later", "l", NOW + 2_100).unwrap();
    let later = alice.outbox(1).unwrap().remove(0).wire;
    bob.receive(&later, NOW + 2_500).unwrap();
    assert_eq!(alice.conversation_started_at(&c).unwrap(), NOW + 10);
    assert_eq!(bob.conversation_started_at(&c).unwrap(), NOW + 10);
    // Durable on both ends.
    drop((alice, bob));
    let (alice, bob) = (core(&ar, DOMAIN), core(&br, DOMAIN));
    assert_eq!(alice.conversation_started_at(&c).unwrap(), NOW + 10);
    assert_eq!(bob.conversation_started_at(&c).unwrap(), NOW + 10);
    assert!(matches!(
        alice.conversation_started_at(&"ab".repeat(32)),
        Err(CoreError::UnknownConversation)
    ));
}

#[test]
fn the_read_through_period_of_a_conversation_only_moves_forward() {
    let mut p = pair();
    let c = p.conversation.clone();
    let carol_root = TempDir::new().unwrap();
    let mut carol = profile(&carol_root, "Carol");
    let other = connect(&mut p.bob, &mut carol);
    assert_eq!(p.bob.swarm_read_through(&c).unwrap(), None);
    p.bob.set_swarm_read_through(&c, 100, NOW).unwrap();
    assert_eq!(p.bob.swarm_read_through(&c).unwrap(), Some(100));
    // Never back: a late or repeated completion changes nothing.
    for earlier in [99, 100] {
        p.bob.set_swarm_read_through(&c, earlier, NOW).unwrap();
        assert_eq!(p.bob.swarm_read_through(&c).unwrap(), Some(100));
    }
    p.bob.set_swarm_read_through(&c, 105, NOW).unwrap();
    assert_eq!(p.bob.swarm_read_through(&other).unwrap(), None);
    assert_eq!(p.alice.swarm_read_through(&c).unwrap(), None);
    for unknown in [
        p.bob.swarm_read_through(&"ab".repeat(32)).map(|_| ()),
        p.bob.set_swarm_read_through(&"ab".repeat(32), 1, NOW),
    ] {
        assert!(matches!(unknown, Err(CoreError::UnknownConversation)));
    }
    drop(p.bob);
    let bob = core(&p.bob_root, DOMAIN);
    assert_eq!(bob.swarm_read_through(&c).unwrap(), Some(105));
}

#[test]
fn a_failed_slot_commit_leaves_nothing_and_reuses_no_slot() {
    let mut p = pair();
    let c = p.conversation.clone();
    let mailbox = p.alice.swarm_mailbox(&c, false, NOW).unwrap();
    p.alice.mailbox_book_account().unwrap();
    p.alice.add_mailbox_book(BOOK, 10, NOW + 100).unwrap();
    let first = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    let db = profile_db::encrypted_db(&p.alice_root);
    for (trigger, event) in [
        ("fail_swarm_update", "UPDATE"),
        ("fail_swarm_insert", "INSERT"),
    ] {
        let before = persisted_state(&p.alice_root);
        db.execute_batch(&format!(
            "CREATE TRIGGER {trigger} BEFORE {event} ON states \
             WHEN NEW.namespace LIKE 'swarm/%' BEGIN SELECT RAISE(ABORT,'disk full'); END;"
        ))
        .unwrap();
        assert!(
            p.alice
                .stamp_mailbox(&mailbox, period(NOW), event.as_bytes(), NOW)
                .is_err()
        );
        assert_eq!(persisted_state(&p.alice_root), before, "{event}");
        db.execute_batch(&format!("DROP TRIGGER {trigger};"))
            .unwrap();
    }
    // Neither failed attempt consumed or skipped a slot.
    let update = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"UPDATE", NOW)
        .unwrap();
    let insert = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"INSERT", NOW)
        .unwrap();
    assert_eq!((first.index, update.index, insert.index), (0, 1, 2));
    assert_eq!(p.alice.mailbox_books().unwrap()[0].used, 3);
}

// --- sending and receiving through the swarm --------------------------------

const MONTH: u64 = 30 * PERIOD_SECONDS;

const BOOK: [u8; 32] = [0xb0; 32];

/// Give the profile a verified book of ten slots; the terms holders check.
fn fund(core: &mut AppCore) -> BookTerms {
    let key = core.mailbox_book_account().unwrap();
    core.add_mailbox_book(BOOK, 10, NOW + MONTH).unwrap();
    BookTerms {
        key,
        count: 10,
        valid_until: NOW + MONTH,
    }
}

fn holder(n: u8) -> HolderKey {
    HolderKey::from_bytes(&[n; 32]).unwrap()
}

/// The swarm a node selected for the delivery: holders 1..=10.
fn swarm() -> Vec<Account> {
    (1..=10).map(|n| holder(n).account()).collect()
}

fn receipt(d: &SwarmDelivery, n: u8) -> Receipt {
    Receipt::sign(
        &DOMAIN,
        d.mailbox,
        d.stamp.operation,
        d.stamp.ticket_id(&DOMAIN),
        [n; 32],
        NOW + 1,
        &holder(n),
    )
}

/// Receipts in holder order: the contract does not fix their order.
fn sorted(mut receipts: Vec<Receipt>) -> Vec<Receipt> {
    receipts.sort_by_key(|r| r.holder);
    receipts
}

fn texts(core: &AppCore) -> Vec<String> {
    core.snapshot().unwrap().conversations[0]
        .messages
        .iter()
        .map(|m| m.text.clone())
        .collect()
}

#[test]
fn the_swarm_outbox_holds_conversation_messages_but_never_a_welcome() {
    let (ar, br) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let mut alice = profile(&ar, "Alice");
    let mut bob = profile(&br, "Bob");
    let invite = bob.create_invitation(NOW, vec![]).unwrap();
    let c = alice.add_contact("Bob", &invite, NOW).unwrap().id;
    // The Welcome waits for direct delivery: its recipient cannot derive the
    // conversation's mailbox before joining.
    let welcome = alice.outbox(10).unwrap().remove(0);
    assert!(alice.swarm_outbox(10).unwrap().is_empty());
    fund(&mut alice);
    assert!(matches!(
        alice.prepare_swarm_delivery(&welcome.message_id, NOW),
        Err(CoreError::InvalidInput)
    ));
    // A message staged before the Welcome is acknowledged already goes to
    // the swarm; the recipient reads it once joined.
    let early = alice.send_message(&c, "early", "early", NOW).unwrap();
    assert_eq!(
        alice.swarm_outbox(10).unwrap(),
        vec![SwarmPending {
            message_id: early.id.clone(),
            conversation_id: c.clone(),
        }]
    );
    let reply = bob.receive(&welcome.wire, NOW).unwrap().reply.unwrap();
    alice.receive(&reply, NOW).unwrap();
    let second = alice.send_message(&c, "second", "second", NOW).unwrap();
    let ids = |pending: Vec<SwarmPending>| {
        pending
            .into_iter()
            .map(|p| p.message_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(alice.swarm_outbox(10).unwrap()),
        [early.id.clone(), second.id.clone()]
    );
    assert_eq!(
        ids(alice.swarm_outbox(1).unwrap()),
        std::slice::from_ref(&early.id)
    );
    for limit in [0, 65] {
        assert!(matches!(
            alice.swarm_outbox(limit),
            Err(CoreError::InvalidInput)
        ));
    }
    assert_eq!(alice.swarm_outbox(64).unwrap().len(), 2);
    assert!(bob.swarm_outbox(10).unwrap().is_empty());
}

#[test]
fn a_prepared_delivery_is_sealed_stamped_and_keeps_its_period_while_readable() {
    let mut p = pair();
    let c = p.conversation.clone();
    let m = p.alice.send_message(&c, "Secret", "m", NOW).unwrap();
    let wire = p.alice.outbox(10).unwrap().remove(0).wire;
    let author = *VerifiedDocument::decode(&wire, DOMAIN, NOW)
        .unwrap()
        .author();
    // Without a verified book nothing is spent or pinned.
    for create in [false, true] {
        if create {
            p.alice.mailbox_book_account().unwrap();
        }
        let before = persisted_state(&p.alice_root);
        assert!(matches!(
            p.alice.prepare_swarm_delivery(&m.id, NOW),
            Err(CoreError::MailboxBookMissing)
        ));
        assert_eq!(persisted_state(&p.alice_root), before);
    }
    let terms = fund(&mut p.alice);
    let d = p.alice.prepare_swarm_delivery(&m.id, NOW).unwrap();
    assert_eq!(
        (d.message_id.as_str(), d.conversation_id.as_str(), d.period),
        (m.id.as_str(), c.as_str(), period(NOW))
    );
    assert_eq!(d.mailbox, p.alice.swarm_mailbox(&c, false, NOW).unwrap());
    assert_eq!(d.mailbox, p.bob.swarm_mailbox(&c, true, NOW).unwrap());
    assert_eq!(
        (d.stamp.book, d.stamp.index, d.stamp.operation),
        (BOOK, 0, operation(&d.mailbox, d.period, &d.envelope))
    );
    assert_eq!(d.stamp.verify(&DOMAIN, &terms, NOW), Ok(()));
    // Holders see neither the author nor the signed wire.
    assert!(!d.envelope.windows(32).any(|w| w == author));
    assert!(
        !d.envelope
            .windows(64)
            .any(|w| w == &wire[wire.len() - 64..])
    );
    // Retries, a restart and the next period reuse the pinned mailbox and
    // slot: the recipient still reads the previous period.
    let start = NOW / PERIOD_SECONDS * PERIOD_SECONDS;
    for at in [
        NOW + 1,
        start + PERIOD_SECONDS,
        start + 2 * PERIOD_SECONDS - 1,
    ] {
        assert_eq!(
            p.alice.prepare_swarm_delivery(&m.id, at).unwrap(),
            d,
            "{at}"
        );
    }
    drop(p.alice);
    let mut alice = core(&p.alice_root, DOMAIN);
    assert_eq!(alice.prepare_swarm_delivery(&m.id, NOW + 5).unwrap(), d);
    assert_eq!(alice.mailbox_books().unwrap()[0].used, 1);
    // Once the recipient no longer reads the pinned period the message moves
    // to the current one and pays for it with the next slot.
    let later = start + 2 * PERIOD_SECONDS;
    let moved = alice.prepare_swarm_delivery(&m.id, later).unwrap();
    assert_eq!(moved.period, period(later));
    assert_eq!(
        moved.mailbox,
        alice.swarm_mailbox(&c, false, later).unwrap()
    );
    assert_eq!(moved.stamp.index, 1);
    assert_ne!(moved.envelope, d.envelope);
    assert_eq!(
        alice.prepare_swarm_delivery(&m.id, later + 1).unwrap(),
        moved
    );
    // A clock that steps back does not move it back.
    assert_eq!(alice.prepare_swarm_delivery(&m.id, NOW).unwrap(), moved);
    assert_eq!(alice.mailbox_books().unwrap()[0].used, 2);
    // Receipts for the abandoned period no longer complete it; the new
    // period's do.
    let stale: Vec<_> = (1..=7).map(|n| receipt(&d, n)).collect();
    assert!(matches!(
        alice.complete_swarm_delivery(&m.id, &swarm(), &stale),
        Err(CoreError::InvalidInput)
    ));
    let fresh: Vec<_> = (1..=7).map(|n| receipt(&moved, n)).collect();
    alice
        .complete_swarm_delivery(&m.id, &swarm(), &fresh)
        .unwrap();
    assert_eq!(sorted(alice.swarm_receipts(&m.id).unwrap().unwrap()), fresh);
    assert!(alice.swarm_outbox(10).unwrap().is_empty());
    // Only own pending conversation messages are prepared.
    assert!(matches!(
        alice.prepare_swarm_delivery(&"ab".repeat(32), NOW),
        Err(CoreError::InvalidInput)
    ));
    let from_bob = p.bob.send_message(&c, "hi", "b", NOW).unwrap();
    let bob_wire = p.bob.outbox(10).unwrap().remove(0).wire;
    alice.receive(&bob_wire, NOW).unwrap();
    assert!(matches!(
        alice.prepare_swarm_delivery(&from_bob.id, NOW),
        Err(CoreError::InvalidInput)
    ));
}

#[test]
fn a_delivery_completes_only_on_a_quorum_of_its_swarm_receipts() {
    let mut p = pair();
    let c = p.conversation.clone();
    fund(&mut p.alice);
    let m = p.alice.send_message(&c, "hello", "m", NOW).unwrap();
    let unprepared = p.alice.send_message(&c, "later", "u", NOW).unwrap();
    let d = p.alice.prepare_swarm_delivery(&m.id, NOW).unwrap();
    let good: Vec<_> = (1..=10).map(|n| receipt(&d, n)).collect();
    let q = QUORUM;
    let pending = |core: &AppCore| {
        assert_eq!(core.swarm_outbox(10).unwrap().len(), 2);
        assert_eq!(core.outbox(10).unwrap().len(), 2);
        assert_eq!(core.swarm_receipts(&m.id).unwrap(), None);
    };
    let ticket = d.stamp.ticket_id(&DOMAIN);
    let sign = |mailbox, operation, ticket, n: u8| {
        Receipt::sign(
            &DOMAIN,
            mailbox,
            operation,
            ticket,
            [n; 32],
            NOW,
            &holder(n),
        )
    };
    let mut forged = good[q - 1].clone();
    forged.signature[5] ^= 1;
    let mut twice = good[..q - 1].to_vec();
    twice.push(good[0].clone());
    let with = |bad: Receipt| {
        let mut set = good[..q - 1].to_vec();
        set.push(bad);
        set
    };
    for (label, set) in [
        ("below quorum", good[..q - 1].to_vec()),
        ("one holder twice", twice),
        (
            "one signer, other bytes",
            with(sign(d.mailbox, d.stamp.operation, ticket, 1)),
        ),
        (
            "another mailbox",
            with(sign([0xee; 32], d.stamp.operation, ticket, 7)),
        ),
        (
            "another operation",
            with(sign(d.mailbox, [0xee; 32], ticket, 7)),
        ),
        (
            "another ticket",
            with(sign(d.mailbox, d.stamp.operation, [0xee; 32], 7)),
        ),
        ("forged signature", with(forged)),
        (
            "outside the swarm",
            with(sign(d.mailbox, d.stamp.operation, ticket, 42)),
        ),
        ("a quorum plus a bad one", {
            let mut set = good[..q].to_vec();
            set.push(sign(d.mailbox, [0xee; 32], ticket, 9));
            set
        }),
    ] {
        assert!(
            matches!(
                p.alice.complete_swarm_delivery(&m.id, &swarm(), &set),
                Err(CoreError::InvalidInput)
            ),
            "{label}"
        );
        pending(&p.alice);
    }
    // The node's swarm, not any key, decides whose receipts count.
    assert!(matches!(
        p.alice
            .complete_swarm_delivery(&m.id, &swarm()[..q - 1], &good[..q]),
        Err(CoreError::InvalidInput)
    ));
    pending(&p.alice);
    assert!(matches!(
        p.alice
            .complete_swarm_delivery(&unprepared.id, &swarm(), &good),
        Err(CoreError::InvalidInput)
    ));
    p.alice
        .complete_swarm_delivery(&m.id, &swarm(), &good[..q])
        .unwrap();
    let left = |core: &AppCore| {
        assert_eq!(
            core.swarm_outbox(10).unwrap(),
            vec![SwarmPending {
                message_id: unprepared.id.clone(),
                conversation_id: c.clone(),
            }]
        );
        assert_eq!(core.outbox(10).unwrap().len(), 1);
        assert_eq!(
            sorted(core.swarm_receipts(&m.id).unwrap().unwrap()),
            good[..q].to_vec()
        );
    };
    left(&p.alice);
    assert_eq!(
        p.alice.snapshot().unwrap().conversations[0].messages[0]
            .delivery
            .phase,
        "delivered"
    );
    // Late responses are no-ops; a stored delivery is not prepared again.
    p.alice
        .complete_swarm_delivery(&m.id, &swarm(), &good)
        .unwrap();
    left(&p.alice);
    assert!(matches!(
        p.alice.prepare_swarm_delivery(&m.id, NOW),
        Err(CoreError::InvalidInput)
    ));
    drop(p.alice);
    let alice = core(&p.alice_root, DOMAIN);
    left(&alice);
}

#[test]
fn the_recipient_opens_its_incoming_mailbox_once_per_message_and_in_order() {
    let mut p = pair();
    let c = p.conversation.clone();
    let carol_root = TempDir::new().unwrap();
    let mut carol = profile(&carol_root, "Carol");
    let to_carol = connect(&mut p.alice, &mut carol);
    let terms = fund(&mut p.alice);
    let first = p.alice.send_message(&c, "the first", "1", NOW).unwrap();
    let second = p.alice.send_message(&c, "the second", "2", NOW).unwrap();
    let d1 = p.alice.prepare_swarm_delivery(&first.id, NOW).unwrap();
    let d2 = p.alice.prepare_swarm_delivery(&second.id, NOW).unwrap();
    // A gap is refused without storing anything; the reader keeps it.
    let before = persisted_state(&p.bob_root);
    assert!(matches!(
        p.bob
            .receive_swarm_envelope(&c, d2.period, &d2.envelope, NOW),
        Err(CoreError::Crypto(CryptoError::ReceiveGap))
    ));
    assert_eq!(persisted_state(&p.bob_root), before);
    assert!(texts(&p.bob).is_empty());
    let got = p
        .bob
        .receive_swarm_envelope(&c, d1.period, &d1.envelope, NOW)
        .unwrap();
    assert_eq!(got.message_id, first.id);
    let got = p
        .bob
        .receive_swarm_envelope(&c, d2.period, &d2.envelope, NOW)
        .unwrap();
    assert_eq!(got.message_id, second.id);
    // Copies from other holders are the same message and change nothing.
    let before = persisted_state(&p.bob_root);
    let again = p
        .bob
        .receive_swarm_envelope(&c, d1.period, &d1.envelope, NOW + 1)
        .unwrap();
    assert_eq!(again.message_id, first.id);
    assert_eq!(persisted_state(&p.bob_root), before);
    assert_eq!(texts(&p.bob), ["the first", "the second"]);
    // Holder receipts acknowledge swarm deliveries: no MLS receipt is queued.
    assert!(p.bob.outbox(10).unwrap().is_empty());
    assert!(p.bob.swarm_outbox(10).unwrap().is_empty());

    let third = p.alice.send_message(&c, "the third", "3", NOW).unwrap();
    let d3 = p.alice.prepare_swarm_delivery(&third.id, NOW).unwrap();
    let for_carol = p.alice.send_message(&to_carol, "Carol", "c", NOW).unwrap();
    let dc = p.alice.prepare_swarm_delivery(&for_carol.id, NOW).unwrap();
    // Both conversations spend the one profile book, slot after slot.
    for (n, d) in [&d1, &d2, &d3, &dc].into_iter().enumerate() {
        assert_eq!(
            (d.stamp.book, d.stamp.index),
            (BOOK, u32::try_from(n).unwrap())
        );
        assert_eq!(d.stamp.verify(&DOMAIN, &terms, NOW), Ok(()));
    }
    let before = persisted_state(&p.bob_root);
    let mut tampered = d3.envelope.clone();
    let middle = tampered.len() / 2;
    tampered[middle] ^= 1;
    for (label, period, envelope) in [
        ("another period", d3.period + 1, d3.envelope.clone()),
        ("previous period", d3.period - 1, d3.envelope.clone()),
        ("tampered", d3.period, tampered),
        ("another conversation", dc.period, dc.envelope.clone()),
    ] {
        assert!(
            matches!(
                p.bob.receive_swarm_envelope(&c, period, &envelope, NOW),
                Err(CoreError::Mailbox(MailboxError::Authentication))
            ),
            "{label}"
        );
    }
    // The sender's own outgoing envelope is not in its incoming mailbox.
    assert!(matches!(
        p.alice
            .receive_swarm_envelope(&c, d3.period, &d3.envelope, NOW),
        Err(CoreError::Mailbox(MailboxError::Authentication))
    ));
    assert!(matches!(
        p.bob
            .receive_swarm_envelope(&"ab".repeat(32), d3.period, &d3.envelope, NOW),
        Err(CoreError::UnknownConversation)
    ));
    assert_eq!(persisted_state(&p.bob_root), before);
    assert_eq!(
        p.bob
            .receive_swarm_envelope(&c, d3.period, &d3.envelope, NOW)
            .unwrap()
            .message_id,
        third.id
    );
    assert_eq!(texts(&p.bob), ["the first", "the second", "the third"]);
    assert_eq!(
        carol
            .receive_swarm_envelope(&to_carol, dc.period, &dc.envelope, NOW)
            .unwrap()
            .message_id,
        for_carol.id
    );
}

/// The network's units as a node tells core; the ten of `mailbox` in
/// rendezvous order.
fn network(count: u8) -> Vec<[u8; 32]> {
    (0..count).map(|n| [0x40 + n; 32]).collect()
}

fn swarm_of(mailbox: &[u8; 32], units: &[[u8; 32]]) -> Vec<[u8; 32]> {
    let members: Vec<_> = units
        .iter()
        .map(|commitment| Member {
            commitment: *commitment,
        })
        .collect();
    rendezvous(mailbox, &members, SWARM_SIZE)
        .into_iter()
        .map(|member| member.commitment)
        .collect()
}

/// Holders earn only from stamps that name them
/// (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md): once the node has told core the
/// registry's units, a stamp of a mailbox message names the holders of its
/// mailbox. The mailbox keeps that list: later messages and resends are named
/// the same even after the units change, so a resend is the same stamp its
/// receipts were given for. Core keeps the units it was told across a
/// restart, so a new mailbox is named before the node has read them again.
#[test]
fn a_stamp_names_the_holders_of_its_mailbox_and_the_mailbox_keeps_them() {
    let mut p = pair();
    let c = p.conversation.clone();
    let mailbox = p.alice.swarm_mailbox(&c, false, NOW).unwrap();
    p.alice.mailbox_book_account().unwrap();
    p.alice.add_mailbox_book(BOOK, 10, NOW + 100).unwrap();
    let units = network(14);
    p.alice.set_swarm_units(units.clone()).unwrap();
    let named = swarm_of(&mailbox, &units);
    let a = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    assert_eq!(a.holders.as_deref(), Some(named.as_slice()));
    assert_eq!(
        a.operation,
        named_operation(&mailbox, period(NOW), &swarm_digest(&named), b"a")
    );
    assert!(a.pays_for(&mailbox, period(NOW), b"a"));
    // A holder of the mailbox leaves the network.
    let fewer: Vec<_> = units.iter().copied().filter(|u| *u != named[0]).collect();
    assert_ne!(swarm_of(&mailbox, &fewer), named);
    p.alice.set_swarm_units(fewer.clone()).unwrap();
    assert_eq!(
        p.alice
            .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
            .unwrap(),
        a
    );
    let b = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"b", NOW)
        .unwrap();
    assert_eq!(b.index, 1);
    assert_eq!(b.holders.as_deref(), Some(named.as_slice()));
    drop(p.alice);
    let mut alice = core(&p.alice_root, DOMAIN);
    assert_eq!(
        alice
            .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
            .unwrap(),
        a
    );
    // The next period's mailbox names its holders among the units told last.
    let next = alice
        .swarm_mailbox(&c, false, NOW + PERIOD_SECONDS)
        .unwrap();
    let n = alice
        .stamp_mailbox(&next, period(NOW) + 1, b"a", NOW)
        .unwrap();
    assert_eq!(n.holders, Some(swarm_of(&next, &fewer)));
}

/// A mailbox first stamped before core knew the network's units keeps naming
/// nobody, so its resends stay the stamps first given out; they pay as
/// before and simply earn no holder a prize.
#[test]
fn a_mailbox_stamped_before_the_units_were_known_names_nobody() {
    let mut p = pair();
    let c = p.conversation.clone();
    let mailbox = p.alice.swarm_mailbox(&c, false, NOW).unwrap();
    p.alice.mailbox_book_account().unwrap();
    p.alice.add_mailbox_book(BOOK, 10, NOW + 100).unwrap();
    let a = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
        .unwrap();
    assert_eq!(a.holders, None);
    assert_eq!(a.operation, operation(&mailbox, period(NOW), b"a"));
    p.alice.set_swarm_units(network(14)).unwrap();
    assert_eq!(
        p.alice
            .stamp_mailbox(&mailbox, period(NOW), b"a", NOW)
            .unwrap(),
        a
    );
    let b = p
        .alice
        .stamp_mailbox(&mailbox, period(NOW), b"b", NOW)
        .unwrap();
    assert_eq!(b.holders, None);
    assert!(b.pays_for(&mailbox, period(NOW), b"b"));
}
