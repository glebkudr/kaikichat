//! Grant books against vectors produced independently with Foundry `cast`
//! (`fixtures/grant-book/v1.json`), plus the network rules holders apply.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use agentic_grant_book::{
    ClaimRequest, GrantBook, GrantEquivocation, GrantError, GrantRevocation, GrantRules,
    GrantTerms, SECONDS_PER_DAY, SecpKey,
};
use serde_json::{Value, json};

fn vector() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/grant-book/v1.json")).unwrap()
}

fn bytes<const N: usize>(value: &Value) -> [u8; N] {
    hex::decode(value.as_str().unwrap().trim_start_matches("0x"))
        .unwrap()
        .try_into()
        .unwrap()
}

fn key(value: &Value) -> SecpKey {
    SecpKey::from_secret(&bytes(value)).unwrap()
}

fn terms(v: &Value) -> GrantTerms {
    GrantTerms {
        domain: bytes(&v["domain"]),
        book: bytes(&v["book"]),
        day: v["day"].as_u64().unwrap(),
        serial: v["serial"].as_u64().unwrap().try_into().unwrap(),
        count: v["count"].as_u64().unwrap().try_into().unwrap(),
        expiry: v["expiry"].as_u64().unwrap(),
    }
}

fn grant() -> GrantBook {
    let v = vector();
    GrantBook::issue(terms(&v), &key(&v["serverSecret"]))
}

/// Rules under which the fixture grant (day 20721, serial 7, 50 coins,
/// expiry day + 30) is valid: 8 books fit a 400-coin cap exactly.
fn rules() -> GrantRules {
    GrantRules {
        domain: bytes(&vector()["domain"]),
        issuer_active: true,
        cap_coins: 400,
        book_size: 50,
        max_validity_days: 30,
    }
}

fn noon(day: u64) -> u64 {
    day * SECONDS_PER_DAY + SECONDS_PER_DAY / 2
}

#[test]
fn issued_grant_matches_the_independent_cast_vector() {
    let v = vector();
    let server = key(&v["serverSecret"]);
    assert_eq!(server.account(), bytes::<20>(&v["server"]));
    assert_eq!(key(&v["bookSecret"]).account(), bytes::<20>(&v["book"]));
    let grant = grant();
    assert_eq!(grant.server, bytes::<20>(&v["server"]));
    assert_eq!(grant.terms(), terms(&v));
    assert_eq!(grant.digest(), bytes::<32>(&v["grantDigest"]));
    assert_eq!(grant.signature, bytes::<65>(&v["grantSignature"]));
    assert_eq!(grant.id(), bytes::<32>(&v["bookId"]));
    assert_eq!(grant.verify_signature(), Ok(()));
}

#[test]
fn a_grant_decoded_from_cast_fields_verifies_and_round_trips_as_json() {
    let v = vector();
    let wire = json!({"domain":v["domain"],"server":v["server"],"book":v["book"],"day":v["day"],
        "serial":v["serial"],"count":v["count"],"expiry":v["expiry"],"signature":v["grantSignature"]});
    let decoded: GrantBook = serde_json::from_value(wire).unwrap();
    assert_eq!(decoded, grant());
    assert_eq!(decoded.verify_signature(), Ok(()));
    let encoded = serde_json::to_value(&decoded).unwrap();
    assert_eq!(
        encoded["server"],
        json!(v["server"].as_str().unwrap().to_lowercase())
    );
    assert_eq!(
        serde_json::from_value::<GrantBook>(encoded.clone()).unwrap(),
        decoded
    );
    // Unsigned extra fields must not ride along with a signed grant.
    let mut extra = encoded;
    extra["coins"] = json!(1_000_000);
    assert!(serde_json::from_value::<GrantBook>(extra).is_err());
}

#[test]
fn changing_any_signed_field_breaks_the_signature() {
    let edits: [fn(&mut GrantBook); 8] = [
        |g| g.domain[0] ^= 1,
        |g| g.server[19] ^= 1,
        |g| g.book[0] ^= 1,
        |g| g.day += 1,
        |g| g.serial += 1,
        |g| g.count += 1,
        |g| g.expiry += 1,
        |g| g.signature[10] ^= 1,
    ];
    for edit in edits {
        let mut changed = grant();
        edit(&mut changed);
        assert_eq!(changed.verify_signature(), Err(GrantError::BadSignature));
        assert_eq!(
            changed.check(&rules(), noon(changed.day)),
            Err(GrantError::BadSignature)
        );
    }
}

#[test]
fn high_s_and_malformed_recovery_bytes_are_rejected() {
    // The low-s twin (n - s, other parity) recovers the same key; only
    // low-s signatures are accepted, as with EVM-side checks.
    let mut high = grant();
    high.signature = bytes(&vector()["grantSignatureHighS"]);
    assert_eq!(high.verify_signature(), Err(GrantError::BadSignature));
    for v in [0u8, 1, 26, 29, 255] {
        let mut odd = grant();
        odd.signature[64] = v;
        assert_eq!(odd.verify_signature(), Err(GrantError::BadSignature));
    }
}

#[test]
fn rules_accept_the_last_serial_under_the_cap_and_reject_the_next() {
    let grant = grant();
    let day = grant.day;
    assert_eq!(grant.check(&rules(), noon(day)), Ok(()));
    let tight = GrantRules {
        cap_coins: 399,
        ..rules()
    };
    assert_eq!(grant.check(&tight, noon(day)), Err(GrantError::OverCap));
    let zero = GrantRules {
        cap_coins: 0,
        ..rules()
    };
    assert_eq!(grant.check(&zero, noon(day)), Err(GrantError::OverCap));
    // A stolen key cannot wrap the cap check around with a huge serial.
    let v = vector();
    let wrapped = GrantBook::issue(
        GrantTerms {
            serial: u32::MAX,
            ..terms(&v)
        },
        &key(&v["serverSecret"]),
    );
    let generous = GrantRules {
        cap_coins: u64::from(u32::MAX) * 49,
        ..rules()
    };
    assert_eq!(
        wrapped.check(&generous, noon(day)),
        Err(GrantError::OverCap)
    );
}

#[test]
fn rules_reject_another_network_inactive_issuer_and_other_size() {
    let grant = grant();
    let now = noon(grant.day);
    let inactive = GrantRules {
        issuer_active: false,
        ..rules()
    };
    assert_eq!(grant.check(&inactive, now), Err(GrantError::IssuerInactive));
    let other_size = GrantRules {
        book_size: 49,
        ..rules()
    };
    assert_eq!(
        grant.check(&other_size, now),
        Err(GrantError::CountMismatch)
    );
    // The real server's grant for another network (e.g. a testnet sharing
    // the key) opens a fresh serial space there and must not count here.
    let v = vector();
    let foreign = GrantBook::issue(
        GrantTerms {
            domain: bytes(&v["foreignDomain"]),
            book: bytes(&v["other"]),
            ..terms(&v)
        },
        &key(&v["serverSecret"]),
    );
    assert_eq!(foreign.digest(), bytes::<32>(&v["foreignDigest"]));
    assert_eq!(foreign.signature, bytes::<65>(&v["foreignSignature"]));
    assert_eq!(foreign.verify_signature(), Ok(()));
    assert_eq!(foreign.check(&rules(), now), Err(GrantError::WrongDomain));
}

#[test]
fn rules_bound_the_grant_to_its_day_and_validity() {
    let grant = grant();
    let day = grant.day;
    assert_eq!(
        grant.check(&rules(), day * SECONDS_PER_DAY - 1),
        Err(GrantError::FutureDay)
    );
    assert_eq!(grant.check(&rules(), day * SECONDS_PER_DAY), Ok(()));
    // day * 86400 must not wrap a far-future day back onto today.
    let v = vector();
    let wrapped = GrantBook::issue(
        GrantTerms {
            day: day + (1 << 57),
            ..terms(&v)
        },
        &key(&v["serverSecret"]),
    );
    assert_eq!(
        wrapped.check(&rules(), noon(day)),
        Err(GrantError::FutureDay)
    );
    assert!(!wrapped.first_seen_in_time(noon(day), 3_600));
    // Fixture expiry is exactly day + 30 days: it is used up to the second before.
    assert_eq!(grant.check(&rules(), grant.expiry - 1), Ok(()));
    assert_eq!(
        grant.check(&rules(), grant.expiry),
        Err(GrantError::Expired)
    );
    // A grant may live until the end of its day plus the maximum validity.
    let v = vector();
    let server = key(&v["serverSecret"]);
    let limit = (day + 1 + 30) * SECONDS_PER_DAY;
    let longest = GrantBook::issue(
        GrantTerms {
            expiry: limit,
            ..terms(&v)
        },
        &server,
    );
    assert_eq!(longest.check(&rules(), noon(day)), Ok(()));
    let too_long = GrantBook::issue(
        GrantTerms {
            expiry: limit + 1,
            ..terms(&v)
        },
        &server,
    );
    assert_eq!(
        too_long.check(&rules(), noon(day)),
        Err(GrantError::ExpiryTooFar)
    );
}

#[test]
fn the_notary_must_first_see_a_grant_on_its_own_day() {
    let grant = grant();
    let start = grant.day * SECONDS_PER_DAY;
    let end = start + SECONDS_PER_DAY;
    assert!(!grant.first_seen_in_time(start - 1, 0));
    assert!(grant.first_seen_in_time(start, 0));
    assert!(grant.first_seen_in_time(end - 1, 0));
    assert!(!grant.first_seen_in_time(end, 0));
    // Skew is allowed on both sides of the UTC day, never a whole day.
    assert!(grant.first_seen_in_time(start - 3_600, 3_600));
    assert!(!grant.first_seen_in_time(start - 3_601, 3_600));
    assert!(grant.first_seen_in_time(end + 3_599, 3_600));
    assert!(!grant.first_seen_in_time(end + 3_600, 3_600));
}

#[test]
fn claim_request_matches_cast_and_binds_every_field() {
    let v = vector();
    let book = key(&v["bookSecret"]);
    let request = ClaimRequest::sign(
        bytes(&v["domain"]),
        bytes(&v["claimNonce"]),
        v["claimCreatedAt"].as_u64().unwrap(),
        &book,
    );
    assert_eq!(request.book, bytes::<20>(&v["book"]));
    assert_eq!(request.digest(), bytes::<32>(&v["claimDigest"]));
    assert_eq!(request.signature, bytes::<65>(&v["claimSignature"]));
    assert_eq!(request.verify(), Ok(()));
    let edits: [fn(&mut ClaimRequest); 5] = [
        |r| r.domain[31] ^= 1,
        |r| r.book[0] ^= 1,
        |r| r.nonce[0] ^= 1,
        |r| r.created_at += 1,
        |r| r.signature[3] ^= 1,
    ];
    for edit in edits {
        let mut changed = request.clone();
        edit(&mut changed);
        assert_eq!(changed.verify(), Err(GrantError::BadSignature));
    }
    let wire = serde_json::to_value(&request).unwrap();
    assert_eq!(
        wire,
        json!({"domain":v["domain"],"book":v["book"].as_str().unwrap().to_lowercase(),
        "nonce":v["claimNonce"],"createdAt":v["claimCreatedAt"],"signature":v["claimSignature"]})
    );
}

#[test]
fn two_different_grants_for_one_slot_prove_the_issuer_misbehaved() {
    let v = vector();
    let server = key(&v["serverSecret"]);
    let first = grant();
    let second = GrantBook::issue(
        GrantTerms {
            book: bytes(&v["conflictingBook"]),
            ..terms(&v)
        },
        &server,
    );
    assert_eq!(second.digest(), bytes::<32>(&v["conflictingDigest"]));
    assert_eq!(second.signature, bytes::<65>(&v["conflictingSignature"]));
    assert_eq!(second.id(), first.id());
    let proof = GrantEquivocation {
        first: first.clone(),
        second: second.clone(),
    };
    assert_eq!(proof.verify(), Ok(bytes(&v["server"])));
    let reversed = GrantEquivocation {
        first: second.clone(),
        second: first.clone(),
    };
    assert_eq!(reversed.verify(), Ok(bytes(&v["server"])));
}

#[test]
fn equivocation_needs_one_slot_two_statements_and_valid_signatures() {
    let v = vector();
    let server = key(&v["serverSecret"]);
    let first = grant();
    let same = GrantEquivocation {
        first: first.clone(),
        second: first.clone(),
    };
    assert_eq!(same.verify(), Err(GrantError::NotEquivocation));
    let next_serial = GrantBook::issue(
        GrantTerms {
            serial: 8,
            book: bytes(&v["conflictingBook"]),
            ..terms(&v)
        },
        &server,
    );
    assert_eq!(next_serial.signature, bytes::<65>(&v["serial8Signature"]));
    let other_slot = GrantEquivocation {
        first: first.clone(),
        second: next_serial,
    };
    assert_eq!(other_slot.verify(), Err(GrantError::NotEquivocation));
    // Serials restart every day: the same serial tomorrow is another slot.
    let next_day = GrantBook::issue(
        GrantTerms {
            day: first.day + 1,
            book: bytes(&v["conflictingBook"]),
            ..terms(&v)
        },
        &server,
    );
    assert_eq!(next_day.signature, bytes::<65>(&v["nextDaySignature"]));
    let other_day = GrantEquivocation {
        first: first.clone(),
        second: next_day,
    };
    assert_eq!(other_day.verify(), Err(GrantError::NotEquivocation));
    let other_server = GrantBook::issue(
        GrantTerms {
            book: bytes(&v["conflictingBook"]),
            ..terms(&v)
        },
        &key(&v["otherSecret"]),
    );
    let two_issuers = GrantEquivocation {
        first: first.clone(),
        second: other_server,
    };
    assert_eq!(two_issuers.verify(), Err(GrantError::NotEquivocation));
    let mut unsigned = GrantBook::issue(
        GrantTerms {
            book: bytes(&v["conflictingBook"]),
            ..terms(&v)
        },
        &server,
    );
    unsigned.signature[5] ^= 1;
    let forged = GrantEquivocation {
        first: first.clone(),
        second: unsigned.clone(),
    };
    assert_eq!(forged.verify(), Err(GrantError::BadSignature));
    let forged_first = GrantEquivocation {
        first: unsigned,
        second: first.clone(),
    };
    assert_eq!(forged_first.verify(), Err(GrantError::BadSignature));
    // One grant signed twice (another valid low-s nonce) is not two grants.
    let mut resigned = first.clone();
    resigned.signature = bytes(&v["grantSignatureAlternate"]);
    assert_ne!(resigned.signature, first.signature);
    assert_eq!(resigned.verify_signature(), Ok(()));
    let same_terms = GrantEquivocation {
        first: first.clone(),
        second: resigned,
    };
    assert_eq!(same_terms.verify(), Err(GrantError::NotEquivocation));
    // The same slot in another network is that network's grant.
    let foreign = GrantBook::issue(
        GrantTerms {
            domain: bytes(&v["foreignDomain"]),
            book: bytes(&v["other"]),
            ..terms(&v)
        },
        &server,
    );
    let across_networks = GrantEquivocation {
        first,
        second: foreign,
    };
    assert_eq!(across_networks.verify(), Err(GrantError::NotEquivocation));
}

#[test]
fn a_revocation_withdraws_one_grant_and_only_its_issuer_can_sign_it() {
    let v = vector();
    let server = key(&v["serverSecret"]);
    let grant = grant();
    let at = noon(grant.day) + 3_600;
    let revocation = GrantRevocation::issue(&grant, at, &server);
    assert_eq!(
        (
            revocation.domain,
            revocation.server,
            revocation.book,
            revocation.expiry,
            revocation.revoked_at
        ),
        (grant.domain, grant.server, grant.id(), grant.expiry, at)
    );
    assert_eq!(revocation.verify(), Ok(()));
    assert!(revocation.revokes(&grant));
    // Another serial of the same issuer and day is another book.
    let next = GrantBook::issue(
        GrantTerms {
            serial: grant.serial + 1,
            ..terms(&v)
        },
        &server,
    );
    assert!(!revocation.revokes(&next));
    // Nobody but the issuer revokes its grants, even naming the issuer.
    let stranger = GrantRevocation::issue(&grant, at, &key(&v["otherSecret"]));
    assert_eq!(stranger.verify(), Ok(()));
    assert!(!stranger.revokes(&grant));
    let mut claimed = stranger;
    claimed.server = grant.server;
    assert_eq!(claimed.verify(), Err(GrantError::BadSignature));
    assert!(!claimed.revokes(&grant));
    let edits: [fn(&mut GrantRevocation); 6] = [
        |r| r.domain[0] ^= 1,
        |r| r.book[0] ^= 1,
        |r| r.expiry += 1,
        |r| r.revoked_at += 1,
        |r| r.signature[7] ^= 1,
        |r| r.server[0] ^= 1,
    ];
    for edit in edits {
        let mut changed = revocation.clone();
        edit(&mut changed);
        assert_eq!(changed.verify(), Err(GrantError::BadSignature));
        assert!(!changed.revokes(&grant));
    }
}
