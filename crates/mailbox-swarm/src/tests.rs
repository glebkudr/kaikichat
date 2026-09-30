#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::access::{AccessError, AccessPass};
use crate::address::{
    PERIOD_SECONDS, intro_mailbox_id, mailbox_id, period, public_group_mailbox_id,
};
use crate::payout::{win_score, wins};
use crate::proof::{HolderEquivocation, ProofError, SenderEquivocation};
use crate::receipt::{HolderKey, Receipt};
use crate::select::{Member, QUORUM, SWARM_SIZE, rendezvous};
use crate::stamp::{
    BookKey, BookTerms, Stamp, StampError, book_id, named_operation, operation, swarm_digest,
    ticket_id,
};

const DOMAIN: [u8; 32] = [0xd0; 32];
const OTHER_DOMAIN: [u8; 32] = [0xd1; 32];

fn members(count: u8) -> Vec<Member> {
    (0..count)
        .map(|n| {
            let mut commitment = [0; 32];
            commitment[0] = n;
            commitment[31] = n.wrapping_mul(37);
            Member { commitment }
        })
        .collect()
}

fn key(n: u8) -> [u8; 32] {
    let mut key = [0x5a; 32];
    key[0] = n;
    key
}

// --- selection -------------------------------------------------------------

#[test]
fn a_swarm_is_ten_distinct_members_and_seven_of_them_store_a_message() {
    assert_eq!((SWARM_SIZE, QUORUM), (10, 7));
    let all = members(30);
    let swarm = rendezvous(&key(1), &all, SWARM_SIZE);
    assert_eq!(swarm.len(), 10);
    let mut distinct = swarm.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 10);
    assert!(swarm.iter().all(|m| all.contains(m)));
    // Fewer members than the swarm size: everyone holds.
    assert_eq!(rendezvous(&key(1), &all[..4], SWARM_SIZE).len(), 4);
    assert!(rendezvous(&key(1), &[], SWARM_SIZE).is_empty());
}

#[test]
fn selection_depends_only_on_the_member_set() {
    let all = members(30);
    let swarm = rendezvous(&key(2), &all, SWARM_SIZE);
    let mut reversed = all.clone();
    reversed.reverse();
    assert_eq!(rendezvous(&key(2), &reversed, SWARM_SIZE), swarm);
    let mut doubled = all.clone();
    doubled.extend(all.iter().copied());
    assert_eq!(rendezvous(&key(2), &doubled, SWARM_SIZE), swarm);
}

#[test]
fn losing_a_holder_hands_its_place_to_exactly_the_next_member() {
    let all = members(30);
    let ranked = rendezvous(&key(3), &all, all.len());
    let swarm = &ranked[..SWARM_SIZE];
    // An outsider leaving changes nothing.
    let outsider = ranked[SWARM_SIZE + 3];
    let without_outsider: Vec<_> = all.iter().copied().filter(|m| *m != outsider).collect();
    assert_eq!(rendezvous(&key(3), &without_outsider, SWARM_SIZE), swarm);
    // A holder leaving is replaced by the eleventh-ranked member only.
    let leaving = swarm[4];
    let without_holder: Vec<_> = all.iter().copied().filter(|m| *m != leaving).collect();
    let repaired = rendezvous(&key(3), &without_holder, SWARM_SIZE);
    let mut expected: Vec<_> = swarm.iter().copied().filter(|m| *m != leaving).collect();
    expected.push(ranked[SWARM_SIZE]);
    assert_eq!(repaired, expected);
}

#[test]
fn a_new_member_displaces_at_most_one_holder() {
    let all = members(30);
    let newcomers = [members(31)[30], {
        let mut commitment = [0xee; 32];
        commitment[5] = 1;
        Member { commitment }
    }];
    for k in 0..40 {
        let before = rendezvous(&key(k), &all, SWARM_SIZE);
        for newcomer in newcomers {
            let mut grown = all.clone();
            grown.push(newcomer);
            let after = rendezvous(&key(k), &grown, SWARM_SIZE);
            let kept = before.iter().filter(|m| after.contains(m)).count();
            assert!(kept >= SWARM_SIZE - 1, "key {k}: kept {kept}");
            if kept == SWARM_SIZE - 1 {
                assert!(after.contains(&newcomer));
            }
        }
    }
}

#[test]
fn different_mailboxes_spread_over_all_members() {
    let all = members(30);
    let mut load = std::collections::BTreeMap::new();
    for k in 0..=255u8 {
        for m in rendezvous(&key(k), &all, SWARM_SIZE) {
            *load.entry(m).or_insert(0u32) += 1;
        }
    }
    // 256 mailboxes x 10 holders over 30 members: about 85 each.
    assert_eq!(load.len(), 30, "every member holds some mailbox");
    assert!(load.values().all(|n| (40..=140).contains(n)), "{load:?}");
    assert_ne!(
        rendezvous(&key(1), &all, SWARM_SIZE),
        rendezvous(&key(2), &all, SWARM_SIZE)
    );
}

// --- addressing ------------------------------------------------------------

#[test]
fn a_mailbox_address_rotates_each_period_and_binds_its_inputs() {
    assert_eq!(PERIOD_SECONDS, 86_400);
    assert_eq!(period(0), 0);
    assert_eq!(period(86_399), 0);
    assert_eq!(period(86_400), 1);
    let exporter = [7; 32];
    let id = mailbox_id(&DOMAIN, &exporter, 20_000);
    assert_eq!(id, mailbox_id(&DOMAIN, &exporter, 20_000));
    assert_ne!(id, mailbox_id(&DOMAIN, &exporter, 20_001));
    assert_ne!(id, mailbox_id(&DOMAIN, &[8; 32], 20_000));
    assert_ne!(id, mailbox_id(&OTHER_DOMAIN, &exporter, 20_000));
    assert_ne!(id, exporter);
}

#[test]
fn an_intro_mailbox_binds_its_identity_domain_and_period() {
    let identity = [7; 32];
    let id = intro_mailbox_id(&DOMAIN, &identity, 20_000);
    assert_eq!(id, intro_mailbox_id(&DOMAIN, &identity, 20_000));
    assert_ne!(id, intro_mailbox_id(&DOMAIN, &identity, 20_001));
    assert_ne!(id, intro_mailbox_id(&DOMAIN, &[8; 32], 20_000));
    assert_ne!(id, intro_mailbox_id(&OTHER_DOMAIN, &identity, 20_000));
    // Never a conversation's mailbox for the same bytes.
    assert_ne!(id, mailbox_id(&DOMAIN, &identity, 20_000));
}

/// Anyone who knows an open group's reference reads its posts there.
#[test]
fn a_public_group_mailbox_binds_its_group_domain_and_period() {
    let group = [7; 32];
    let id = public_group_mailbox_id(&DOMAIN, &group, 20_000);
    assert_eq!(id, public_group_mailbox_id(&DOMAIN, &group, 20_000));
    assert_ne!(id, public_group_mailbox_id(&DOMAIN, &group, 20_001));
    assert_ne!(id, public_group_mailbox_id(&DOMAIN, &[8; 32], 20_000));
    assert_ne!(id, public_group_mailbox_id(&OTHER_DOMAIN, &group, 20_000));
    assert_ne!(id, mailbox_id(&DOMAIN, &group, 20_000));
    assert_ne!(id, intro_mailbox_id(&DOMAIN, &group, 20_000));
}

// --- stamps ----------------------------------------------------------------

fn book_key(n: u8) -> BookKey {
    BookKey::from_bytes(&[n; 32]).unwrap()
}

fn terms(key: &BookKey) -> BookTerms {
    BookTerms {
        key: key.account(),
        count: 100,
        valid_until: 5_000,
    }
}

/// The purchase contract computes the same id: `contracts/test/BookShop.t.sol`
/// pins this vector, computed independently with `cast keccak`.
#[test]
fn a_book_id_matches_the_purchase_contract() {
    let domain = [0xa1; 32];
    let account = [0x22; 20];
    let salt = [0x33; 32];
    assert_eq!(
        hex(book_id(&domain, &account, &salt)),
        "31165017cd3777e6f74336534ea20d41b9b3df2cdc92685eb57cca728191fa21"
    );
}

#[test]
fn a_stamp_is_the_book_keys_signature_over_book_index_and_operation() {
    let key = book_key(3);
    assert!(BookKey::from_bytes(&[0; 32]).is_none(), "zero is not a key");
    assert_ne!(key.account(), book_key(4).account());
    let stamp = Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key);
    assert_eq!(
        (stamp.book, stamp.index, stamp.operation),
        ([1; 32], 7, [2; 32])
    );
    assert!(matches!(stamp.signature[64], 27 | 28));
    assert_eq!(stamp.signer(&DOMAIN).unwrap(), key.account());
    assert_eq!(stamp.verify(&DOMAIN, &terms(&key), 4_999), Ok(()));
}

#[test]
fn a_stamp_fails_outside_its_book_or_for_any_changed_field() {
    let key = book_key(3);
    let stamp = Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key);
    let t = terms(&key);
    assert_eq!(stamp.verify(&DOMAIN, &t, 5_000), Err(StampError::Expired));
    let small = BookTerms { count: 7, ..t };
    assert_eq!(stamp.verify(&DOMAIN, &small, 1), Err(StampError::Index));
    let foreign = BookTerms {
        key: book_key(4).account(),
        ..t
    };
    assert_eq!(stamp.verify(&DOMAIN, &foreign, 1), Err(StampError::Signer));
    let altered = [
        Stamp {
            book: [9; 32],
            ..stamp.clone()
        },
        Stamp {
            index: 8,
            ..stamp.clone()
        },
        Stamp {
            operation: [9; 32],
            ..stamp.clone()
        },
    ];
    for changed in altered {
        assert_ne!(changed.signer(&DOMAIN).ok(), Some(key.account()));
        assert!(changed.verify(&DOMAIN, &t, 1).is_err());
    }
    assert_ne!(stamp.signer(&OTHER_DOMAIN).ok(), Some(key.account()));
    let mut garbage = stamp.clone();
    garbage.signature = [0xff; 65];
    assert_eq!(garbage.signer(&DOMAIN), Err(StampError::Signature));
    let mut bad_v = stamp.clone();
    bad_v.signature[64] = 5;
    assert_eq!(bad_v.signer(&DOMAIN), Err(StampError::Signature));
}

#[test]
fn a_ticket_names_the_slot_whatever_it_pays_for() {
    let key = book_key(3);
    let a = Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key);
    let b = Stamp::sign(&DOMAIN, [1; 32], 7, [3; 32], &key);
    assert_eq!(a.ticket_id(&DOMAIN), b.ticket_id(&DOMAIN));
    assert_eq!(a.ticket_id(&DOMAIN), ticket_id(&DOMAIN, &[1; 32], 7));
    assert_ne!(ticket_id(&DOMAIN, &[1; 32], 8), a.ticket_id(&DOMAIN));
    assert_ne!(ticket_id(&DOMAIN, &[9; 32], 7), a.ticket_id(&DOMAIN));
    assert_ne!(ticket_id(&OTHER_DOMAIN, &[1; 32], 7), a.ticket_id(&DOMAIN));
}

// --- receipts --------------------------------------------------------------

fn receipt(operation: [u8; 32], ticket: [u8; 32], key: &HolderKey) -> Receipt {
    Receipt::sign(&DOMAIN, [4; 32], operation, ticket, [5; 32], 1_000, key)
}

#[test]
fn a_receipt_names_its_holder_key_over_every_field() {
    let key = HolderKey::from_bytes(&[11; 32]).unwrap();
    let signed = receipt([2; 32], [6; 32], &key);
    assert_eq!(
        (
            signed.mailbox,
            signed.operation,
            signed.ticket,
            signed.holder,
            signed.stored_at
        ),
        ([4; 32], [2; 32], [6; 32], [5; 32], 1_000)
    );
    assert_eq!(signed.signer(&DOMAIN).unwrap(), key.account());
    let altered = [
        Receipt {
            mailbox: [9; 32],
            ..signed.clone()
        },
        Receipt {
            operation: [9; 32],
            ..signed.clone()
        },
        Receipt {
            ticket: [9; 32],
            ..signed.clone()
        },
        Receipt {
            holder: [9; 32],
            ..signed.clone()
        },
        Receipt {
            stored_at: 1_001,
            ..signed.clone()
        },
    ];
    for changed in altered {
        assert_ne!(changed.signer(&DOMAIN).ok(), Some(key.account()));
    }
    assert_ne!(signed.signer(&OTHER_DOMAIN).ok(), Some(key.account()));
}

// --- proofs ----------------------------------------------------------------

#[test]
fn two_operations_on_one_slot_prove_the_sender_equivocated() {
    let key = book_key(3);
    let first = Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key);
    let second = Stamp::sign(&DOMAIN, [1; 32], 7, [3; 32], &key);
    let proof = SenderEquivocation {
        first: first.clone(),
        second,
    };
    assert_eq!(proof.verify(&DOMAIN, &key.account()), Ok(()));
    assert_eq!(
        proof.verify(&DOMAIN, &book_key(4).account()),
        Err(ProofError::Signer)
    );
    let not_conflicting = [
        Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key),
        Stamp::sign(&DOMAIN, [1; 32], 8, [3; 32], &key),
        Stamp::sign(&DOMAIN, [9; 32], 7, [3; 32], &key),
    ];
    for second in not_conflicting {
        let proof = SenderEquivocation {
            first: first.clone(),
            second,
        };
        assert_eq!(
            proof.verify(&DOMAIN, &key.account()),
            Err(ProofError::NoConflict)
        );
    }
    let forged = SenderEquivocation {
        first: first.clone(),
        second: Stamp::sign(&DOMAIN, [1; 32], 7, [3; 32], &book_key(4)),
    };
    assert_eq!(
        forged.verify(&DOMAIN, &key.account()),
        Err(ProofError::Signer)
    );
}

#[test]
fn two_receipted_operations_on_one_slot_prove_the_holder_equivocated() {
    let key = HolderKey::from_bytes(&[11; 32]).unwrap();
    let other = HolderKey::from_bytes(&[12; 32]).unwrap();
    let first = receipt([2; 32], [6; 32], &key);
    let proof = HolderEquivocation {
        first: first.clone(),
        second: receipt([3; 32], [6; 32], &key),
    };
    assert_eq!(proof.verify(&DOMAIN, &key.account()), Ok(()));
    assert_eq!(
        proof.verify(&DOMAIN, &other.account()),
        Err(ProofError::Signer)
    );
    for second in [
        receipt([2; 32], [6; 32], &key),
        receipt([3; 32], [7; 32], &key),
    ] {
        let proof = HolderEquivocation {
            first: first.clone(),
            second,
        };
        assert_eq!(
            proof.verify(&DOMAIN, &key.account()),
            Err(ProofError::NoConflict)
        );
    }
    let forged = HolderEquivocation {
        first,
        second: receipt([3; 32], [6; 32], &other),
    };
    assert_eq!(
        forged.verify(&DOMAIN, &key.account()),
        Err(ProofError::Signer)
    );
}

// --- EVM golden vectors (computed with Foundry `cast keccak`, `cast wallet
// address` and `cast wallet sign --no-hash`, secret 0x…01) ------------------

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn secret_one() -> [u8; 32] {
    let mut secret = [0; 32];
    secret[31] = 1;
    secret
}

#[test]
fn signed_formats_and_selection_match_evm_golden_vectors() {
    let key = BookKey::from_bytes(&secret_one()).unwrap();
    let holder = HolderKey::from_bytes(&secret_one()).unwrap();
    assert_eq!(
        hex(key.account()),
        "7e5f4552091a69125d5dfcb7b8c2659029395bdf"
    );
    assert_eq!(holder.account(), key.account());
    assert_eq!(
        hex(Stamp::digest(&DOMAIN, &[1; 32], 7, &[2; 32])),
        "fe3bdca2f3c46cfe1a4f6f0234028c7e393375634d7061e6c8922bfb01bea4fb"
    );
    assert_eq!(
        hex(Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key).signature),
        "fccd998c67263a38fecda3735e5e7e2619924431c198c55b96a5000df92f4dbe\
         1fa77dcfdeb0d250abb737e288a90a6be975a05ae0778a69cdf0c3b49013d7231c"
    );
    assert_eq!(
        hex(ticket_id(&DOMAIN, &[1; 32], 7)),
        "352ef7fc8910a5f2e81a823e1dbc34ca1eef551d9e8fc7bb1dbc13f1ad4f9670"
    );
    assert_eq!(
        hex(mailbox_id(&DOMAIN, &[7; 32], 20_000)),
        "417f98630355de9448315dd9a4f3500adc57ab6259773692ab3a95e329fc54b2"
    );
    let signed = Receipt::sign(&DOMAIN, [4; 32], [2; 32], [6; 32], [5; 32], 1_000, &holder);
    assert_eq!(
        hex(signed.digest(&DOMAIN)),
        "88044432f905e9331c148db3faca7d69ed904e18c7af75148b5e1dd8353e6697"
    );
    assert_eq!(
        hex(signed.signature),
        "170afca9b9d1f789c28dbf80c2592e3f998d90af8e599859cdac9a7d739e2752\
         0088ef3715e30cd31cca21f8d03bfb8b2d9aabe2b7ba19d1bf0ce5bddc949d4e1c"
    );
    // Scores keccak256(keccak256("AIN_RENDEZVOUS_V1") ‖ key ‖ commitment),
    // highest first: 0xffaf…, 0xb663…, 0x5b38…, 0x2eb8…, 0x23c0….
    let set: Vec<_> = (1..=5u8)
        .map(|n| Member {
            commitment: [n; 32],
        })
        .collect();
    let order: Vec<u8> = rendezvous(&[0x5a; 32], &set, 5)
        .iter()
        .map(|m| m.commitment[0])
        .collect();
    assert_eq!(order, [5, 2, 3, 4, 1]);
}

// --- access passes ---------------------------------------------------------

/// A day well inside `terms`' validity once scaled: the book below lasts
/// until the end of day 30.
const PASS_DAY: u64 = 20;

fn long_terms(key: &BookKey) -> BookTerms {
    BookTerms {
        key: key.account(),
        count: 0,
        valid_until: 31 * PERIOD_SECONDS,
    }
}

#[test]
fn a_pass_is_the_book_keys_signature_over_book_peer_and_day() {
    let key = book_key(3);
    let pass = AccessPass::sign(&DOMAIN, [1; 32], [2; 32], PASS_DAY, &key);
    assert!(matches!(pass.signature[64], 27 | 28));
    assert_eq!(pass.signer(&DOMAIN).unwrap(), key.account());
    // Any moment of its own day, and of the days next to it (clock skew).
    // A book with nothing left to spend still lets its owner in.
    let t = long_terms(&key);
    for day in [PASS_DAY - 1, PASS_DAY, PASS_DAY + 1] {
        for at in [day * PERIOD_SECONDS, (day + 1) * PERIOD_SECONDS - 1] {
            assert_eq!(pass.verify(&DOMAIN, &t, at), Ok(()), "at {at}");
        }
    }
}

#[test]
fn a_pass_fails_for_another_key_an_expired_book_another_day_or_any_changed_field() {
    let key = book_key(3);
    let pass = AccessPass::sign(&DOMAIN, [1; 32], [2; 32], PASS_DAY, &key);
    let t = long_terms(&key);
    let inside = PASS_DAY * PERIOD_SECONDS + 5;
    let foreign = BookTerms {
        key: book_key(4).account(),
        ..t
    };
    assert_eq!(
        pass.verify(&DOMAIN, &foreign, inside),
        Err(AccessError::Signer)
    );
    let ended = BookTerms {
        valid_until: inside,
        ..t
    };
    assert_eq!(
        pass.verify(&DOMAIN, &ended, inside),
        Err(AccessError::Expired)
    );
    for far in [PASS_DAY - 2, PASS_DAY + 2] {
        let at = far * PERIOD_SECONDS + 5;
        assert_eq!(
            pass.verify(&DOMAIN, &t, at),
            Err(AccessError::Day),
            "day {far}"
        );
    }
    let altered = [
        AccessPass {
            book: [9; 32],
            ..pass.clone()
        },
        AccessPass {
            peer: [9; 32],
            ..pass.clone()
        },
        AccessPass {
            day: PASS_DAY + 1,
            ..pass.clone()
        },
    ];
    for changed in altered {
        assert_ne!(changed.signer(&DOMAIN).ok(), Some(key.account()));
        assert!(changed.verify(&DOMAIN, &t, inside).is_err());
    }
    assert_ne!(pass.signer(&OTHER_DOMAIN).ok(), Some(key.account()));
    let mut garbage = pass.clone();
    garbage.signature = [0xff; 65];
    assert_eq!(garbage.signer(&DOMAIN), Err(AccessError::Signature));
    assert_eq!(
        garbage.verify(&DOMAIN, &t, inside),
        Err(AccessError::Signature)
    );
}

/// Computed independently: `cast keccak` over the tag hash and the fields,
/// and `cast wallet sign --no-hash` with the private key 1.
#[test]
fn a_pass_matches_its_evm_golden_vector() {
    let key = BookKey::from_bytes(&secret_one()).unwrap();
    assert_eq!(
        hex(AccessPass::digest(&DOMAIN, &[1; 32], &[2; 32], 19_683)),
        "8aaf454aa9e8d5e8d12811ae5de8414070a889f7e058ff20e15d4824fc4a8e6e"
    );
    assert_eq!(
        hex(AccessPass::sign(&DOMAIN, [1; 32], [2; 32], 19_683, &key).signature),
        "1a89aa6437887f6be2691d85825c3fd9799779eee14631091dee8a6615ff8e2c\
         461e59b54929ba58deda7d7b6cf8914ae1e4b6dd9463cb389fbdb20a8a8665b71c"
    );
}

// --- signature canonical form ----------------------------------------------

/// secp256k1 group order n.
const ORDER: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
    0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36, 0x41, 0x41,
];

/// The same signature with s' = n − s and the recovery parity flipped.
fn high_s_twin(signature: &[u8; 65]) -> [u8; 65] {
    let mut twin = *signature;
    let mut borrow = 0i16;
    for i in (0..32).rev() {
        let d = i16::from(ORDER[i]) - i16::from(signature[32 + i]) - borrow;
        twin[32 + i] = d.rem_euclid(256) as u8;
        borrow = i16::from(d < 0);
    }
    twin[64] = if signature[64] == 27 { 28 } else { 27 };
    twin
}

#[test]
fn only_low_s_signatures_with_v_27_or_28_recover() {
    let key = book_key(3);
    let stamp = Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key);
    let twin = Stamp {
        signature: high_s_twin(&stamp.signature),
        ..stamp.clone()
    };
    assert_ne!(twin.signature, stamp.signature);
    assert_eq!(twin.signer(&DOMAIN), Err(StampError::Signature));
    for v in [0u8, 1, 29] {
        let mut changed = stamp.clone();
        changed.signature[64] = v;
        assert_eq!(changed.signer(&DOMAIN), Err(StampError::Signature));
    }
    let holder = HolderKey::from_bytes(&[11; 32]).unwrap();
    let signed = receipt([2; 32], [6; 32], &holder);
    let twin = Receipt {
        signature: high_s_twin(&signed.signature),
        ..signed.clone()
    };
    assert_eq!(twin.signer(&DOMAIN), Err(StampError::Signature));
}

#[test]
fn a_forged_stamp_fails_on_its_signer_before_any_book_limit() {
    let key = book_key(3);
    let forged = Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &book_key(4));
    let t = BookTerms {
        count: 7,
        valid_until: 10,
        ..terms(&key)
    };
    assert_eq!(forged.verify(&DOMAIN, &t, 10), Err(StampError::Signer));
}

// Book class limits (bytes, retention) are checked by the holder service
// against the envelope in phase 1; a stamp itself carries no class.

// --- proof edge cases ------------------------------------------------------

/// A second valid low-s signature over the same stamp fields, made with a
/// different nonce than RFC 6979 picks.
fn resigned(stamp: &Stamp, key: &BookKey) -> Stamp {
    use k256::ecdsa::signature::hazmat::RandomizedPrehashSigner;
    use k256::elliptic_curve::rand_core::{CryptoRng, Error, RngCore};
    struct Fixed(u8);
    impl RngCore for Fixed {
        fn next_u32(&mut self) -> u32 {
            u32::from(self.0)
        }
        fn next_u64(&mut self) -> u64 {
            u64::from(self.0)
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            dest.fill(self.0);
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
            dest.fill(self.0);
            Ok(())
        }
    }
    impl CryptoRng for Fixed {}
    let digest = Stamp::digest(&DOMAIN, &stamp.book, stamp.index, &stamp.operation);
    let signature: k256::ecdsa::Signature = key
        .0
        .sign_prehash_with_rng(&mut Fixed(0x42), &digest)
        .unwrap();
    let signature = signature.normalize_s().unwrap_or(signature);
    [27u8, 28]
        .into_iter()
        .map(|v| {
            let mut bytes = [0; 65];
            bytes[..64].copy_from_slice(&signature.to_bytes());
            bytes[64] = v;
            Stamp {
                signature: bytes,
                ..stamp.clone()
            }
        })
        .find(|s| s.signer(&DOMAIN).ok() == Some(key.account()))
        .unwrap()
}

#[test]
fn re_signing_the_same_operation_is_not_equivocation() {
    let key = book_key(3);
    let first = Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key);
    let second = resigned(&first, &key);
    assert_ne!(first.signature, second.signature);
    let proof = SenderEquivocation { first, second };
    assert_eq!(
        proof.verify(&DOMAIN, &key.account()),
        Err(ProofError::NoConflict)
    );
}

#[test]
fn both_statements_of_a_proof_must_be_the_accused_keys() {
    let key = book_key(3);
    let stranger = book_key(4);
    let proof = SenderEquivocation {
        first: Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &stranger),
        second: Stamp::sign(&DOMAIN, [1; 32], 7, [3; 32], &key),
    };
    assert_eq!(
        proof.verify(&DOMAIN, &key.account()),
        Err(ProofError::Signer)
    );
    let holder = HolderKey::from_bytes(&[11; 32]).unwrap();
    let other = HolderKey::from_bytes(&[12; 32]).unwrap();
    let proof = HolderEquivocation {
        first: receipt([2; 32], [6; 32], &other),
        second: receipt([3; 32], [6; 32], &holder),
    };
    assert_eq!(
        proof.verify(&DOMAIN, &holder.account()),
        Err(ProofError::Signer)
    );
}

#[test]
fn a_proof_holds_only_in_the_domain_it_was_signed_for() {
    let key = book_key(3);
    let proof = SenderEquivocation {
        first: Stamp::sign(&DOMAIN, [1; 32], 7, [2; 32], &key),
        second: Stamp::sign(&DOMAIN, [1; 32], 7, [3; 32], &key),
    };
    assert_eq!(
        proof.verify(&OTHER_DOMAIN, &key.account()),
        Err(ProofError::Signer)
    );
    let holder = HolderKey::from_bytes(&[11; 32]).unwrap();
    let proof = HolderEquivocation {
        first: receipt([2; 32], [6; 32], &holder),
        second: receipt([3; 32], [6; 32], &holder),
    };
    assert_eq!(
        proof.verify(&OTHER_DOMAIN, &holder.account()),
        Err(ProofError::Signer)
    );
}

#[test]
fn a_holder_conflict_is_two_operations_on_one_ticket_in_any_mailboxes() {
    let key = HolderKey::from_bytes(&[11; 32]).unwrap();
    let base = receipt([2; 32], [6; 32], &key);
    // Re-receipting the same operation later or from another mailbox (repair,
    // period rotation) is not a conflict.
    let same_operation = [
        Receipt::sign(&DOMAIN, [4; 32], [2; 32], [6; 32], [5; 32], 2_000, &key),
        Receipt::sign(&DOMAIN, [8; 32], [2; 32], [6; 32], [5; 32], 1_000, &key),
    ];
    for second in same_operation {
        let proof = HolderEquivocation {
            first: base.clone(),
            second,
        };
        assert_eq!(
            proof.verify(&DOMAIN, &key.account()),
            Err(ProofError::NoConflict)
        );
    }
    // A double spend routed through two mailboxes is still a conflict.
    let proof = HolderEquivocation {
        first: base,
        second: Receipt::sign(&DOMAIN, [8; 32], [3; 32], [6; 32], [5; 32], 2_000, &key),
    };
    assert_eq!(proof.verify(&DOMAIN, &key.account()), Ok(()));
}

#[test]
fn selection_edge_cases() {
    let all = members(4);
    assert!(rendezvous(&key(1), &all, 0).is_empty());
    let mut doubled = all.clone();
    doubled.extend(all.iter().copied());
    assert_eq!(rendezvous(&key(1), &doubled, SWARM_SIZE).len(), 4);
    // Newcomers that sort first or in the middle of the member set.
    let set = members(30);
    let first = {
        let mut commitment = [0; 32];
        commitment[31] = 1;
        commitment
    };
    assert!(!set.iter().any(|m| m.commitment == first));
    for newcomer in [first, [15; 32]].map(|commitment| Member { commitment }) {
        for k in 0..40 {
            let before = rendezvous(&key(k), &set, SWARM_SIZE);
            let mut grown = set.clone();
            grown.insert(0, newcomer);
            let after = rendezvous(&key(k), &grown, SWARM_SIZE);
            let kept = before.iter().filter(|m| after.contains(m)).count();
            assert!(kept >= SWARM_SIZE - 1);
            assert!(kept == SWARM_SIZE || after.contains(&newcomer));
        }
    }
}

#[test]
fn an_operation_is_the_tagged_keccak_of_mailbox_period_and_exact_envelope() {
    // keccak256(keccak256("AIN_OPERATION_V1") ‖ [0xa1; 32] ‖ u64be(20000) ‖
    // "hello"), via cast.
    assert_eq!(
        hex(operation(&[0xa1; 32], 20_000, b"hello")),
        "ed2955d69aa714c8539f9924fa3cd08ac885b6a57afe2fb7da3d2d96a6e17462"
    );
    assert_eq!(
        hex(operation(&[0xa1; 32], 20_001, b"hello")),
        "1e96025209d3a6103b07025015dc2bcb1e3245888ce3dc25fce2a993796d5f1f"
    );
    let base = operation(&[0xa1; 32], 20_000, b"hello");
    assert_ne!(base, operation(&[0xa1; 32], 20_000, b"hello "));
    assert_ne!(base, operation(&[0xa2; 32], 20_000, b"hello"));
    assert_ne!(
        operation(&[0xa1; 32], 20_000, b""),
        operation(&[0xa1; 32], 20_000, b"\0")
    );
}

#[test]
fn a_mailbox_is_written_around_its_period_and_kept_thirty_days_after_it() {
    use crate::address::{RETENTION_PERIODS, expires_at, live, writable};
    assert_eq!(RETENTION_PERIODS, 30);
    let p = 20_000;
    let start = p * PERIOD_SECONDS;
    // Written from the day before (clock skew) until the day after, which
    // the sender keeps while the recipient still reads it.
    for (at, ok) in [
        (start - PERIOD_SECONDS - 1, false),
        (start - PERIOD_SECONDS, true),
        (start, true),
        (start + 2 * PERIOD_SECONDS - 1, true),
        (start + 2 * PERIOD_SECONDS, false),
    ] {
        assert_eq!(writable(p, at), ok, "{at}");
    }
    // Kept until thirty whole periods after its own.
    assert_eq!(expires_at(p), (p + 1 + 30) * PERIOD_SECONDS);
    for (at, ok) in [
        (start, true),
        (expires_at(p) - 1, true),
        (expires_at(p), false),
    ] {
        assert_eq!(live(p, at), ok, "{at}");
    }
    assert!(writable(0, 0) && live(0, 0));
    // Far periods saturate instead of wrapping.
    assert_eq!(expires_at(u64::MAX), u64::MAX);
    assert!(!writable(u64::MAX, 0));
}

#[test]
fn a_book_id_commits_to_the_book_account_and_salt() {
    let account = BookKey::from_bytes(&secret_one()).unwrap().account();
    // keccak256(keccak256("AIN_BOOK_V1") ‖ DOMAIN ‖ account ‖ [0x55; 32]), via cast.
    assert_eq!(
        hex(book_id(&DOMAIN, &account, &[0x55; 32])),
        "66dade3ab857216c6fd9bbc5741a85660b50f39983ceb5a5f21560a441ddcf39"
    );
    assert_ne!(
        book_id(&DOMAIN, &account, &[0x56; 32]),
        book_id(&DOMAIN, &account, &[0x55; 32])
    );
    assert_ne!(
        book_id(&DOMAIN, &book_key(3).account(), &[0x55; 32]),
        book_id(&DOMAIN, &account, &[0x55; 32])
    );
    assert_ne!(
        book_id(&OTHER_DOMAIN, &account, &[0x55; 32]),
        book_id(&DOMAIN, &account, &[0x55; 32])
    );
}

#[test]
fn a_keys_notaries_are_the_rendezvous_of_its_notary_point() {
    use crate::select::{notaries, notary_point};
    // keccak256(keccak256("AIN_NOTARY_V1") ‖ [0x42; 32]), via cast.
    assert_eq!(
        hex(notary_point(&[0x42; 32])),
        "b3fa548aed5659fb5718e466d1a04e94ca8045e4163c35f34866f5ad28f7178b"
    );
    let members: Vec<Member> = (0..30u8)
        .map(|n| Member {
            commitment: [n; 32],
        })
        .collect();
    let chosen = notaries(&[0x42; 32], &members);
    assert_eq!(chosen.len(), SWARM_SIZE);
    assert_eq!(
        chosen,
        rendezvous(&notary_point(&[0x42; 32]), &members, SWARM_SIZE)
    );
    // Not the swarm of a mailbox with the same bytes.
    assert_ne!(chosen, rendezvous(&[0x42; 32], &members, SWARM_SIZE));
    assert_ne!(notaries(&[0x43; 32], &members), chosen);
}

// --- directory ------------------------------------------------------------------

use crate::directory::{MAX_ADDRESSES, RecordError, UnitRecord, unit_commitment};

/// What an operator bonds: the vector is `cast keccak` over the same fields.
#[test]
fn a_unit_commitment_binds_the_transport_key_and_the_receipt_account() {
    let (domain, key, receipt) = ([0xa1; 32], [0x44; 32], [0x22; 20]);
    assert_eq!(
        hex(unit_commitment(&domain, &key, &receipt)),
        "000bf7dee9500d727a3a1aa217e6407d193da1eb66e69b20e27fe5dc8448268d"
    );
    let base = unit_commitment(&domain, &key, &receipt);
    for other in [
        unit_commitment(&[0xa2; 32], &key, &receipt),
        unit_commitment(&domain, &[0x45; 32], &receipt),
        unit_commitment(&domain, &key, &[0x23; 20]),
    ] {
        assert_ne!(other, base);
    }
}

fn transport() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[0x44; 32])
}

fn record(addresses: Vec<String>, issued_at: u64) -> UnitRecord {
    let key = transport();
    UnitRecord::sign(
        &DOMAIN,
        key.verifying_key().to_bytes(),
        [0x22; 20],
        addresses,
        issued_at,
        |digest| {
            use ed25519_dalek::Signer;
            key.sign(digest).to_bytes()
        },
    )
}

#[test]
fn a_unit_record_proves_itself_against_its_commitment() {
    let addresses = vec![
        "/ip4/203.0.113.7/udp/4001/quic-v1".to_owned(),
        "/ip4/203.0.113.7/tcp/4001".to_owned(),
    ];
    let signed = record(addresses.clone(), 1_800_000_000);
    let key = transport().verifying_key().to_bytes();
    assert_eq!(
        signed.verify(&DOMAIN),
        Ok(unit_commitment(&DOMAIN, &key, &[0x22; 20]))
    );
    // Any change after signing breaks it, and so does another network.
    let mut moved = signed.clone();
    moved.addresses[1] = "/ip4/198.51.100.1/tcp/4001".into();
    let mut reordered = signed.clone();
    reordered.addresses.reverse();
    let mut joined = signed.clone();
    joined.addresses = vec![addresses.concat()];
    let mut older = signed.clone();
    older.issued_at -= 1;
    let mut other_account = signed.clone();
    other_account.receipt = [0x23; 20];
    let mut other_key = signed.clone();
    other_key.transport_key = ed25519_dalek::SigningKey::from_bytes(&[0x45; 32])
        .verifying_key()
        .to_bytes();
    for (label, bad) in [
        ("moved", moved),
        ("reordered", reordered),
        ("joined", joined),
        ("older", older),
        ("another account", other_account),
        ("another key", other_key),
    ] {
        assert_eq!(bad.verify(&DOMAIN), Err(RecordError::Signature), "{label}");
    }
    assert_eq!(signed.verify(&OTHER_DOMAIN), Err(RecordError::Signature));
    // Bounded, even when signed.
    let many: Vec<String> = (0..=MAX_ADDRESSES)
        .map(|n| format!("/ip4/10.0.0.{n}/tcp/1"))
        .collect();
    assert_eq!(
        record(many, 1).verify(&DOMAIN),
        Err(RecordError::TooManyAddresses)
    );
    assert_eq!(
        record(vec!["x".repeat(257)], 1).verify(&DOMAIN),
        Err(RecordError::AddressTooLong)
    );
}

// --- operator payouts (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md) -------------

fn unhex32(text: &str) -> [u8; 32] {
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

/// `bytes32(1)` .. `bytes32(10)`: the holders of the contract's vectors.
fn numbered_holders() -> Vec<[u8; 32]> {
    (1..=10u8)
        .map(|n| {
            let mut unit = [0; 32];
            unit[31] = n;
            unit
        })
        .collect()
}

/// `OperatorPool` recomputes a claimed stamp's operation, its slot and its
/// draw from these digests: `contracts/test/OperatorPool.t.sol` pins the
/// same vectors, computed with `cast keccak`.
#[test]
fn a_named_stamp_and_its_draw_match_the_operator_pool() {
    let holders = numbered_holders();
    let swarm = swarm_digest(&holders);
    assert_eq!(
        hex(swarm),
        "c4c4ec6d96bf24101619bb4ed2d644915cadd263f9b8e274df175130ac9f7ee9"
    );
    assert_eq!(
        hex(named_operation(&[0x11; 32], 20_833, &swarm, b"envelope")),
        "38e95e7e9e9d39732b91626f16f474f110ff67c9c3102f4e72d269bdcde7e085"
    );
    let book = unhex32("31165017cd3777e6f74336534ea20d41b9b3df2cdc92685eb57cca728191fa21");
    let ticket = ticket_id(&[0xa1; 32], &book, 7);
    assert_eq!(
        hex(ticket),
        "d45234d39174f6f5ea7e37e04b6774afcf35a17f1134a38111268845294e3d46"
    );
    let seed = unhex32("66a80b61b29ec044d14c4c8c613e762ba1fb8eeb0c454d1ee00ed6dedaa5b5c5");
    let score = win_score(&seed, &ticket);
    assert_eq!(
        hex(score),
        "35807ea272f633c31d9709e4ab801142fe20e5df6d90ba15f940734b729b8f77"
    );
    // The testnet threshold: one paid stamp in about 1111 wins; this one
    // loses, and a slot wins only strictly below the threshold.
    let threshold = unhex32("003afb7e90ff972474538ef34d6a161e4f765fd8adab9f559b3d07c6a2df8000");
    assert!(!wins(&seed, &ticket, &threshold));
    assert!(!wins(&seed, &ticket, &score));
    let mut above = score;
    above[31] += 1;
    assert!(wins(&seed, &ticket, &above));
}

/// A network smaller than a swarm names fewer holders: the rest of the
/// list is empty, as the contract reads it.
#[test]
fn fewer_holders_than_a_swarm_are_named_with_an_empty_rest() {
    let holders = numbered_holders();
    let mut padded = holders[..4].to_vec();
    padded.resize(SWARM_SIZE, [0; 32]);
    assert_eq!(swarm_digest(&holders[..4]), swarm_digest(&padded));
}

/// A stamp carries the list it names; the operation it signs commits to
/// the list, so a list swapped in transit pays for nothing.
#[test]
fn a_named_stamp_pays_only_for_its_envelope_mailbox_period_and_list() {
    let key = book_key(3);
    let mailbox = [0x21; 32];
    let holders = numbered_holders();
    let mut stamp = Stamp::sign(
        &DOMAIN,
        [1; 32],
        7,
        named_operation(&mailbox, 5, &swarm_digest(&holders), b"hello"),
        &key,
    );
    stamp.holders = Some(holders.clone());
    assert!(stamp.pays_for(&mailbox, 5, b"hello"));
    assert!(!stamp.pays_for(&[0x22; 32], 5, b"hello"));
    assert!(!stamp.pays_for(&mailbox, 6, b"hello"));
    assert!(!stamp.pays_for(&mailbox, 5, b"hellO"));
    let swapped = Stamp {
        holders: Some(holders[1..].to_vec()),
        ..stamp.clone()
    };
    assert!(!swapped.pays_for(&mailbox, 5, b"hello"));
    let dropped = Stamp {
        holders: None,
        ..stamp.clone()
    };
    assert!(!dropped.pays_for(&mailbox, 5, b"hello"));
    // More names than a swarm has places is no list.
    let mut long = holders.clone();
    long.push([0x0b; 32]);
    let mut overlong = Stamp::sign(
        &DOMAIN,
        [1; 32],
        9,
        named_operation(&mailbox, 5, &swarm_digest(&long), b"hello"),
        &key,
    );
    overlong.holders = Some(long);
    assert!(!overlong.pays_for(&mailbox, 5, b"hello"));
    // A stamp that names nobody pays as before: for its envelope in its
    // mailbox and period.
    let first = Stamp::sign(&DOMAIN, [1; 32], 8, operation(&mailbox, 5, b"hello"), &key);
    assert_eq!(first.holders, None);
    assert!(first.pays_for(&mailbox, 5, b"hello"));
    assert!(!first.pays_for(&mailbox, 5, b"hellO"));
}
