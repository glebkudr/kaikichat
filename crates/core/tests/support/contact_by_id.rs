//! Contact by ID (spec/contact-by-id-v1.md), core side: intro cards, requests
//! sealed to them and the recipient's policy, between real profiles.
use super::*;
use agentic_core::{CoreError, IntroCard, IntroMode, IntroOutcome, IntroPolicy, SwarmDelivery};
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, intro_mailbox_id, period};

const DAY: u64 = PERIOD_SECONDS;

struct Person {
    root: TempDir,
    core: AppCore,
    id: String,
}

/// A named profile with a stamp book, so its requests can be paid.
fn person(name: &str) -> Person {
    let root = TempDir::new().unwrap();
    let mut core = profile(&root, name);
    core.mailbox_book_account().unwrap();
    core.add_mailbox_book([0xb0; 32], 100, NOW + 120 * DAY)
        .unwrap();
    let id = core.snapshot().unwrap().identity.unwrap().network_id;
    Person { root, core, id }
}

fn addresses() -> Vec<String> {
    vec!["/ip4/127.0.0.1/tcp/4002".into()]
}

fn digest(network_id: &str) -> [u8; 32] {
    hex::decode(network_id.strip_prefix("ain1").unwrap())
        .unwrap()
        .try_into()
        .unwrap()
}

/// `from` asks the card's owner for a conversation: the conversation id and
/// the paid copy for the owner's intro mailbox.
fn request(
    from: &mut Person,
    to: &str,
    card: &IntroCard,
    operation: &str,
    now: u64,
) -> (String, SwarmDelivery) {
    let conversation = from
        .core
        .request_contact("Them", to, &card.envelope, operation, now)
        .unwrap();
    let pending = from.core.swarm_outbox(64).unwrap();
    let item = pending
        .iter()
        .find(|p| p.conversation_id == conversation.id)
        .expect("the request travels through the swarm");
    let delivery = from
        .core
        .prepare_swarm_delivery(&item.message_id, now)
        .unwrap();
    (conversation.id, delivery)
}

/// The direct copy of `from`'s only pending request.
fn direct_copy(from: &Person) -> Vec<u8> {
    let mut pending = from.core.outbox(100).unwrap();
    assert_eq!(pending.len(), 1);
    pending.remove(0).wire
}

fn contacts(core: &AppCore) -> Vec<(String, String)> {
    core.snapshot()
        .unwrap()
        .conversations
        .into_iter()
        .map(|c| (c.id, c.title))
        .collect()
}

fn policy(mode: IntroMode, daily_limit: u32, allowed: &[&Person]) -> IntroPolicy {
    IntroPolicy {
        mode,
        daily_limit,
        allowed: allowed.iter().map(|p| p.id.clone()).collect(),
    }
}

fn joined(outcome: IntroOutcome) -> String {
    match outcome {
        IntroOutcome::Joined(conversation) => conversation.id,
        _ => panic!("expected a join"),
    }
}

/// Deliver `from`'s text in `conversation` to `to` directly.
fn talk(from: &mut Person, to: &mut Person, conversation: &str, text: &str, op: &str) {
    let sent = from.core.send_message(conversation, text, op, NOW).unwrap();
    let wire = from
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|w| w.message_id == sent.id)
        .unwrap()
        .wire;
    to.core.receive(&wire, NOW).unwrap();
    assert!(
        to.core
            .snapshot()
            .unwrap()
            .conversations
            .iter()
            .find(|c| c.id == conversation)
            .unwrap()
            .messages
            .iter()
            .any(|m| m.text == text)
    );
}

#[test]
fn a_request_through_an_intro_card_joins_both_sides_and_tells_readers_nothing() {
    let mut bob = person("Bob");
    let mut alice = person("Alice");
    let mut carol = person("Carol");
    let nine: Vec<String> = (0..9).map(|n| format!("/ip4/127.0.0.1/tcp/{n}")).collect();
    assert!(matches!(
        bob.core.intro_card(nine, NOW),
        Err(CoreError::InvalidInput)
    ));
    let card = bob.core.intro_card(addresses(), NOW).unwrap();
    assert_eq!(card.envelope[0], 1);
    assert_eq!(card.expires_at, NOW + 30 * DAY);
    assert_eq!(bob.core.intro_card(addresses(), NOW + 60).unwrap(), card);
    let info = alice
        .core
        .open_intro_card(&bob.id, &card.envelope, NOW)
        .unwrap();
    assert_eq!(
        (
            info.network_id.as_str(),
            info.name.as_str(),
            &info.addresses
        ),
        (bob.id.as_str(), "Bob", &addresses())
    );
    assert_eq!(info.expires_at, card.expires_at);
    // Another profile's card under Bob's id, or an expired one, is refused,
    // whether only opened or used for a request.
    let carols = carol.core.intro_card(vec![], NOW).unwrap();
    assert!(matches!(
        alice.core.open_intro_card(&bob.id, &carols.envelope, NOW),
        Err(CoreError::Unauthorized)
    ));
    assert!(
        alice
            .core
            .open_intro_card(&bob.id, &card.envelope, card.expires_at)
            .is_err()
    );
    assert!(matches!(
        alice
            .core
            .request_contact("Them", &bob.id, &carols.envelope, "bad-1", NOW),
        Err(CoreError::Unauthorized)
    ));
    assert!(
        alice
            .core
            .request_contact("Them", &bob.id, &card.envelope, "bad-2", card.expires_at)
            .is_err()
    );
    assert!(contacts(&alice.core).is_empty());
    assert!(alice.core.outbox(100).unwrap().is_empty());

    let (conversation, delivery) = request(&mut alice, &bob.id, &card, "ask-1", NOW);
    // Safe to retry: the same operation is the same conversation, and the
    // same delivery on the same slot.
    assert_eq!(
        alice
            .core
            .request_contact("Them", &bob.id, &card.envelope, "ask-1", NOW)
            .unwrap()
            .id,
        conversation
    );
    assert_eq!(contacts(&alice.core).len(), 1);
    assert_eq!(
        alice
            .core
            .prepare_swarm_delivery(&delivery.message_id, NOW + 60)
            .unwrap(),
        delivery
    );
    assert_eq!(alice.core.mailbox_books().unwrap()[0].used, 1);
    // Paid into Bob's intro mailbox of the period, sealed to the card.
    let mailbox = intro_mailbox_id(&DOMAIN, &digest(&bob.id), period(NOW));
    assert_eq!(delivery.mailbox, mailbox);
    assert_eq!(delivery.period, period(NOW));
    assert_eq!(bob.core.intro_mailbox(&bob.id, NOW).unwrap(), mailbox);
    assert_eq!(alice.core.intro_mailbox(&bob.id, NOW).unwrap(), mailbox);
    assert_eq!(delivery.envelope[0], 2);
    assert_eq!(delivery.envelope[1..33], hex::decode(&card.id).unwrap());
    // The direct copy goes to the card's addresses.
    let direct = alice.core.outbox(100).unwrap();
    assert_eq!(direct.len(), 1);
    assert_eq!(
        (direct[0].destination.as_str(), &direct[0].addresses),
        (bob.id.as_str(), &addresses())
    );
    // Readers of the mailbox learn nothing of the requester or the
    // conversation: not its name, id, root key or group.
    let root = *VerifiedDocument::decode(&direct[0].wire, DOMAIN, NOW)
        .unwrap()
        .author();
    let hidden: [Vec<u8>; 5] = [
        b"Alice".to_vec(),
        digest(&alice.id).to_vec(),
        alice.id.as_bytes().to_vec(),
        root.to_vec(),
        hex::decode(&conversation).unwrap(),
    ];
    for needle in &hidden {
        assert!(
            !delivery
                .envelope
                .windows(needle.len())
                .any(|w| w == needle.as_slice())
        );
    }

    let IntroOutcome::Joined(joined) = bob
        .core
        .receive_intro_envelope(&delivery.envelope, NOW)
        .unwrap()
    else {
        panic!("the default policy lets a stranger in");
    };
    assert_eq!(
        (joined.id.as_str(), joined.title.as_str()),
        (conversation.as_str(), "Alice")
    );
    // The same request again changes nothing; its direct copy is a receipt
    // that ends both of the requester's deliveries.
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&delivery.envelope, NOW)
            .unwrap(),
        IntroOutcome::Ignored
    ));
    let receipt = bob
        .core
        .receive(&direct[0].wire, NOW)
        .unwrap()
        .reply
        .unwrap();
    alice.core.receive(&receipt, NOW).unwrap();
    assert!(alice.core.outbox(100).unwrap().is_empty());
    assert!(alice.core.swarm_outbox(64).unwrap().is_empty());
    assert_eq!(
        contacts(&bob.core),
        [(conversation.clone(), "Alice".into())]
    );
    talk(&mut alice, &mut bob, &conversation, "hi by ID", "m-1");
    talk(&mut bob, &mut alice, &conversation, "you too", "m-2");

    // The card serves every requester, each under its own ephemeral key;
    // under the default policy the direct copy alone joins.
    let (theirs, sealed) = request(&mut carol, &bob.id, &card, "ask-2", NOW);
    assert_ne!(sealed.envelope[33..65], delivery.envelope[33..65]);
    let receipt = bob
        .core
        .receive(&direct_copy(&carol), NOW)
        .unwrap()
        .reply
        .unwrap();
    carol.core.receive(&receipt, NOW).unwrap();
    assert!(carol.core.outbox(100).unwrap().is_empty());
    assert!(
        contacts(&bob.core)
            .iter()
            .any(|(id, title)| id == &theirs && title == "Carol")
    );
    talk(&mut carol, &mut bob, &theirs, "directly", "m-3");
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW)
            .unwrap(),
        IntroOutcome::Ignored
    ));
}

#[test]
fn cards_rotate_and_forged_foreign_or_stale_requests_are_refused() {
    let mut bob = person("Bob");
    let mut alice = person("Alice");
    let mut carol = person("Carol");
    let mut dave = person("Dave");
    let mut erin = person("Erin");
    let card = bob.core.intro_card(addresses(), NOW).unwrap();
    // Kept while at least 23 days are left, then replaced.
    assert_eq!(
        bob.core.intro_card(addresses(), NOW + 7 * DAY).unwrap(),
        card
    );
    let next = bob.core.intro_card(addresses(), NOW + 7 * DAY + 1).unwrap();
    assert_ne!(next.id, card.id);
    assert_eq!(next.expires_at, NOW + 37 * DAY + 1);
    assert_eq!(
        bob.core.intro_card(addresses(), NOW + 8 * DAY).unwrap(),
        next
    );
    // A request made with the older card while it was valid is taken until
    // 31 days after the card expired.
    let (old, sealed) = request(&mut alice, &bob.id, &card, "ask-1", NOW + 8 * DAY);
    assert_eq!(
        joined(
            bob.core
                .receive_intro_envelope(&sealed.envelope, card.expires_at + 31 * DAY - 1)
                .unwrap()
        ),
        old
    );

    let (_, sealed) = request(&mut carol, &bob.id, &next, "ask-2", NOW + 9 * DAY);
    let mut changed = sealed.envelope.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    assert!(
        bob.core
            .receive_intro_envelope(&changed, NOW + 9 * DAY)
            .is_err()
    );
    let mut unknown = sealed.envelope.clone();
    unknown[1..33].copy_from_slice(&[0x5a; 32]);
    assert!(matches!(
        bob.core.receive_intro_envelope(&unknown, NOW + 9 * DAY),
        Err(CoreError::InvalidInvitation)
    ));
    // Sealed to Carol's card, it is nothing Bob can open.
    let carols = carol.core.intro_card(vec![], NOW + 9 * DAY).unwrap();
    let (_, to_carol) = request(&mut dave, &carol.id, &carols, "ask-3", NOW + 9 * DAY);
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&to_carol.envelope, NOW + 9 * DAY),
        Err(CoreError::InvalidInvitation)
    ));
    // Nobody asks oneself.
    assert!(matches!(
        bob.core
            .request_contact("Me", &bob.id, &next.envelope, "self", NOW + 9 * DAY),
        Err(CoreError::Unauthorized)
    ));
    // A waiting request goes with its card's keys.
    bob.core
        .set_intro_policy(policy(IntroMode::Manual, 20, &[]))
        .unwrap();
    let (_, sealed) = request(&mut erin, &bob.id, &next, "ask-4", NOW + 9 * DAY);
    let IntroOutcome::Pending(waiting) = bob
        .core
        .receive_intro_envelope(&sealed.envelope, NOW + 9 * DAY)
        .unwrap()
    else {
        panic!("waits");
    };
    let gone = next.expires_at + 31 * DAY;
    assert!(
        bob.core
            .accept_intro_request(&waiting.request_id, gone)
            .is_err()
    );
    bob.core.intro_card(addresses(), gone).unwrap();
    assert!(bob.core.intro_requests().unwrap().is_empty());
    // So does a request read too late.
    let (_, late) = request(&mut dave, &bob.id, &next, "ask-5", NOW + 37 * DAY);
    assert!(matches!(
        bob.core.receive_intro_envelope(&late.envelope, gone),
        Err(CoreError::InvalidInvitation)
    ));
    assert_eq!(contacts(&bob.core).len(), 1);
}

#[test]
fn manual_and_list_policies_hold_requests_until_the_owner_decides() {
    let mut bob = person("Bob");
    assert_eq!(
        bob.core.intro_policy().unwrap(),
        IntroPolicy {
            mode: IntroMode::All,
            daily_limit: 100,
            allowed: vec![],
        }
    );
    let card = bob.core.intro_card(addresses(), NOW).unwrap();
    bob.core
        .set_intro_policy(policy(IntroMode::Manual, 20, &[]))
        .unwrap();
    let mut alice = person("Alice");
    let (conversation, sealed) = request(&mut alice, &bob.id, &card, "ask-1", NOW);
    let IntroOutcome::Pending(waiting) = bob
        .core
        .receive_intro_envelope(&sealed.envelope, NOW)
        .unwrap()
    else {
        panic!("a manual policy holds the request");
    };
    assert_eq!(
        (
            waiting.network_id.as_str(),
            waiting.name.as_str(),
            waiting.received_at
        ),
        (alice.id.as_str(), "Alice", NOW)
    );
    assert!(contacts(&bob.core).is_empty());
    // Waiting requests, the policy and the card survive a restart.
    let Person { root, core, id } = bob;
    drop(core);
    let mut bob = Person {
        core: super::core(&root, DOMAIN),
        root,
        id,
    };
    assert_eq!(
        bob.core.intro_requests().unwrap(),
        std::slice::from_ref(&waiting)
    );
    assert_eq!(bob.core.intro_policy().unwrap().mode, IntroMode::Manual);
    assert_eq!(bob.core.intro_card(addresses(), NOW + 60).unwrap(), card);
    let accepted = bob
        .core
        .accept_intro_request(&waiting.request_id, NOW + 5)
        .unwrap();
    assert_eq!(
        (accepted.id.as_str(), accepted.title.as_str()),
        (conversation.as_str(), "Alice")
    );
    assert!(bob.core.intro_requests().unwrap().is_empty());
    talk(&mut alice, &mut bob, &conversation, "accepted", "m-1");
    // Read again, an accepted request is nothing new.
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW + 6)
            .unwrap(),
        IntroOutcome::Ignored
    ));

    // A rejected request stays rejected when the mailbox is read again.
    let mut carol = person("Carol");
    let (_, sealed) = request(&mut carol, &bob.id, &card, "ask-2", NOW);
    let IntroOutcome::Pending(theirs) = bob
        .core
        .receive_intro_envelope(&sealed.envelope, NOW)
        .unwrap()
    else {
        panic!("waits");
    };
    bob.core.reject_intro_request(&theirs.request_id).unwrap();
    assert!(bob.core.intro_requests().unwrap().is_empty());
    assert!(matches!(
        bob.core.accept_intro_request(&theirs.request_id, NOW),
        Err(CoreError::InvalidInput)
    ));
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW + 1)
            .unwrap(),
        IntroOutcome::Ignored
    ));
    assert!(bob.core.intro_requests().unwrap().is_empty());
    assert_eq!(contacts(&bob.core).len(), 1);

    // One waiting request per requester: the one issued later wins, in
    // whatever order they are read.
    let mut dave = person("Dave");
    let (_, first) = request(&mut dave, &bob.id, &card, "ask-3", NOW);
    let (_, second) = request(&mut dave, &bob.id, &card, "ask-4", NOW + 1);
    // Two requests of one requester never share an ephemeral key.
    assert_ne!(first.envelope[33..65], second.envelope[33..65]);
    let IntroOutcome::Pending(newer) = bob
        .core
        .receive_intro_envelope(&second.envelope, NOW + 1)
        .unwrap()
    else {
        panic!("waits");
    };
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&first.envelope, NOW + 2)
            .unwrap(),
        IntroOutcome::Ignored
    ));
    assert_eq!(
        bob.core.intro_requests().unwrap(),
        std::slice::from_ref(&newer)
    );
    let mut eve = person("Eve");
    let (_, early) = request(&mut eve, &bob.id, &card, "ask-5", NOW);
    let (_, later) = request(&mut eve, &bob.id, &card, "ask-6", NOW + 1);
    bob.core
        .receive_intro_envelope(&early.envelope, NOW + 1)
        .unwrap();
    let IntroOutcome::Pending(replacing) = bob
        .core
        .receive_intro_envelope(&later.envelope, NOW + 2)
        .unwrap()
    else {
        panic!("waits");
    };
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&early.envelope, NOW + 3)
            .unwrap(),
        IntroOutcome::Ignored
    ));
    let mut waiting_now = bob.core.intro_requests().unwrap();
    waiting_now.sort_by(|a, b| a.request_id.cmp(&b.request_id));
    let mut expected = vec![newer, replacing];
    expected.sort_by(|a, b| a.request_id.cmp(&b.request_id));
    assert_eq!(waiting_now, expected);

    // The direct copy of a request meets the same policy, once, and its
    // receipt ends the requester's retries.
    let mut frank = person("Frank");
    let (_, sealed) = request(&mut frank, &bob.id, &card, "ask-7", NOW);
    let receipt = bob
        .core
        .receive(&direct_copy(&frank), NOW)
        .unwrap()
        .reply
        .unwrap();
    let frank_waits = |bob: &Person| {
        bob.core
            .intro_requests()
            .unwrap()
            .into_iter()
            .filter(|r| r.network_id == frank.id)
            .count()
    };
    assert_eq!(frank_waits(&bob), 1);
    frank.core.receive(&receipt, NOW).unwrap();
    assert!(frank.core.outbox(100).unwrap().is_empty());
    assert!(frank.core.swarm_outbox(64).unwrap().is_empty());
    assert!(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW)
            .is_ok()
    );
    assert_eq!(frank_waits(&bob), 1);
    assert_eq!(contacts(&bob.core).len(), 1);

    // Listed ids join at once, in list and in manual mode; others wait.
    let mut grace = person("Grace");
    let mut heidi = person("Heidi");
    let mut ivan = person("Ivan");
    bob.core
        .set_intro_policy(policy(IntroMode::List, 20, &[&grace]))
        .unwrap();
    let (_, sealed) = request(&mut grace, &bob.id, &card, "ask-8", NOW);
    joined(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW)
            .unwrap(),
    );
    let (_, sealed) = request(&mut ivan, &bob.id, &card, "ask-9", NOW);
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW)
            .unwrap(),
        IntroOutcome::Pending(_)
    ));
    bob.core
        .set_intro_policy(policy(IntroMode::Manual, 20, &[&heidi]))
        .unwrap();
    let (_, sealed) = request(&mut heidi, &bob.id, &card, "ask-10", NOW);
    joined(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW)
            .unwrap(),
    );
    // A contact asking again is ignored.
    let (_, again) = request(&mut alice, &bob.id, &card, "ask-11", NOW);
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&again.envelope, NOW)
            .unwrap(),
        IntroOutcome::Ignored
    ));
    assert_eq!(contacts(&bob.core).len(), 3);
    // A policy names network ids, at most 1000, and a limit of 0 to 1000.
    let ids = |count: u32| -> Vec<String> {
        (0..count)
            .map(|n| format!("ain1{}", hex::encode([n.to_be_bytes(); 8].concat())))
            .collect()
    };
    for bad in [
        IntroPolicy {
            mode: IntroMode::All,
            daily_limit: 20,
            allowed: vec!["not-an-id".into()],
        },
        IntroPolicy {
            mode: IntroMode::All,
            daily_limit: 1001,
            allowed: vec![],
        },
        IntroPolicy {
            mode: IntroMode::All,
            daily_limit: 20,
            allowed: ids(1001),
        },
    ] {
        assert!(matches!(
            bob.core.set_intro_policy(bad),
            Err(CoreError::InvalidInput)
        ));
    }
    bob.core
        .set_intro_policy(IntroPolicy {
            mode: IntroMode::List,
            daily_limit: 1000,
            allowed: ids(1000),
        })
        .unwrap();
    let zero = policy(IntroMode::All, 0, &[]);
    bob.core.set_intro_policy(zero.clone()).unwrap();
    assert_eq!(bob.core.intro_policy().unwrap(), zero);
}

#[test]
fn a_daily_limit_and_a_waiting_cap_bound_what_strangers_cause() {
    let mut bob = person("Bob");
    let card = bob.core.intro_card(addresses(), NOW).unwrap();
    // A listed id joins without using up the day's allowance.
    let mut listed = person("Listed");
    bob.core
        .set_intro_policy(policy(IntroMode::All, 2, &[&listed]))
        .unwrap();
    let (_, sealed) = request(&mut listed, &bob.id, &card, "ask", NOW);
    joined(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW)
            .unwrap(),
    );
    let mut outcomes = vec![];
    for n in 0..3 {
        let mut stranger = person(&format!("S{n}"));
        let (_, sealed) = request(&mut stranger, &bob.id, &card, "ask", NOW);
        outcomes.push(
            bob.core
                .receive_intro_envelope(&sealed.envelope, NOW)
                .unwrap(),
        );
    }
    assert!(matches!(
        outcomes.as_slice(),
        [
            IntroOutcome::Joined(_),
            IntroOutcome::Joined(_),
            IntroOutcome::Pending(_)
        ]
    ));
    // Listed ids join past the limit too.
    let mut also = person("Also listed");
    bob.core
        .set_intro_policy(policy(IntroMode::All, 2, &[&listed, &also]))
        .unwrap();
    let (_, sealed) = request(&mut also, &bob.id, &card, "ask", NOW);
    joined(
        bob.core
            .receive_intro_envelope(&sealed.envelope, NOW)
            .unwrap(),
    );
    // The day is the UTC period of the read, not of the request: one made
    // today, when the allowance is gone, joins when read in the next period,
    // which starts less than a day later.
    let tomorrow = (period(NOW) + 1) * DAY;
    assert!(tomorrow < NOW + DAY);
    let mut next = person("S3");
    let (_, sealed) = request(&mut next, &bob.id, &card, "ask", NOW);
    joined(
        bob.core
            .receive_intro_envelope(&sealed.envelope, tomorrow)
            .unwrap(),
    );
    // No allowance: a stranger waits.
    bob.core
        .set_intro_policy(policy(IntroMode::All, 0, &[]))
        .unwrap();
    let mut none = person("S4");
    let (_, sealed) = request(&mut none, &bob.id, &card, "ask", tomorrow);
    assert!(matches!(
        bob.core
            .receive_intro_envelope(&sealed.envelope, tomorrow)
            .unwrap(),
        IntroOutcome::Pending(_)
    ));
    // At most 32 wait; a request beyond that is dropped.
    bob.core
        .set_intro_policy(policy(IntroMode::Manual, 2, &[]))
        .unwrap();
    for n in 0..31 {
        let mut stranger = person(&format!("W{n}"));
        let (_, sealed) = request(&mut stranger, &bob.id, &card, "ask", tomorrow);
        let outcome = bob
            .core
            .receive_intro_envelope(&sealed.envelope, tomorrow)
            .unwrap();
        if n < 30 {
            assert!(matches!(outcome, IntroOutcome::Pending(_)), "request {n}");
        } else {
            assert!(matches!(outcome, IntroOutcome::Ignored), "request {n}");
        }
    }
    assert_eq!(bob.core.intro_requests().unwrap().len(), 32);
    assert_eq!(contacts(&bob.core).len(), 5);
}
