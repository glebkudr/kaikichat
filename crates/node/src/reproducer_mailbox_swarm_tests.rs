//! Sending and receiving through the mailbox swarm on the managed-time rig:
//! ten holder runtimes, a sender and a recipient. A message is stored at a
//! quorum of verified holder receipts; the recipient reads its incoming
//! mailbox from every holder of the swarm by cursor and imports each message
//! once, in MLS order.
use super::*;
use crate::runtime::mailbox_client::Holder;
use agentic_core::SwarmDelivery;
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::address::{PERIOD_SECONDS, period};
use agentic_mailbox_swarm::select::{Member, QUORUM, SWARM_SIZE, rendezvous};
use agentic_mailbox_swarm::stamp::BookTerms;
use std::collections::{BTreeMap, BTreeSet};

const HOLDERS: usize = 10;
const BOB: usize = 10;
const ALICE: usize = 11;
const CAROL: usize = 12;
/// A fourth profile, with `Setup::dave`.
const DAVE: usize = 13;
/// The joining holder of `Setup::spare` (instead of Carol).
const SPARE: usize = 12;
/// Early in a period, so no scenario crosses a period boundary by accident.
const WALL: u64 = 19_675 * PERIOD_SECONDS + 1_000;
const STEPS: usize = 4_000;

fn unit(holder: usize) -> [u8; 32] {
    [0x40 + holder as u8; 32]
}

#[derive(Default)]
struct Setup {
    /// Holders nobody can reach: never listening, listed without addresses.
    offline: Vec<usize>,
    /// Holders that receipt as another unit than the directory lists.
    wrong_unit: Vec<usize>,
    /// Two holders whose receipt accounts Alice's directory swaps.
    swapped: Option<(usize, usize)>,
    /// Bob lists holders without addresses and is connected to none.
    bob_offline: bool,
    /// Bob's book has nothing left to spend: he reads but pays for nothing.
    bob_without_book: bool,
    /// Bob has no book at all, so no holder lets him read.
    bob_bookless: bool,
    /// Alice has no bought book; a scenario grants her one.
    alice_granted: bool,
    alice_without_directory: bool,
    /// Bob also talks to Carol.
    carol: bool,
    /// Holders know the directory, listen and replicate among themselves.
    sync: bool,
    /// Listening holders that Alice's directory lists without addresses
    /// and Alice is not connected to.
    hidden_from_alice: Vec<usize>,
    /// Node `SPARE` is a holder outside every directory until it joins.
    spare: bool,
    /// Holders know no book; the books are bought on the rig's chain.
    books_on_chain: bool,
    /// Bob listens and his invitation carries his addresses, so Alice also
    /// delivers to him directly.
    direct: bool,
    /// Nobody is given a directory or a unit: the registry on the rig's
    /// chain lists the holders' commitments, and nodes pull records. Every
    /// holder is also connected to holder 0.
    directory_from_chain: bool,
    /// Alice and Bob (and Carol) have never met: no conversation joins them.
    strangers: bool,
    /// Profiles keep their cards unpublished: the scenario checks exact
    /// counts of its own lane (slots, receipts, pulls, failures).
    silent_cards: bool,
    /// Dave, a stranger to everyone, with a book (needs `carol`).
    dave: bool,
    /// `made_group` leaves Dave out of the group.
    dave_outside: bool,
    /// Dave has no book until a scenario gives him one.
    dave_bookless: bool,
    /// Dave is connected to only this many holders, the first ones: the
    /// bootstrap routes of his network preset. The rest he finds in the
    /// directory (with `directory_from_chain`).
    dave_bootstrap: Option<usize>,
    /// The holders share one host with eleven docker bridges, like the
    /// testnet's (with `sync`): listening on every interface, each also binds
    /// an address on every bridge, which only that host reaches. Their
    /// operators name the route everyone else reaches them at.
    bridged_host: bool,
}

struct Chat {
    rig: Rig,
    /// Alice ↔ Bob.
    conversation: String,
    /// Carol ↔ Bob, with `Setup::carol`.
    carol: Option<String>,
    /// Every book the holders know.
    terms: Vec<([u8; 32], BookTerms)>,
}

/// `inviter` adds `invitee`, reachable at `addresses`; both keep the
/// conversation.
fn converse(
    rig: &mut Rig,
    inviter: usize,
    invitee: usize,
    addresses: Vec<String>,
    time: u64,
) -> String {
    let invitation = rig.nodes[invitee]
        .core
        .create_invitation(time, addresses)
        .unwrap();
    let name = if invitee == BOB { "Bob" } else { "Peer" };
    let conversation = rig.nodes[inviter]
        .core
        .add_contact(name, &invitation, time)
        .unwrap()
        .id;
    let welcome = rig.nodes[inviter].core.outbox(1).unwrap().remove(0);
    let reply = rig.nodes[invitee]
        .core
        .receive(&welcome.wire, time)
        .unwrap()
        .reply
        .unwrap();
    rig.nodes[inviter].core.receive(&reply, time).unwrap();
    conversation
}

async fn chat(setup: Setup) -> Chat {
    assert!(!(setup.carol && setup.spare));
    assert!(!setup.dave || setup.carol);
    assert!(!setup.bridged_host || setup.sync);
    assert!(setup.dave_bootstrap.is_none() || setup.directory_from_chain);
    let nodes = if setup.dave {
        14
    } else if setup.carol || setup.spare {
        13
    } else {
        12
    };
    let mut rig = Rig::new(nodes, WALL);
    rig.pass_serviced = true;
    let time = rig.clock.wall();
    let mut ends = vec![ALICE, BOB];
    for (node, name) in [(ALICE, "Alice"), (BOB, "Bob")] {
        rig.nodes[node].core.create_profile(name).unwrap();
    }
    for node in &mut rig.nodes {
        node.mailbox_client.intro.silent = setup.silent_cards;
    }
    let bob_addresses = if setup.direct {
        listen(&mut rig, BOB).await;
        rig.nodes[BOB].listeners.iter().cloned().collect()
    } else {
        vec![]
    };
    let conversation = if setup.strangers {
        String::new()
    } else {
        converse(&mut rig, ALICE, BOB, bob_addresses, time)
    };
    // One book per profile, spent by all of its conversations.
    let mut funded = Vec::new();
    if !setup.alice_granted {
        funded.push(ALICE);
    }
    if !setup.bob_bookless {
        funded.push(BOB);
    }
    let carol = setup.carol.then(|| {
        rig.nodes[CAROL].core.create_profile("Carol").unwrap();
        ends.push(CAROL);
        funded.push(CAROL);
        // Among strangers Carol has never met Bob either.
        if setup.strangers {
            String::new()
        } else {
            converse(&mut rig, CAROL, BOB, vec![], time)
        }
    });
    if setup.dave {
        rig.nodes[DAVE].core.create_profile("Dave").unwrap();
        ends.push(DAVE);
        if !setup.dave_bookless {
            funded.push(DAVE);
        }
    }
    let valid_until = time + 30 * PERIOD_SECONDS;
    let mut terms = Vec::new();
    for end in &funded {
        let core = &mut rig.nodes[*end].core;
        let key = core.mailbox_book_account().unwrap();
        let book = [0xb0 + u8::try_from(*end).unwrap(); 32];
        let count = if *end == BOB && setup.bob_without_book {
            0
        } else {
            1_000
        };
        core.add_mailbox_book(book, count, valid_until).unwrap();
        terms.push((
            book,
            BookTerms {
                key,
                count,
                valid_until,
            },
        ));
    }
    let mut holders: Vec<usize> = (0..HOLDERS).collect();
    if setup.spare {
        holders.push(SPARE);
    }
    for holder in &holders {
        let service = &mut rig.nodes[*holder].mailbox_holder;
        if !setup.directory_from_chain {
            service.set_unit(if setup.wrong_unit.contains(holder) {
                [0x77; 32]
            } else {
                unit(*holder)
            });
        }
        if !setup.books_on_chain {
            for (book, terms) in &terms {
                service.learn_book(*book, *terms).unwrap();
            }
        }
    }
    if setup.books_on_chain {
        for (book, terms) in &terms {
            rig.chain.buy(*book, record(terms));
        }
    }
    if setup.sync {
        for holder in &holders {
            if !setup.offline.contains(holder) {
                listen(&mut rig, *holder).await;
                if setup.bridged_host {
                    on_bridged_host(&mut rig, *holder);
                }
            }
        }
    }
    for holder in 0..HOLDERS {
        if setup.offline.contains(&holder) {
            continue;
        }
        for end in &ends {
            let hidden = (*end == ALICE && setup.hidden_from_alice.contains(&holder))
                || (*end == DAVE && setup.dave_bootstrap.is_some_and(|routes| holder >= routes));
            if (*end != BOB || !setup.bob_offline) && !hidden {
                rig.connect(*end, holder).await;
            }
        }
    }
    for end in ends {
        if (end == ALICE && setup.alice_without_directory) || setup.directory_from_chain {
            continue;
        }
        for holder in 0..HOLDERS {
            let listed = match setup.swapped {
                Some((a, b)) if end == ALICE && holder == a => b,
                Some((a, b)) if end == ALICE && holder == b => a,
                _ => holder,
            };
            let account = rig.nodes[listed].mailbox_holder.account();
            let hidden = end == ALICE && setup.hidden_from_alice.contains(&holder);
            let addresses = if (end == BOB && setup.bob_offline) || hidden {
                vec![]
            } else {
                rig.nodes[holder].swarm.listeners().cloned().collect()
            };
            let peer = *rig.nodes[holder].swarm.local_peer_id();
            rig.nodes[end].mailbox_client.learn_holder(
                unit(holder),
                Holder {
                    peer,
                    addresses,
                    account,
                },
            );
        }
    }
    if setup.directory_from_chain {
        let units = (0..HOLDERS)
            .map(|h| rig.nodes[h].own_commitment())
            .collect();
        rig.chain.set_units(units);
        let spare = setup.spare.then_some(SPARE);
        for holder in (1..HOLDERS).chain(spare) {
            rig.connect(holder, 0).await;
        }
    } else if setup.sync {
        let spare = setup.spare.then_some(SPARE);
        // An offline holder neither listens nor dials anyone.
        for holder in (0..HOLDERS)
            .chain(spare)
            .filter(|h| !setup.offline.contains(h))
        {
            for other in 0..HOLDERS {
                let entry = directory_entry(&rig, other);
                rig.nodes[holder]
                    .mailbox_client
                    .learn_holder(unit(other), entry);
            }
        }
    }
    Chat {
        rig,
        conversation,
        carol,
        terms,
    }
}

/// A book as the purchase contract records it.
fn record(terms: &BookTerms) -> crate::runtime::chain::BookRecord {
    crate::runtime::chain::BookRecord {
        key: terms.key,
        count: terms.count,
        valid_until: terms.valid_until,
    }
}

/// The book `chat` funds Alice with.
fn alice_book() -> [u8; 32] {
    [0xb0 + ALICE as u8; 32]
}

/// Start listening and wait for the address, like `Rig::connect` does for
/// the side it dials.
async fn listen(rig: &mut Rig, node: usize) {
    let runtime = &mut rig.nodes[node];
    runtime
        .swarm
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    loop {
        let event = runtime.swarm.select_next_some().await;
        let listening = matches!(event, SwarmEvent::NewListenAddr { .. });
        runtime.event(event);
        if listening {
            return;
        }
    }
}

/// The testnet host's docker bridges, as a holder's wildcard listener sees
/// them: an address on each, with the port of its route, sorted before it.
/// Its operator names the route others reach it at as its public address.
fn on_bridged_host(rig: &mut Rig, holder: usize) {
    let node = &mut rig.nodes[holder];
    let route = node.swarm.listeners().next().unwrap().clone();
    let Some(Protocol::Tcp(port)) = route.iter().nth(1) else {
        panic!("a TCP route: {route}");
    };
    for bridge in 0..=10 {
        node.event(SwarmEvent::NewListenAddr {
            listener_id: ListenerId::next(),
            address: format!("/ip4/10.0.{bridge}.1/tcp/{port}").parse().unwrap(),
        });
    }
    node.set_public_addresses(&[route.to_string()]).unwrap();
}

/// A holder as every directory lists it: its unit, transport peer, listen
/// addresses and receipt account.
fn directory_entry(rig: &Rig, holder: usize) -> Holder {
    Holder {
        peer: *rig.nodes[holder].swarm.local_peer_id(),
        addresses: rig.nodes[holder].swarm.listeners().cloned().collect(),
        account: rig.nodes[holder].mailbox_holder.account(),
    }
}

fn messages(rig: &Rig, node: usize, conversation: &str, own: Option<bool>) -> Vec<String> {
    rig.nodes[node]
        .core
        .snapshot()
        .unwrap()
        .conversations
        .into_iter()
        .find(|c| c.id == conversation)
        .map(|c| {
            c.messages
                .into_iter()
                .filter(|m| own.is_none_or(|own| m.own == own))
                .map(|m| m.text)
                .collect()
        })
        .unwrap_or_default()
}

fn texts(rig: &Rig, node: usize, conversation: &str) -> Vec<String> {
    messages(rig, node, conversation, None)
}

fn incoming(rig: &Rig, node: usize, conversation: &str) -> Vec<String> {
    messages(rig, node, conversation, Some(false))
}

fn stored(rig: &Rig, node: usize, id: &str) -> bool {
    rig.nodes[node].core.swarm_receipts(id).unwrap().is_some()
}

/// Holder index by receipt account, over every node that holds as a unit.
fn holders(rig: &Rig) -> BTreeMap<Account, usize> {
    (0..rig.nodes.len())
        .filter(|h| rig.nodes[*h].mailbox_holder.unit().is_some())
        .map(|h| (rig.nodes[h].mailbox_holder.account(), h))
        .collect()
}

/// Which holders signed the stored receipts of a message, each once, each
/// for the given mailbox as its own registry unit.
fn receipted(rig: &Rig, node: usize, id: &str, mailbox: &[u8; 32]) -> BTreeSet<usize> {
    let receipts = rig.nodes[node].core.swarm_receipts(id).unwrap().unwrap();
    let by_account = holders(rig);
    let signers: BTreeSet<_> = receipts
        .iter()
        .map(|r| {
            let h = by_account[&r.signer(&NETWORK_DOMAIN).unwrap()];
            assert_eq!((r.holder, &r.mailbox), (unit(h), mailbox));
            h
        })
        .collect();
    assert_eq!(signers.len(), receipts.len());
    assert!(signers.len() >= QUORUM);
    signers
}

/// Every entry a holder has in `mailbox`, read page by page.
fn entries(rig: &Rig, holder: usize, mailbox: &[u8; 32]) -> Vec<mailbox_holder::Entry> {
    let service = &rig.nodes[holder].mailbox_holder;
    let mut all = Vec::new();
    let mut after = 0;
    for _ in 0..64 {
        let page = service
            .read(mailbox, after, mailbox_holder::MAX_PAGE)
            .unwrap();
        if page.entries.is_empty() {
            break;
        }
        after = page.next;
        all.extend(page.entries);
    }
    all
}

fn held(rig: &Rig, holder: usize, mailbox: &[u8; 32]) -> usize {
    entries(rig, holder, mailbox).len()
}

/// The holder a reader that stops early would reach last.
fn last_holder(mailbox: &[u8; 32]) -> usize {
    let members: Vec<_> = (0..HOLDERS)
        .map(|h| Member {
            commitment: unit(h),
        })
        .collect();
    let last = rendezvous(mailbox, &members, SWARM_SIZE)
        .last()
        .unwrap()
        .commitment;
    (0..HOLDERS).find(|h| unit(*h) == last).unwrap()
}

/// Let `seconds` of virtual time pass: several read polls.
async fn idle(rig: &mut Rig, seconds: u64) {
    let until = rig.clock.instant() + Duration::from_secs(seconds);
    rig.run_until(STEPS, |r| r.clock.instant() >= until).await;
    assert!(rig.clock.instant() >= until, "{:?}", rig.trace);
}

/// Alice's messages, sent and prepared directly (her own sender is idle).
fn prepared(rig: &mut Rig, c: &str, words: &[String]) -> Vec<SwarmDelivery> {
    let time = rig.clock.wall();
    let alice = &mut rig.nodes[ALICE].core;
    words
        .iter()
        .enumerate()
        .map(|(n, text)| {
            let id = alice
                .send_message(c, text, &format!("p{n}"), time)
                .unwrap()
                .id;
            alice.prepare_swarm_delivery(&id, time).unwrap()
        })
        .collect()
}

fn store_at(rig: &mut Rig, holder: usize, d: &SwarmDelivery) {
    let time = rig.clock.wall();
    rig.nodes[holder]
        .mailbox_holder
        .store(d.mailbox, d.period, &d.envelope, &d.stamp, time)
        .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn a_message_is_stored_at_a_quorum_and_read_once_by_the_recipient() {
    assert_eq!(
        mailbox_holder::MAX_ENVELOPE,
        agentic_crypto::mailbox::MAX_SWARM_ENVELOPE
    );
    let Chat {
        mut rig,
        conversation: c,
        carol,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        carol: true,
        ..Setup::default()
    })
    .await;
    let from_carol = carol.unwrap();
    let time = rig.clock.wall();
    let text = "Hi through the swarm";
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, text, "a1", time)
        .unwrap();
    let wire = rig.nodes[ALICE].core.outbox(1).unwrap().remove(0).wire;
    let author = *agentic_protocol::VerifiedDocument::decode(&wire, NETWORK_DOMAIN, time)
        .unwrap()
        .author();
    // Bob reads every conversation, not only one.
    let carol_sent = rig.nodes[CAROL]
        .core
        .send_message(&from_carol, "from Carol", "c1", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent.id)
            && stored(r, CAROL, &carol_sent.id)
            && incoming(r, BOB, &c) == [text]
            && incoming(r, BOB, &from_carol) == ["from Carol"]
    })
    .await;
    assert!(stored(&rig, ALICE, &sent.id), "{:?}", rig.trace);
    assert_eq!(incoming(&rig, BOB, &c), [text]);
    assert_eq!(incoming(&rig, BOB, &from_carol), ["from Carol"]);
    // New conversations are read from the period before they began, not
    // over the whole retention window.
    let p = period(WALL);
    let info = rig.nodes[BOB].mailbox_client.info();
    let read: BTreeSet<u64> = info["reads"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["period"].as_u64().unwrap())
        .collect();
    assert_eq!(read, BTreeSet::from([p - 1, p]), "{info}");
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    receipted(&rig, ALICE, &sent.id, &to_bob);
    // The slot the message spent is on record with its notaries.
    let receipts = rig.nodes[ALICE]
        .core
        .swarm_receipts(&sent.id)
        .unwrap()
        .unwrap();
    let ticket = receipts[0].ticket;
    let noted = |r: &Rig| {
        (0..HOLDERS)
            .filter(|h| {
                matches!(
                    r.nodes[*h].mailbox_holder.notary_record(&ticket).unwrap(),
                    Some(mailbox_holder::Notarized {
                        first: mailbox_holder::Statement::Ticket(stamp),
                        ..
                    }) if stamp.ticket_id(&NETWORK_DOMAIN) == ticket
                        && stamp.book == [0xb0 + ALICE as u8; 32]
                )
            })
            .count()
    };
    rig.run_until(STEPS, |r| noted(r) >= QUORUM).await;
    assert!(noted(&rig) >= QUORUM, "{:?}", rig.trace);
    // Stored receipts replace MLS acknowledgments on both ends.
    assert!(rig.nodes[ALICE].core.outbox(10).unwrap().is_empty());
    assert!(rig.nodes[ALICE].core.swarm_outbox(10).unwrap().is_empty());
    assert!(rig.nodes[BOB].core.outbox(10).unwrap().is_empty());

    let answer = "An answer through the swarm";
    let reply = rig.nodes[BOB]
        .core
        .send_message(&c, answer, "b1", time)
        .unwrap();
    let to_alice = rig.nodes[BOB].core.swarm_mailbox(&c, false, time).unwrap();
    // Requests still in flight at the quorum complete too.
    rig.run_until(STEPS, |r| {
        stored(r, BOB, &reply.id)
            && incoming(r, ALICE, &c) == [answer]
            && (0..HOLDERS).all(|h| held(r, h, &to_bob) == 1 && held(r, h, &to_alice) == 1)
    })
    .await;
    assert_eq!(texts(&rig, ALICE, &c), [text, answer]);
    assert_eq!(texts(&rig, BOB, &c), [text, answer]);
    receipted(&rig, BOB, &reply.id, &to_alice);
    // Every holder got its copy; each paid with one slot, never again.
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &to_bob), 1, "holder {holder}");
        assert_eq!(held(&rig, holder, &to_alice), 1, "holder {holder}");
        let page = rig.nodes[holder]
            .mailbox_holder
            .read(&to_bob, 0, 16)
            .unwrap();
        let envelope = &page.entries[0].envelope;
        assert!(!envelope.windows(text.len()).any(|w| w == text.as_bytes()));
        assert!(!envelope.windows(32).any(|w| w == author));
    }
    for end in [ALICE, BOB] {
        assert_eq!(rig.nodes[end].core.mailbox_books().unwrap()[0].used, 1);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn three_offline_holders_leave_exactly_the_quorum_and_the_reader_whole() {
    let offline = vec![2, 5, 8];
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        offline: offline.clone(),
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let words = ["one", "two", "three"];
    let ids: Vec<_> = words
        .iter()
        .enumerate()
        .map(|(n, w)| {
            rig.nodes[ALICE]
                .core
                .send_message(&c, w, &format!("m{n}"), time)
                .unwrap()
                .id
        })
        .collect();
    rig.run_until(STEPS, |r| {
        ids.iter().all(|id| stored(r, ALICE, id)) && incoming(r, BOB, &c) == words
    })
    .await;
    assert_eq!(incoming(&rig, BOB, &c), words, "{:?}", rig.trace);
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    let online: BTreeSet<_> = (0..HOLDERS).filter(|h| !offline.contains(h)).collect();
    for id in &ids {
        assert_eq!(receipted(&rig, ALICE, id, &to_bob), online);
    }
    for holder in &offline {
        assert_eq!(held(&rig, *holder, &to_bob), 0);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn below_the_quorum_a_message_stays_queued_until_a_holder_returns() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        offline: vec![0, 1, 2, 3],
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "patience", "m", time)
        .unwrap();
    idle(&mut rig, 120).await;
    // Six receipts are not "stored": the job stays and keeps its one slot.
    assert!(!stored(&rig, ALICE, &sent.id));
    assert_eq!(rig.nodes[ALICE].core.swarm_outbox(10).unwrap().len(), 1);
    assert_eq!(rig.nodes[ALICE].core.mailbox_books().unwrap()[0].used, 1);
    // Reading does not wait for the sender's quorum.
    assert_eq!(incoming(&rig, BOB, &c), ["patience"]);
    rig.connect(ALICE, 0).await;
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent.id)).await;
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    assert_eq!(
        receipted(&rig, ALICE, &sent.id, &to_bob),
        BTreeSet::from([0, 4, 5, 6, 7, 8, 9])
    );
    assert_eq!(rig.nodes[ALICE].core.mailbox_books().unwrap()[0].used, 1);
}

#[tokio::test(flavor = "current_thread")]
async fn a_message_below_the_quorum_moves_to_the_period_its_recipient_reads() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        offline: vec![0, 1, 2, 3],
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "patience", "m", time)
        .unwrap();
    idle(&mut rig, 120).await;
    assert!(!stored(&rig, ALICE, &sent.id));
    let first = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    // Two periods later Bob no longer reads the first mailbox; six old
    // receipts must not mix with the new period's.
    rig.clock.advance(Duration::from_secs(2 * PERIOD_SECONDS));
    let now = rig.clock.wall();
    assert_eq!(period(now), period(time) + 2);
    rig.connect(ALICE, 0).await;
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent.id)).await;
    let moved = rig.nodes[ALICE].core.swarm_mailbox(&c, false, now).unwrap();
    assert_ne!(moved, first);
    let online = BTreeSet::from([0, 4, 5, 6, 7, 8, 9]);
    assert_eq!(receipted(&rig, ALICE, &sent.id, &moved), online);
    assert_eq!(rig.nodes[ALICE].core.mailbox_books().unwrap()[0].used, 2);
    rig.run_until(STEPS, |r| online.iter().all(|h| held(r, *h, &moved) == 1))
        .await;
    idle(&mut rig, 30).await;
    assert_eq!(incoming(&rig, BOB, &c), ["patience"]);
}

#[tokio::test(flavor = "current_thread")]
async fn receipts_bound_to_another_unit_or_key_do_not_count() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        wrong_unit: vec![1],
        swapped: Some((4, 7)),
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "check", "m", time)
        .unwrap();
    // A response arriving after the quorum is still checked and counted.
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent.id) && r.nodes[ALICE].mailbox_client.stats().rejected_receipts == 3
    })
    .await;
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    assert_eq!(
        receipted(&rig, ALICE, &sent.id, &to_bob),
        BTreeSet::from([0, 2, 3, 5, 6, 8, 9])
    );
    assert_eq!(rig.nodes[ALICE].mailbox_client.stats().rejected_receipts, 3);
}

#[tokio::test(flavor = "current_thread")]
async fn the_recipient_catches_up_after_the_sender_left_and_a_day_passed() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        bob_offline: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let words = ["while you were away", "one more"];
    let ids: Vec<_> = words
        .iter()
        .enumerate()
        .map(|(n, w)| {
            rig.nodes[ALICE]
                .core
                .send_message(&c, w, &format!("m{n}"), time)
                .unwrap()
                .id
        })
        .collect();
    rig.run_until(STEPS, |r| ids.iter().all(|id| stored(r, ALICE, id)))
        .await;
    assert!(incoming(&rig, BOB, &c).is_empty());
    // Alice leaves the network for good; Bob comes back in the next period.
    drop(rig.nodes.pop());
    rig.clock.advance(Duration::from_secs(PERIOD_SECONDS));
    assert_eq!(period(rig.clock.wall()), period(time) + 1);
    for holder in 0..HOLDERS {
        rig.connect(BOB, holder).await;
    }
    rig.run_until(STEPS, |r| incoming(r, BOB, &c) == words)
        .await;
    assert_eq!(incoming(&rig, BOB, &c), words, "{:?}", rig.trace);
}

#[tokio::test(flavor = "current_thread")]
async fn an_envelope_read_before_its_predecessor_waits_for_it() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let words: Vec<String> = ["the first", "the second"].map(String::from).to_vec();
    let d = prepared(&mut rig, &c, &words);
    for holder in 0..HOLDERS {
        store_at(&mut rig, holder, &d[1]);
    }
    rig.run_until(STEPS, |r| r.nodes[BOB].mailbox_client.buffered() == 1)
        .await;
    // Held once across ten copies and many polls, never imported early.
    idle(&mut rig, 30).await;
    assert_eq!(
        rig.nodes[BOB].mailbox_client.buffered(),
        1,
        "{:?}",
        rig.trace
    );
    assert!(incoming(&rig, BOB, &c).is_empty());
    // Nine of ten copies of the missing one are lost; the survivor is the
    // holder a reader that stops early reaches last.
    store_at(&mut rig, last_holder(&d[0].mailbox), &d[0]);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 2)
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        words,
        "{}",
        rig.nodes[BOB].mailbox_client.info()
    );
    assert_eq!(rig.nodes[BOB].mailbox_client.buffered(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn a_mailbox_longer_than_a_page_is_read_by_cursor() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let count = mailbox_holder::MAX_PAGE + 2;
    let words: Vec<String> = (0..count).map(|n| format!("message {n}")).collect();
    let d = prepared(&mut rig, &c, &words);
    // One holder has everything but the first message, more than one page.
    let holder = last_holder(&d[0].mailbox);
    for delivery in &d[1..] {
        store_at(&mut rig, holder, delivery);
    }
    rig.run_until(STEPS, |r| {
        r.nodes[BOB].mailbox_client.buffered() == count - 1
    })
    .await;
    assert_eq!(
        rig.nodes[BOB].mailbox_client.buffered(),
        count - 1,
        "{}",
        rig.nodes[BOB].mailbox_client.info()
    );
    assert!(incoming(&rig, BOB, &c).is_empty());
    store_at(&mut rig, holder, &d[0]);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == count)
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        words,
        "{}",
        rig.nodes[BOB].mailbox_client.info()
    );
    assert_eq!(rig.nodes[BOB].mailbox_client.buffered(), 0);
}

// --- replication inside the swarm -------------------------------------------

/// The holder loses its disk: a fresh store under the same key, unit and
/// books; the runtime's replication state stays.
fn wipe(rig: &mut Rig, holder: usize, terms: &[([u8; 32], BookTerms)]) {
    let path = rig._dirs[holder]
        .path()
        .join(format!("replaced-{}.db", rig.clock.wall()));
    let key = rig.nodes[holder].transport_key.clone();
    let mut service =
        mailbox_holder::Service::open(&path, &[31; 32], &key, NETWORK_DOMAIN).unwrap();
    service.set_unit(unit(holder));
    for (book, terms) in terms {
        service.learn_book(*book, *terms).unwrap();
    }
    rig.nodes[holder].mailbox_holder = service;
}

/// Holders `among` all hold exactly the same `count` entries of `mailbox`.
fn same_copies(rig: &Rig, among: &[usize], mailbox: &[u8; 32], count: usize) -> bool {
    let copies: Vec<BTreeSet<(Vec<u8>, [u8; 32])>> = among
        .iter()
        .map(|h| {
            entries(rig, *h, mailbox)
                .into_iter()
                .map(|e| (e.envelope, e.stamp.operation))
                .collect()
        })
        .collect();
    copies.iter().all(|c| c.len() == count && *c == copies[0])
}

fn replication(rig: &Rig, node: usize) -> crate::runtime::mailbox_replication::Replication {
    rig.nodes[node].mailbox_client.replication()
}

fn total(rig: &Rig, field: fn(&crate::runtime::mailbox_replication::Replication) -> u64) -> u64 {
    (0..rig.nodes.len())
        .map(|n| field(&replication(rig, n)))
        .sum()
}

/// Update `nodes`' directories: `leaving` is gone and `joining` listed.
fn move_unit(rig: &mut Rig, nodes: &[usize], leaving: usize, joining: usize) {
    let entry = directory_entry(rig, joining);
    for node in nodes {
        let client = &mut rig.nodes[*node].mailbox_client;
        client.forget_holder(&unit(leaving));
        client.learn_holder(unit(joining), entry.clone());
    }
}

/// A test-owned book every listed holder knows with `key`'s account.
fn test_book(
    rig: &mut Rig,
    holders: &[usize],
    book: [u8; 32],
    key: &agentic_mailbox_swarm::stamp::BookKey,
    count: u32,
) {
    let time = rig.clock.wall();
    for holder in holders {
        rig.nodes[*holder]
            .mailbox_holder
            .learn_book(
                book,
                BookTerms {
                    key: key.account(),
                    count,
                    valid_until: time + PERIOD_SECONDS,
                },
            )
            .unwrap();
    }
}

fn paid(
    book: [u8; 32],
    index: u32,
    mailbox: &[u8; 32],
    envelope: &[u8],
    key: &agentic_mailbox_swarm::stamp::BookKey,
) -> agentic_mailbox_swarm::stamp::Stamp {
    agentic_mailbox_swarm::stamp::Stamp::sign(
        &NETWORK_DOMAIN,
        book,
        index,
        // Every direct store in these scenarios happens in WALL's period.
        agentic_mailbox_swarm::stamp::operation(mailbox, period(WALL), envelope),
        key,
    )
}

#[tokio::test(flavor = "current_thread")]
async fn holders_the_sender_missed_get_the_message_from_the_swarm() {
    let hidden = vec![1, 4, 7];
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        sync: true,
        hidden_from_alice: hidden.clone(),
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "to all ten", "m", time)
        .unwrap();
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent.id)).await;
    let reached: BTreeSet<_> = (0..HOLDERS).filter(|h| !hidden.contains(h)).collect();
    assert_eq!(receipted(&rig, ALICE, &sent.id, &to_bob), reached);
    let all: Vec<usize> = (0..HOLDERS).collect();
    let started = rig.clock.instant();
    rig.run_until(STEPS, |r| same_copies(r, &all, &to_bob, 1))
        .await;
    assert!(same_copies(&rig, &all, &to_bob, 1), "{:?}", rig.trace);
    assert!(rig.clock.instant() - started <= Duration::from_secs(120));
    // Let the round that noticed the difference finish first.
    idle(&mut rig, 60).await;
    for h in &hidden {
        assert_eq!(replication(&rig, *h).pulled, 1);
    }
    // While the swarm agrees only summaries move: no page is read again.
    let (summaries, pages, pulled) = (
        total(&rig, |r| r.summaries),
        total(&rig, |r| r.pages),
        total(&rig, |r| r.pulled),
    );
    idle(&mut rig, 90).await;
    assert!(total(&rig, |r| r.summaries) > summaries);
    assert_eq!(total(&rig, |r| r.pages), pages);
    assert_eq!(total(&rig, |r| r.pulled), pulled);
    assert!(same_copies(&rig, &all, &to_bob, 1));
}

#[tokio::test(flavor = "current_thread")]
async fn nine_lost_copies_are_restored_from_the_last_one() {
    let Chat {
        mut rig,
        conversation: c,
        terms,
        ..
    } = chat(Setup {
        sync: true,
        bob_offline: true,
        bob_without_book: true,
        alice_without_directory: true,
        ..Setup::default()
    })
    .await;
    let count = mailbox_holder::MAX_PAGE + 2;
    let words: Vec<String> = (0..count).map(|n| format!("copy {n}")).collect();
    let d = prepared(&mut rig, &c, &words);
    let mailbox = d[0].mailbox;
    // One holder, the last a reader in rendezvous order would reach, has
    // more than a page; the other nine get it only from the swarm.
    let survivor = last_holder(&mailbox);
    for delivery in &d {
        store_at(&mut rig, survivor, delivery);
    }
    let all: Vec<usize> = (0..HOLDERS).collect();
    rig.run_until(STEPS, |r| same_copies(r, &all, &mailbox, count))
        .await;
    assert!(same_copies(&rig, &all, &mailbox, count), "{:?}", rig.trace);
    assert!(total(&rig, |r| r.pulled) >= 9 * count as u64);
    // Nine holders lose their disks; their replication state stays.
    for holder in (0..HOLDERS).filter(|h| *h != survivor) {
        wipe(&mut rig, holder, &terms);
        assert_eq!(held(&rig, holder, &mailbox), 0);
    }
    rig.run_until(STEPS, |r| same_copies(r, &all, &mailbox, count))
        .await;
    assert!(same_copies(&rig, &all, &mailbox, count), "{:?}", rig.trace);
    // Bob comes online without ever reaching the survivor.
    for holder in (0..HOLDERS).filter(|h| *h != survivor) {
        rig.connect(BOB, holder).await;
    }
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == count)
        .await;
    assert_eq!(incoming(&rig, BOB, &c), words, "{:?}", rig.trace);
}

#[tokio::test(flavor = "current_thread")]
async fn a_new_member_takes_over_its_share_of_a_mailbox() {
    let leaving = 3;
    let hidden = vec![0, 5, 8];
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        spare: true,
        hidden_from_alice: hidden.clone(),
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let first: Vec<_> = ["before", "the switch"]
        .iter()
        .enumerate()
        .map(|(n, w)| {
            rig.nodes[ALICE]
                .core
                .send_message(&c, w, &format!("m{n}"), time)
                .unwrap()
                .id
        })
        .collect();
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    let old: Vec<usize> = (0..HOLDERS).collect();
    rig.run_until(STEPS, |r| {
        first.iter().all(|id| stored(r, ALICE, id)) && same_copies(r, &old, &to_bob, 2)
    })
    .await;
    assert!(same_copies(&rig, &old, &to_bob, 2), "{:?}", rig.trace);
    let reached: BTreeSet<_> = (0..HOLDERS).filter(|h| !hidden.contains(h)).collect();
    for id in &first {
        assert_eq!(receipted(&rig, ALICE, id, &to_bob), reached);
    }
    // Holder 3 leaves the registry and the spare unit joins. Everybody but
    // the two learns it first: the spare, not yet a member by its own
    // directory, takes nothing it is offered.
    let mut others: Vec<usize> = (0..HOLDERS).filter(|h| *h != leaving).collect();
    others.extend([ALICE, BOB]);
    move_unit(&mut rig, &others, leaving, SPARE);
    idle(&mut rig, 90).await;
    assert_eq!(held(&rig, SPARE, &to_bob), 0);
    assert_eq!(replication(&rig, SPARE).pulled, 0);
    // Now the spare and the leaving holder see the new registry too.
    move_unit(&mut rig, &[SPARE, leaving], leaving, SPARE);
    let new: Vec<usize> = (0..HOLDERS)
        .filter(|h| *h != leaving)
        .chain([SPARE])
        .collect();
    rig.run_until(STEPS, |r| same_copies(r, &new, &to_bob, 2))
        .await;
    assert!(same_copies(&rig, &new, &to_bob, 2), "{:?}", rig.trace);
    // New messages go to the new swarm only: Alice reaches the six old
    // members she can and the spare.
    let later = rig.nodes[ALICE]
        .core
        .send_message(&c, "after", "m2", time)
        .unwrap()
        .id;
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &later) && same_copies(r, &new, &to_bob, 3)
    })
    .await;
    let mut expected: BTreeSet<_> = reached.clone();
    expected.remove(&leaving);
    expected.insert(SPARE);
    assert_eq!(receipted(&rig, ALICE, &later, &to_bob), expected);
    // The holder that left keeps what it had and takes nothing new.
    idle(&mut rig, 90).await;
    assert_eq!(held(&rig, leaving, &to_bob), 2);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 3)
        .await;
    assert_eq!(incoming(&rig, BOB, &c), ["before", "the switch", "after"]);
}

#[tokio::test(flavor = "current_thread")]
async fn a_unit_outside_the_swarm_of_a_mailbox_never_takes_it() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        spare: true,
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    // Eleven units: every holder's directory also lists the spare.
    let eleven: Vec<usize> = (0..HOLDERS).chain([SPARE]).collect();
    let entry = directory_entry(&rig, SPARE);
    for holder in &eleven {
        rig.nodes[*holder]
            .mailbox_client
            .learn_holder(unit(SPARE), entry.clone());
    }
    // A mailbox whose rendezvous ranks the spare eleventh.
    let members: Vec<_> = eleven
        .iter()
        .map(|h| Member {
            commitment: unit(*h),
        })
        .collect();
    let mailbox = (0..=255u8)
        .map(|k| [k; 32])
        .find(|m| rendezvous(m, &members, 11).last().unwrap().commitment == unit(SPARE))
        .unwrap();
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x63; 32]).unwrap();
    let book = [0x64; 32];
    test_book(&mut rig, &eleven, book, &key, 4);
    let time = rig.clock.wall();
    let stamp = paid(book, 0, &mailbox, b"for ten", &key);
    rig.nodes[2]
        .mailbox_holder
        .store(mailbox, period(WALL), b"for ten", &stamp, time)
        .unwrap();
    // The outsider also holds an entry of the mailbox: it offers it to the
    // members, who take it, but takes nothing from them.
    let offered = paid(book, 1, &mailbox, b"from outside", &key);
    rig.nodes[SPARE]
        .mailbox_holder
        .store(mailbox, period(WALL), b"from outside", &offered, time)
        .unwrap();
    let ten: Vec<usize> = (0..HOLDERS).collect();
    rig.run_until(STEPS, |r| same_copies(r, &ten, &mailbox, 2))
        .await;
    assert!(same_copies(&rig, &ten, &mailbox, 2), "{:?}", rig.trace);
    idle(&mut rig, 90).await;
    let outside = entries(&rig, SPARE, &mailbox);
    assert_eq!(outside.len(), 1);
    assert_eq!(outside[0].stamp, offered);
    assert_eq!(replication(&rig, SPARE).pulled, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn summaries_beyond_one_request_are_all_compared() {
    let offline: Vec<usize> = (2..HOLDERS).collect();
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        offline,
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x65; 32]).unwrap();
    let book = [0x66; 32];
    test_book(&mut rig, &[0, 1], book, &key, 200);
    let time = rig.clock.wall();
    let count = mailbox_holder::MAX_SUMMARIES + 1;
    // Holder 0 has one more mailbox than fits one summary request; holder 1
    // lacks exactly the one that sorts last.
    for k in 0..count {
        let mailbox = [u8::try_from(k).unwrap(); 32];
        let index = u32::try_from(k).unwrap();
        let stamp = paid(book, index, &mailbox, b"one", &key);
        rig.nodes[0]
            .mailbox_holder
            .store(mailbox, period(WALL), b"one", &stamp, time)
            .unwrap();
        if k + 1 < count {
            rig.nodes[1]
                .mailbox_holder
                .store(mailbox, period(WALL), b"one", &stamp, time)
                .unwrap();
        }
    }
    let last = [u8::try_from(count - 1).unwrap(); 32];
    rig.run_until(STEPS, |r| held(r, 1, &last) == 1).await;
    assert_eq!(held(&rig, 1, &last), 1, "{:?}", rig.trace);
    assert_eq!(replication(&rig, 1).pulled, 1);
}

#[tokio::test(flavor = "current_thread")]
async fn replication_finds_a_double_spent_stamp_and_keeps_the_first_operation() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let all: Vec<usize> = (0..HOLDERS).collect();
    // A sender whose book key signs two operations for slot 0.
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x61; 32]).unwrap();
    let book = [0x62; 32];
    test_book(&mut rig, &all, book, &key, 10);
    let mailbox = [0x5a; 32];
    let (one, two) = (b"first spend".to_vec(), b"second spend".to_vec());
    let (first, second) = (
        paid(book, 0, &mailbox, &one, &key),
        paid(book, 0, &mailbox, &two, &key),
    );
    // Beside it, two honest entries split between the halves merge.
    let merged = [0x5b; 32];
    let (left, right) = (
        paid(book, 1, &merged, b"left", &key),
        paid(book, 2, &merged, b"right", &key),
    );
    for holder in 0..HOLDERS {
        let service = &mut rig.nodes[holder].mailbox_holder;
        if holder < 5 {
            service
                .store(mailbox, period(WALL), &one, &first, time)
                .unwrap();
            service
                .store(merged, period(WALL), b"left", &left, time)
                .unwrap();
        } else {
            service
                .store(mailbox, period(WALL), &two, &second, time)
                .unwrap();
            service
                .store(merged, period(WALL), b"right", &right, time)
                .unwrap();
        }
    }
    // An entry paid by a key the other holders do not accept for its book
    // is never taken from the one holder that has it.
    let rogue = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x67; 32]).unwrap();
    let rogue_book = [0x68; 32];
    test_book(&mut rig, &[0], rogue_book, &rogue, 4);
    test_book(&mut rig, &all[1..], rogue_book, &key, 4);
    let unpaid = [0x5c; 32];
    let unpaid_stamp = paid(rogue_book, 0, &unpaid, b"unpaid", &rogue);
    rig.nodes[0]
        .mailbox_holder
        .store(unpaid, period(WALL), b"unpaid", &unpaid_stamp, time)
        .unwrap();
    let settled = |r: &Rig| {
        same_copies(r, &all, &merged, 2)
            && (0..HOLDERS).all(|h| r.nodes[h].mailbox_holder.equivocations().unwrap().len() == 1)
    };
    rig.run_until(STEPS, settled).await;
    assert!(settled(&rig), "{:?}", rig.trace);
    idle(&mut rig, 90).await;
    assert!(settled(&rig));
    for holder in 0..HOLDERS {
        let proofs = rig.nodes[holder].mailbox_holder.equivocations().unwrap();
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].verify(&NETWORK_DOMAIN, &key.account()), Ok(()));
        let pair = BTreeSet::from([proofs[0].first.operation, proofs[0].second.operation]);
        assert_eq!(pair, BTreeSet::from([first.operation, second.operation]));
        // Each holder keeps the operation it receipted first, and only it.
        let kept = entries(&rig, holder, &mailbox);
        let expected = if holder < 5 { &first } else { &second };
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].stamp, *expected);
        assert_eq!(held(&rig, holder, &unpaid), usize::from(holder == 0));
    }
    assert!(total(&rig, |r| r.conflicts) >= HOLDERS as u64);
}

// --- retention ---------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn a_recipient_offline_for_days_reads_every_day_it_missed() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        bob_offline: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let words = ["monday", "tuesday", "wednesday", "thursday", "friday"];
    let first = period(rig.clock.wall());
    for (n, word) in words.iter().enumerate() {
        let time = rig.clock.wall();
        let id = rig.nodes[ALICE]
            .core
            .send_message(&c, word, &format!("d{n}"), time)
            .unwrap()
            .id;
        rig.run_until(STEPS, |r| stored(r, ALICE, &id)).await;
        assert!(stored(&rig, ALICE, &id), "{:?}", rig.trace);
        rig.clock.advance(Duration::from_secs(PERIOD_SECONDS));
    }
    let now = period(rig.clock.wall());
    assert_eq!(now, first + 5);
    let through = |r: &Rig| r.nodes[BOB].core.swarm_read_through(&c).unwrap();
    assert_eq!(through(&rig), None);
    // Bob reaches one holder fewer than the quorum: he reads every message,
    // but a period counts as read only through seven holders.
    for holder in 0..QUORUM - 1 {
        rig.connect(BOB, holder).await;
    }
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == words.len())
        .await;
    assert_eq!(incoming(&rig, BOB, &c), words, "{:?}", rig.trace);
    idle(&mut rig, 30).await;
    assert_eq!(through(&rig), None);
    rig.connect(BOB, QUORUM - 1).await;
    // Days no one can write any more are remembered as read, never the two
    // still writable, and are not read again.
    rig.run_until(STEPS, |r| through(r) == Some(now - 2)).await;
    assert_eq!(
        through(&rig),
        Some(now - 2),
        "{}",
        rig.nodes[BOB].mailbox_client.info()
    );
    idle(&mut rig, 30).await;
    assert_eq!(through(&rig), Some(now - 2));
    let info = rig.nodes[BOB].mailbox_client.info();
    // The conversation's mailboxes; the intro ones follow at their own pace.
    let read: Vec<u64> = info["reads"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["conversationId"] == c.as_str())
        .map(|r| r["period"].as_u64().unwrap())
        .collect();
    assert!(!read.is_empty());
    assert!(read.iter().all(|p| *p > now - 2), "{info}");
    assert_eq!(incoming(&rig, BOB, &c), words);
}

#[tokio::test(flavor = "current_thread")]
async fn a_period_counts_as_read_only_after_its_writes_closed() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let p = period(rig.clock.wall());
    let d = prepared(&mut rig, &c, &["at the last moment".to_string()]);
    assert_eq!(d[0].period, p);
    // Bob reads period p to its (empty) end while it can still be written.
    rig.clock.advance(Duration::from_secs(
        (p + 2) * PERIOD_SECONDS - 20 - rig.clock.wall(),
    ));
    idle(&mut rig, 10).await;
    assert!(rig.clock.wall() < (p + 2) * PERIOD_SECONDS);
    // The last write lands in its final seconds at a bare quorum, before
    // Bob reads again.
    for holder in HOLDERS - QUORUM..HOLDERS {
        store_at(&mut rig, holder, &d[0]);
    }
    rig.clock.advance(Duration::from_secs(
        (p + 2) * PERIOD_SECONDS + 1 - rig.clock.wall(),
    ));
    rig.run_until(STEPS, |r| {
        incoming(r, BOB, &c).len() == 1
            && r.nodes[BOB].core.swarm_read_through(&c).unwrap() == Some(p)
    })
    .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["at the last moment"],
        "{:?}",
        rig.trace
    );
    assert_eq!(rig.nodes[BOB].core.swarm_read_through(&c).unwrap(), Some(p));
}

#[tokio::test(flavor = "current_thread")]
async fn holders_forget_a_mailbox_thirty_days_after_its_period() {
    use agentic_mailbox_swarm::address::expires_at;
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "for a month", "m", time)
        .unwrap()
        .id;
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    let all: Vec<usize> = (0..HOLDERS).collect();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent)
            && same_copies(r, &all, &to_bob, 1)
            && incoming(r, BOB, &c) == ["for a month"]
    })
    .await;
    assert!(same_copies(&rig, &all, &to_bob, 1), "{:?}", rig.trace);
    // Shortly before the end it is still there.
    let end = expires_at(period(time));
    rig.clock
        .advance(Duration::from_secs(end - 90 - rig.clock.wall()));
    idle(&mut rig, 60).await;
    assert!(rig.clock.wall() < end);
    assert!(same_copies(&rig, &all, &to_bob, 1));
    rig.clock
        .advance(Duration::from_secs(end - rig.clock.wall()));
    idle(&mut rig, 60).await;
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &to_bob), 0, "holder {holder}");
        assert!(
            rig.nodes[holder]
                .mailbox_holder
                .mailboxes(None, 64)
                .unwrap()
                .is_empty()
        );
    }
    // Nothing is compared or pulled for it any more.
    let (summaries, pages) = (total(&rig, |r| r.summaries), total(&rig, |r| r.pages));
    idle(&mut rig, 90).await;
    assert_eq!(total(&rig, |r| r.summaries), summaries);
    assert_eq!(total(&rig, |r| r.pages), pages);
    assert_eq!(incoming(&rig, BOB, &c), ["for a month"]);
}

#[tokio::test(flavor = "current_thread")]
async fn a_copy_is_restored_after_its_book_expired() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x69; 32]).unwrap();
    let book = [0x6a; 32];
    // The book ends one day after the entry is paid.
    test_book(&mut rig, &all, book, &key, 4);
    let time = rig.clock.wall();
    let terms = vec![(
        book,
        BookTerms {
            key: key.account(),
            count: 4,
            valid_until: time + PERIOD_SECONDS,
        },
    )];
    let mailbox = [0x5d; 32];
    let stamp = paid(book, 0, &mailbox, b"paid in time", &key);
    for holder in 0..HOLDERS {
        rig.nodes[holder]
            .mailbox_holder
            .store(mailbox, period(WALL), b"paid in time", &stamp, time)
            .unwrap();
    }
    rig.clock.advance(Duration::from_secs(2 * PERIOD_SECONDS));
    let later = rig.clock.wall();
    // The book can no longer pay for anything new.
    let fresh = [0x5e; 32];
    let late = agentic_mailbox_swarm::stamp::Stamp::sign(
        &NETWORK_DOMAIN,
        book,
        1,
        agentic_mailbox_swarm::stamp::operation(&fresh, period(later), b"late"),
        &key,
    );
    assert_eq!(
        rig.nodes[0]
            .mailbox_holder
            .store(fresh, period(later), b"late", &late, later),
        Err(mailbox_holder::Refusal::Stamp(
            agentic_mailbox_swarm::stamp::StampError::Expired
        ))
    );
    // A holder that lost its disk still gets the paid copy back.
    wipe(&mut rig, 4, &terms);
    assert_eq!(held(&rig, 4, &mailbox), 0);
    rig.run_until(STEPS, |r| same_copies(r, &all, &mailbox, 1))
        .await;
    assert!(same_copies(&rig, &all, &mailbox, 1), "{:?}", rig.trace);
    assert_eq!(replication(&rig, 4).pulled, 1);
}

// --- notary ----------------------------------------------------------------------

/// Twelve holders whose directories also list eight units nobody reaches:
/// swarms and notary sets are drawn from twenty units, so two mailboxes'
/// swarms can miss each other.
async fn wide() -> (Rig, Vec<[u8; 32]>) {
    let mut rig = Rig::new(12, WALL);
    rig.pass_serviced = true;
    let real = 12;
    for holder in 0..real {
        rig.nodes[holder].mailbox_holder.set_unit(unit(holder));
        listen(&mut rig, holder).await;
    }
    let fake: Vec<[u8; 32]> = (0..8u8).map(|i| [0x90 + i; 32]).collect();
    for holder in 0..real {
        for other in 0..real {
            let entry = directory_entry(&rig, other);
            rig.nodes[holder]
                .mailbox_client
                .learn_holder(unit(other), entry);
        }
        for f in &fake {
            rig.nodes[holder].mailbox_client.learn_holder(
                *f,
                Holder {
                    peer: PeerId::random(),
                    addresses: vec![],
                    account: [f[0]; 20],
                },
            );
        }
    }
    let units = (0..real).map(unit).chain(fake).collect();
    (rig, units)
}

#[tokio::test(flavor = "current_thread")]
async fn a_notary_catches_a_slot_spent_in_two_swarms_that_never_meet() {
    use agentic_mailbox_swarm::select::notaries;
    use agentic_mailbox_swarm::stamp::ticket_id;
    let (mut rig, units) = wide().await;
    let real: Vec<usize> = (0..12).collect();
    let members: Vec<_> = units.iter().map(|u| Member { commitment: *u }).collect();
    let index = |u: &[u8; 32]| real.iter().copied().find(|h| unit(*h) == *u);
    let real_of = |chosen: Vec<Member>| -> Vec<usize> {
        chosen.iter().filter_map(|m| index(&m.commitment)).collect()
    };
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x6b; 32]).unwrap();
    let book = [0x6c; 32];
    test_book(&mut rig, &real, book, &key, 4);
    let ticket = ticket_id(&NETWORK_DOMAIN, &book, 0);
    let notary: Vec<usize> = real_of(notaries(&ticket, &members));
    assert!(!notary.is_empty());
    let swarm = |m: &[u8; 32]| real_of(rendezvous(m, &members, SWARM_SIZE));
    // Two mailboxes, and members of the second's swarm that are neither in
    // the first's nor notaries of the slot: they can learn of the first
    // spend only from the notaries. The witness gets the second spend
    // directly, the followers only as replicas.
    let m1 = (0..=255u8)
        .map(|k| [k; 32])
        .find(|m| swarm(m).len() >= 2)
        .unwrap();
    let apart = |m: &[u8; 32]| -> Vec<usize> {
        swarm(m)
            .into_iter()
            .filter(|h| !swarm(&m1).contains(h) && !notary.contains(h))
            .collect()
    };
    let m2 = (0..=255u8)
        .map(|k| [k; 32])
        .find(|m| *m != m1 && apart(m).len() >= 2)
        .unwrap();
    let (witness, followers) = (apart(&m2)[0], apart(&m2)[1..].to_vec());
    let time = rig.clock.wall();
    let first = paid(book, 0, &m1, b"first spend", &key);
    for holder in swarm(&m1) {
        rig.nodes[holder]
            .mailbox_holder
            .store(m1, period(WALL), b"first spend", &first, time)
            .unwrap();
    }
    // The first spend's holders put it on record with the slot's notaries.
    let on_record = |r: &Rig| {
        notary.iter().all(|h| {
            matches!(
                r.nodes[*h].mailbox_holder.notary_record(&ticket).unwrap(),
                Some(mailbox_holder::Notarized {
                    first: mailbox_holder::Statement::Ticket(ref s),
                    ..
                }) if *s == first
            )
        })
    };
    rig.run_until(STEPS, on_record).await;
    assert!(on_record(&rig), "{:?}", rig.trace);
    // The same slot spent again in the other mailbox, only at the witness.
    let second = paid(book, 0, &m2, b"second spend", &key);
    rig.nodes[witness]
        .mailbox_holder
        .store(m2, period(WALL), b"second spend", &second, rig.clock.wall())
        .unwrap();
    let proven = |r: &Rig, h: usize| {
        r.nodes[h]
            .mailbox_holder
            .equivocations()
            .unwrap()
            .iter()
            .any(|p| {
                BTreeSet::from([p.first.operation, p.second.operation])
                    == BTreeSet::from([first.operation, second.operation])
            })
    };
    let all_proven = |r: &Rig| {
        proven(r, witness)
            && followers
                .iter()
                .all(|f| held(r, *f, &m2) == 1 && proven(r, *f))
    };
    rig.run_until(STEPS, all_proven).await;
    assert!(all_proven(&rig), "{:?}", rig.trace);
    // None of them ever held the first spend; each keeps one proof, however
    // many notaries answered.
    for h in followers.iter().chain([&witness]) {
        assert_eq!(held(&rig, *h, &m1), 0, "holder {h}");
        let proofs = rig.nodes[*h].mailbox_holder.equivocations().unwrap();
        assert_eq!(proofs.len(), 1, "holder {h}");
        assert_eq!(proofs[0].verify(&NETWORK_DOMAIN, &key.account()), Ok(()));
    }
    // The notaries that answered hold the proof as well.
    assert!(notary.iter().any(|h| {
        !rig.nodes[*h]
            .mailbox_holder
            .equivocations()
            .unwrap()
            .is_empty()
    }));
}

// --- proofs and blocking ---------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn a_double_spend_proof_spreads_and_every_holder_refuses_the_book() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x6d; 32]).unwrap();
    let book = [0x6e; 32];
    test_book(&mut rig, &all, book, &key, 4);
    let time = rig.clock.wall();
    // Only holder 0 sees the slot spent twice.
    let (m1, m2) = ([0x21; 32], [0x22; 32]);
    let service = &mut rig.nodes[0].mailbox_holder;
    service
        .store(
            m1,
            period(WALL),
            b"one",
            &paid(book, 0, &m1, b"one", &key),
            time,
        )
        .unwrap();
    assert_eq!(
        service.store(
            m2,
            period(WALL),
            b"two",
            &paid(book, 0, &m2, b"two", &key),
            time
        ),
        Err(mailbox_holder::Refusal::Conflict)
    );
    let blocked = |r: &Rig| {
        all.iter().all(|h| {
            r.nodes[*h]
                .mailbox_holder
                .blocked_books()
                .unwrap()
                .contains(&book)
        })
    };
    rig.run_until(STEPS, blocked).await;
    assert!(blocked(&rig), "{:?}", rig.trace);
    // Every holder now refuses anything new the book pays for.
    let m3 = [0x23; 32];
    for holder in all {
        assert_eq!(
            rig.nodes[holder].mailbox_holder.store(
                m3,
                period(WALL),
                b"three",
                &paid(book, 1, &m3, b"three", &key),
                rig.clock.wall()
            ),
            Err(mailbox_holder::Refusal::Blocked),
            "holder {holder}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_holder_proven_to_equivocate_is_left_out() {
    use agentic_mailbox_swarm::proof::HolderEquivocation;
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        sync: true,
        ..Setup::default()
    })
    .await;
    // Holder 3 receipted two operations for one slot; holder 0 learns it.
    let lazy = 3;
    let receipt = |op: u8| {
        rig.nodes[lazy]
            .mailbox_holder
            .receipt_for_tests([0x24; 32], [op; 32], [0x25; 32], WALL)
    };
    let proof = HolderEquivocation {
        first: receipt(1),
        second: receipt(2),
    };
    rig.nodes[0]
        .mailbox_holder
        .accept_proofs(&mailbox_holder::Proofs {
            holders: vec![proof],
            ..Default::default()
        })
        .unwrap();
    let account = rig.nodes[lazy].mailbox_holder.account();
    let known = |r: &Rig| {
        [ALICE, BOB].iter().all(|n| {
            r.nodes[*n]
                .mailbox_holder
                .blocked_holders()
                .unwrap()
                .contains(&account)
        })
    };
    rig.run_until(STEPS, known).await;
    assert!(known(&rig), "{:?}", rig.trace);
    // Alice's message is stored without it, and Bob still reads it.
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "without it", "m", time)
        .unwrap()
        .id;
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent) && incoming(r, BOB, &c) == ["without it"]
    })
    .await;
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    let signers = receipted(&rig, ALICE, &sent, &to_bob);
    assert!(!signers.contains(&lazy), "{signers:?}");
    assert_eq!(incoming(&rig, BOB, &c), ["without it"]);
    // Alice never even asks it: nine receipts at most, however long.
    let receipts = |r: &Rig| r.nodes[ALICE].mailbox_client.stats().receipts;
    rig.run_until(STEPS, |r| receipts(r) >= 9).await;
    idle(&mut rig, 90).await;
    assert_eq!(receipts(&rig), 9);
}

// --- grant books ---------------------------------------------------------------

use crate::runtime::mailbox_replication::SYNC_INTERVAL;
use agentic_grant_book::{GrantBook, GrantEquivocation, GrantTerms, SecpKey};

fn identity_server() -> SecpKey {
    SecpKey::from_secret(&[0x31; 32]).unwrap()
}

/// The chain's `GrantIssuer` lets the identity server grant books of 100
/// stamps, ten a day.
fn accept_grants(rig: &mut Rig) {
    rig.chain.set_grants(test_support::FakeGrants {
        issuers: BTreeMap::from([(identity_server().account(), (0, u64::MAX))]),
        caps: BTreeMap::from([(0, 1_000)]),
        book_size: 100,
        max_validity_days: 30,
    });
}

fn grant(book: Account, day: u64, serial: u32) -> GrantBook {
    GrantBook::issue(
        GrantTerms {
            domain: NETWORK_DOMAIN,
            book,
            day,
            serial,
            count: 100,
            expiry: (day + 20) * PERIOD_SECONDS,
        },
        &identity_server(),
    )
}

/// Alice's node takes a grant to her book key, dated `day`.
fn grant_alice(rig: &mut Rig, day: u64, serial: u32) -> GrantBook {
    let book = rig.nodes[ALICE].core.mailbox_book_account().unwrap();
    let grant = grant(book, day, serial);
    rig.nodes[ALICE].add_mailbox_grant(&grant).unwrap();
    grant
}

/// Holders that pay stamps of the grant's book.
fn learned(rig: &Rig, grant: &GrantBook) -> usize {
    (0..HOLDERS)
        .filter(|h| {
            rig.nodes[*h].mailbox_holder.granted(&grant.id()).unwrap() == Some(grant.clone())
        })
        .count()
}

fn grant_checks(rig: &Rig, holder: usize) -> (u64, u64, u64) {
    let lane = &rig.nodes[holder].mailbox_client.grants;
    (lane.checked, lane.learned, lane.refused)
}

fn pending() -> mailbox_holder::Response {
    mailbox_holder::Response::Refused {
        code: "grant_pending".into(),
    }
}

/// Holders holding the grant as notaries, first seen on its day.
fn noted(rig: &Rig, grant: &GrantBook) -> usize {
    (0..HOLDERS)
        .filter(|h| {
            matches!(
                rig.nodes[*h].mailbox_holder.notary_record(&grant.id()).unwrap(),
                Some(mailbox_holder::Notarized {
                    first: mailbox_holder::Statement::Grant(g),
                    first_seen,
                }) if g == *grant && grant.first_seen_in_time(first_seen, 0)
            )
        })
        .count()
}

#[tokio::test(flavor = "current_thread")]
async fn a_grant_put_on_record_on_its_day_pays_on_later_days() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        sync: true,
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let grant = grant_alice(&mut rig, period(WALL), 0);
    // Nothing is sent that day: only Alice's node puts the grant on record.
    rig.run_until(STEPS, |r| noted(r, &grant) == HOLDERS).await;
    assert_eq!(noted(&rig, &grant), HOLDERS, "{:?}", rig.trace);
    for holder in 0..HOLDERS {
        assert_eq!(grant_checks(&rig, holder), (0, 0, 0), "holder {holder}");
    }
    // The next day, past the tolerance, the first message pays: every
    // holder learns the grant after asking its notaries, once.
    rig.clock
        .advance(Duration::from_secs(PERIOD_SECONDS + 7_200));
    let time = rig.clock.wall();
    assert_eq!(period(time), period(WALL) + 1);
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "on the grant", "g1", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent.id)
            && incoming(r, BOB, &c) == ["on the grant"]
            && learned(r, &grant) == HOLDERS
    })
    .await;
    assert!(stored(&rig, ALICE, &sent.id), "{:?}", rig.trace);
    assert_eq!(incoming(&rig, BOB, &c), ["on the grant"]);
    assert_eq!(learned(&rig, &grant), HOLDERS);
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    receipted(&rig, ALICE, &sent.id, &to_bob);
    for holder in 0..HOLDERS {
        assert_eq!(grant_checks(&rig, holder), (1, 1, 0), "holder {holder}");
    }
    let books = rig.nodes[ALICE].core.mailbox_books().unwrap();
    assert_eq!(
        (books.len(), books[0].book, books[0].used),
        (1, grant.id(), 1)
    );
    // The next message is stored without asking the notaries again.
    let next = rig.nodes[ALICE]
        .core
        .send_message(&c, "more on the grant", "g2", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &next.id) && (0..HOLDERS).all(|h| held(r, h, &to_bob) == 2)
    })
    .await;
    assert!(stored(&rig, ALICE, &next.id), "{:?}", rig.trace);
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &to_bob), 2, "holder {holder}");
        assert_eq!(grant_checks(&rig, holder), (1, 1, 0), "holder {holder}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn unreachable_notaries_leave_a_grant_open_until_they_answer() {
    let offline = vec![0, 1, 2, 3, 4];
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        alice_granted: true,
        offline: offline.clone(),
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let grant = grant_alice(&mut rig, period(WALL), 0);
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "when they return", "back", time)
        .unwrap();
    // Five of ten notaries answer: not most of them, but nobody said late.
    // Holders keep asking; nothing is refused.
    let rechecked = |r: &Rig| (5..HOLDERS).all(|h| grant_checks(r, h).0 >= 2);
    rig.run_until(STEPS, rechecked).await;
    assert!(rechecked(&rig), "{:?}", rig.trace);
    assert!(!stored(&rig, ALICE, &sent.id));
    assert_eq!(learned(&rig, &grant), 0);
    for holder in 5..HOLDERS {
        assert_eq!(grant_checks(&rig, holder).2, 0, "holder {holder}");
    }
    // The rest come back the same day: the grant is learned and paid.
    for holder in &offline {
        listen(&mut rig, *holder).await;
    }
    for holder in &offline {
        let entry = directory_entry(&rig, *holder);
        for node in (0..HOLDERS).chain([ALICE, BOB]) {
            rig.nodes[node]
                .mailbox_client
                .learn_holder(unit(*holder), entry.clone());
        }
        for other in 0..HOLDERS {
            let entry = directory_entry(&rig, other);
            rig.nodes[*holder]
                .mailbox_client
                .learn_holder(unit(other), entry);
        }
        rig.connect(ALICE, *holder).await;
    }
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent.id) && learned(r, &grant) >= QUORUM
    })
    .await;
    assert!(stored(&rig, ALICE, &sent.id), "{:?}", rig.trace);
    for holder in 0..HOLDERS {
        assert_eq!(grant_checks(&rig, holder).2, 0, "holder {holder}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_grant_first_shown_after_its_day_never_pays() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    // Two days old and never on record: a stolen server key spending an
    // unused serial of a past day.
    let grant = grant_alice(&mut rig, period(WALL) - 2, 0);
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "backdated", "late", time)
        .unwrap();
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    let refused = |r: &Rig| (0..HOLDERS).filter(|h| grant_checks(r, *h).2 == 1).count();
    rig.run_until(STEPS, |r| refused(r) == HOLDERS).await;
    assert_eq!(refused(&rig), HOLDERS, "{:?}", rig.trace);
    // A holder answers from its memory of the refusal.
    let refusal = mailbox_holder::Response::Refused {
        code: "grant".into(),
    };
    assert_eq!(rig.nodes[0].offer_grant(grant.clone()), refusal);
    idle(&mut rig, 120).await;
    assert!(!stored(&rig, ALICE, &sent.id));
    assert_eq!(learned(&rig, &grant), 0);
    assert!(incoming(&rig, BOB, &c).is_empty());
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &to_bob), 0, "holder {holder}");
        // Checked once; the refusal is remembered, not asked again.
        assert_eq!(grant_checks(&rig, holder), (1, 0, 1), "holder {holder}");
    }
    // Alice gives up on every holder: a refused grant is final.
    let info = rig.nodes[ALICE].mailbox_client.info();
    assert_eq!(info["sends"][0]["refused"], HOLDERS, "{info}");
    assert_eq!(info["sends"][0]["inFlight"], 0, "{info}");
    // A grant learned since is acknowledged despite the remembered refusal.
    let now = rig.clock.wall();
    rig.nodes[0]
        .mailbox_holder
        .learn_grant(&grant, grant.day * PERIOD_SECONDS + 5, now)
        .unwrap();
    assert_eq!(
        rig.nodes[0].offer_grant(grant.clone()),
        mailbox_holder::Response::Learned
    );
    // The memory lasts five minutes; then the notaries are asked again.
    idle(&mut rig, 300).await;
    assert_eq!(rig.nodes[1].offer_grant(grant.clone()), pending());
    rig.run_until(STEPS, |r| grant_checks(r, 1).2 == 2).await;
    assert_eq!(grant_checks(&rig, 1), (2, 0, 2), "{:?}", rig.trace);
}

#[tokio::test(flavor = "current_thread")]
async fn a_serial_granted_twice_pays_only_its_first_book_and_proves_the_issuer() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let today = period(WALL);
    // The serial went to another book first, on its day, with every notary.
    let first = grant([0x5e; 20], today, 0);
    let time = rig.clock.wall();
    for holder in 0..HOLDERS {
        rig.nodes[holder]
            .mailbox_holder
            .notarize(&mailbox_holder::Statement::Grant(first.clone()), time)
            .unwrap();
    }
    let second = grant_alice(&mut rig, today, 0);
    assert_eq!(second.id(), first.id());
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "duplicate number", "dup", time)
        .unwrap();
    let refused = |r: &Rig| (0..HOLDERS).filter(|h| grant_checks(r, *h).2 == 1).count();
    rig.run_until(STEPS, |r| refused(r) == HOLDERS).await;
    assert_eq!(refused(&rig), HOLDERS, "{:?}", rig.trace);
    assert!(!stored(&rig, ALICE, &sent.id));
    assert_eq!(learned(&rig, &second), 0);
    // Each holder keeps the pair as proof against the issuer, which grants
    // nothing here from tomorrow on.
    let proof = GrantEquivocation {
        first: first.clone(),
        second: second.clone(),
    };
    let tomorrow = grant([0x5f; 20], today + 1, 1);
    for holder in 0..HOLDERS {
        let service = &rig.nodes[holder].mailbox_holder;
        assert_eq!(service.grant_equivocations().unwrap(), vec![proof.clone()]);
        assert_eq!(
            service.check_grant(&tomorrow, (today + 1) * PERIOD_SECONDS + 10),
            Err(mailbox_holder::Refusal::Blocked)
        );
    }
    // The serial's first book still pays: its notaries saw it first, on
    // its day.
    assert_eq!(rig.nodes[0].offer_grant(first.clone()), pending());
    rig.run_until(STEPS, |r| {
        r.nodes[0].mailbox_holder.granted(&first.id()).unwrap() == Some(first.clone())
    })
    .await;
    assert_eq!(grant_checks(&rig, 0), (2, 1, 1), "{:?}", rig.trace);
    assert_eq!(
        rig.nodes[0].offer_grant(first.clone()),
        mailbox_holder::Response::Learned
    );
}

#[tokio::test(flavor = "current_thread")]
async fn holders_never_shown_a_grant_learn_it_from_the_swarm() {
    let hidden = vec![1, 4, 7];
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        sync: true,
        alice_granted: true,
        hidden_from_alice: hidden.clone(),
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let grant = grant_alice(&mut rig, period(WALL), 0);
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "to all on the grant", "m", time)
        .unwrap();
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent.id)).await;
    let reached: BTreeSet<_> = (0..HOLDERS).filter(|h| !hidden.contains(h)).collect();
    assert_eq!(receipted(&rig, ALICE, &sent.id, &to_bob), reached);
    // Pulling the entry brings its grant; each hidden holder checks it
    // with the notaries once and then takes the copy.
    let all: Vec<usize> = (0..HOLDERS).collect();
    let started = rig.clock.instant();
    rig.run_until(STEPS, |r| same_copies(r, &all, &to_bob, 1))
        .await;
    assert!(same_copies(&rig, &all, &to_bob, 1), "{:?}", rig.trace);
    assert!(rig.clock.instant() - started <= 4 * SYNC_INTERVAL);
    assert_eq!(learned(&rig, &grant), HOLDERS);
    for holder in &hidden {
        assert_eq!(grant_checks(&rig, *holder), (1, 1, 0), "holder {holder}");
        assert_eq!(replication(&rig, *holder).pulled, 1, "holder {holder}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn entries_of_a_refused_grant_never_hold_back_the_rest_of_a_mailbox() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let now = rig.clock.wall();
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x65; 32]).unwrap();
    // Holder 0 took a grant its notaries would call late; nobody else will.
    let late = grant(key.account(), period(WALL) - 2, 0);
    rig.nodes[0].mailbox_holder.learn_grant_day(
        identity_server().account(),
        late.day,
        crate::runtime::chain::GrantDay {
            active: true,
            cap_coins: 1_000,
            book_size: 100,
            max_validity_days: 30,
            today: period(WALL),
        },
    );
    rig.nodes[0]
        .mailbox_holder
        .learn_grant(&late, late.day * PERIOD_SECONDS + 5, now)
        .unwrap();
    let all: Vec<usize> = (0..HOLDERS).collect();
    let book = [0x66; 32];
    test_book(&mut rig, &all, book, &key, 200);
    let mailbox = [0x5a; 32];
    let granted = paid(late.id(), 0, &mailbox, b"granted", &key);
    let bought = paid(book, 0, &mailbox, b"bought", &key);
    for (envelope, stamp) in [(&b"granted"[..], &granted), (&b"bought"[..], &bought)] {
        rig.nodes[0]
            .mailbox_holder
            .store(mailbox, period(WALL), envelope, stamp, now)
            .unwrap();
    }
    // A page naming grants of none of its entries starts no check.
    rig.nodes[3].replication_page(
        mailbox,
        unit(0),
        Ok(mailbox_holder::Response::Page {
            entries: vec![],
            next: 0,
            grants: vec![grant(key.account(), period(WALL), 1)],
        }),
    );
    assert_eq!(grant_checks(&rig, 3), (0, 0, 0));
    // Everyone else refuses the grant once and still takes what follows it.
    let only_bought = |r: &Rig| {
        (1..HOLDERS).all(|h| {
            entries(r, h, &mailbox)
                .iter()
                .map(|e| e.stamp.clone())
                .collect::<Vec<_>>()
                == [bought.clone()]
        })
    };
    rig.run_until(STEPS, only_bought).await;
    assert!(only_bought(&rig), "{:?}", rig.trace);
    for holder in 1..HOLDERS {
        assert_eq!(grant_checks(&rig, holder), (1, 0, 1), "holder {holder}");
    }
    // Past the refusal memory, the refused entry is not read again.
    idle(&mut rig, 400).await;
    assert!(only_bought(&rig));
    for holder in 1..HOLDERS {
        assert_eq!(grant_checks(&rig, holder), (1, 0, 1), "holder {holder}");
    }
}

// --- notary load ------------------------------------------------------------

/// Notary requests a node has in flight.
fn notarizing(rig: &Rig, node: usize) -> usize {
    rig.nodes[node]
        .mailbox_client
        .requests
        .values()
        .filter(|p| matches!(p, crate::runtime::mailbox_client::Purpose::Notarize { .. }))
        .count()
}

#[tokio::test(flavor = "current_thread")]
async fn a_burst_goes_on_record_everywhere_without_crowding_out_its_stores() {
    use agentic_mailbox_swarm::select::notaries;
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        sync: true,
        ..Setup::default()
    })
    .await;
    const BURST: usize = 32;
    let time = rig.clock.wall();
    let ids: Vec<String> = (0..BURST)
        .map(|n| {
            rig.nodes[ALICE]
                .core
                .send_message(&c, &format!("batch {n}"), &format!("b{n}"), time)
                .unwrap()
                .id
        })
        .collect();
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    let members: Vec<_> = (0..HOLDERS)
        .map(|h| Member {
            commitment: unit(h),
        })
        .collect();
    // Every slot spent must end up on record with every one of its
    // notaries: batching may not trade coverage for fewer requests.
    let tickets = std::cell::RefCell::new(Vec::new());
    let covered = |r: &Rig| {
        let mut tickets = tickets.borrow_mut();
        if tickets.len() < BURST {
            *tickets = ids
                .iter()
                .filter_map(|id| r.nodes[ALICE].core.swarm_receipts(id).unwrap())
                .map(|receipts| receipts[0].ticket)
                .collect();
        }
        tickets.len() == BURST
            && tickets.iter().all(|ticket| {
                notaries(ticket, &members).iter().all(|m| {
                    let h = (0..HOLDERS).find(|h| unit(*h) == m.commitment).unwrap();
                    r.nodes[h]
                        .mailbox_holder
                        .notary_record(ticket)
                        .unwrap()
                        .is_some()
                })
            })
    };
    let peak = std::cell::Cell::new(0usize);
    rig.run_until(STEPS, |r| {
        let most = (0..r.nodes.len())
            .map(|n| notarizing(r, n))
            .max()
            .unwrap_or(0);
        peak.set(peak.get().max(most));
        (0..HOLDERS).all(|h| held(r, h, &to_bob) == BURST) && covered(r)
    })
    .await;
    assert!(covered(&rig), "{:?}", rig.trace);
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &to_bob), BURST, "holder {holder}");
    }
    // The notary lane stays in the background: at most two requests in
    // flight per node, each carrying whatever piled up meanwhile.
    assert!(peak.get() <= 2, "peak {}", peak.get());
    // So stores are never turned away: one request per holder and message.
    let sender = rig.nodes[ALICE].mailbox_client.info();
    assert_eq!(sender["failures"], 0, "{sender}");
    assert_eq!(
        sender["sent"]["store"],
        (HOLDERS * BURST) as u64,
        "{sender}"
    );
}

// --- acceptance spike (phase 7) -------------------------------------------------

/// Sum of one request kind over nodes, from `info()["sent"|"served"]`.
fn requests(rig: &Rig, nodes: &[usize], side: &str, kind: &str) -> u64 {
    nodes
        .iter()
        .map(|n| {
            rig.nodes[*n].mailbox_client.info()[side][kind]
                .as_u64()
                .unwrap_or(0)
        })
        .sum()
}

/// The acceptance scenario of the redesign, on the managed-time rig: 258
/// messages while one holder receipts as an unlisted unit, two holders are
/// out of the sender's reach and another book spends one slot twice; the
/// sender leaves; nine of ten holders lose their disks; the recipient comes
/// online and reads everything. Prints the numbers of the evidence in
/// `evidence/reviews/mailbox-swarm-spike-2026-09-26/`. Minutes of real time:
/// `cargo test -p agentic-node --lib -- --ignored acceptance_spike --nocapture`
#[tokio::test(flavor = "current_thread")]
#[ignore = "acceptance spike: run explicitly"]
async fn acceptance_spike_258_messages() {
    use agentic_mailbox_swarm::select::notaries;
    // A smaller count measures how the run scales.
    let messages: usize = std::env::var("AIN_SPIKE_MESSAGES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(258);
    const SPIKE_STEPS: usize = 1_000_000;
    eprintln!("SPIKE-PHASE start messages={messages}");
    let checks = std::cell::Cell::new(0u64);
    let progress = |r: &Rig, phase: &str, done: usize| {
        checks.set(checks.get() + 1);
        if checks.get().is_multiple_of(2_000) {
            eprintln!(
                "SPIKE-PROGRESS {phase} done={done} steps={} trace={}",
                checks.get(),
                r.trace.len()
            );
        }
    };
    let malicious = 3;
    let hidden = vec![1, 6];
    let Chat {
        mut rig,
        conversation: c,
        terms,
        ..
    } = chat(Setup {
        sync: true,
        bob_offline: true,
        wrong_unit: vec![malicious],
        hidden_from_alice: hidden.clone(),
        ..Setup::default()
    })
    .await;
    let start = rig.clock.instant();
    let time = rig.clock.wall();
    let all: Vec<usize> = (0..HOLDERS).collect();
    let honest: Vec<usize> = all.iter().copied().filter(|h| *h != malicious).collect();
    let reached: BTreeSet<usize> = honest
        .iter()
        .copied()
        .filter(|h| !hidden.contains(h))
        .collect();
    assert_eq!(reached.len(), QUORUM);
    let words: Vec<String> = (0..messages).map(|n| format!("message {n}")).collect();
    let real = std::time::Instant::now();
    let ids: Vec<String> = words
        .iter()
        .enumerate()
        .map(|(n, w)| {
            rig.nodes[ALICE]
                .core
                .send_message(&c, w, &format!("s{n}"), time)
                .unwrap()
                .id
        })
        .collect();
    let to_bob = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&c, false, time)
        .unwrap();
    // Another book spends one slot twice, at holders 0 and 5. With ten
    // holders every one belongs to both mailboxes' swarms, so replication
    // and the notaries both see it; detection by the notaries alone is
    // `a_notary_catches_a_slot_spent_in_two_swarms_that_never_meet`.
    let cheat = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x6d; 32]).unwrap();
    let cheat_book = [0x6e; 32];
    test_book(&mut rig, &all, cheat_book, &cheat, 4);
    let (m1, m2) = ([0x21; 32], [0x22; 32]);
    rig.nodes[0]
        .mailbox_holder
        .store(
            m1,
            period(WALL),
            b"one",
            &paid(cheat_book, 0, &m1, b"one", &cheat),
            time,
        )
        .unwrap();
    rig.nodes[5]
        .mailbox_holder
        .store(
            m2,
            period(WALL),
            b"two",
            &paid(cheat_book, 0, &m2, b"two", &cheat),
            time,
        )
        .unwrap();
    let blocked = |r: &Rig| {
        all.iter().all(|h| {
            r.nodes[*h]
                .mailbox_holder
                .blocked_books()
                .unwrap()
                .contains(&cheat_book)
        })
    };
    let blocked_at = std::cell::Cell::new(None);
    let note_blocked = |r: &Rig| {
        if blocked_at.get().is_none() && blocked(r) {
            blocked_at.set(Some((r.clock.instant(), real.elapsed())));
        }
    };

    // Conditions read counters: they run after every step.
    let count = |r: &Rig, holder: usize| {
        r.nodes[holder]
            .mailbox_holder
            .summary(&to_bob)
            .map_or(0, |s| s.count as usize)
    };
    rig.run_until(SPIKE_STEPS, |r| {
        note_blocked(r);
        let done = r.nodes[ALICE].mailbox_client.stats().stored as usize;
        progress(r, "stored", done);
        done == messages
    })
    .await;
    let stored_in = rig.clock.instant() - start;
    let stored_real = real.elapsed();
    eprintln!("SPIKE-PHASE stored virtual={stored_in:?} real={stored_real:?}");
    // Stored at exactly the seven holders the sender reaches that receipt
    // as their listed units; the other one's receipts came and were refused.
    for id in &ids {
        assert_eq!(receipted(&rig, ALICE, id, &to_bob), reached);
    }
    let sender = rig.nodes[ALICE].mailbox_client.info();
    assert_eq!(sender["rejectedReceipts"], messages as u64, "{sender}");
    assert_eq!(
        rig.nodes[ALICE].core.mailbox_books().unwrap()[0].used as usize,
        messages
    );
    // No store was turned away: the sender's only failures are dials to the
    // two holders it cannot reach, and every holder it reaches served one
    // store per message.
    let kinds = sender["failureKinds"].as_object().unwrap();
    assert!(kinds.keys().all(|k| k == "dial"), "{sender}");
    for holder in reached.iter().chain([&malicious]) {
        assert_eq!(
            requests(&rig, &[*holder], "served", "store"),
            messages as u64,
            "holder {holder}"
        );
    }

    rig.run_until(SPIKE_STEPS, |r| {
        note_blocked(r);
        blocked_at.get().is_some()
    })
    .await;
    let (blocked_instant, blocked_real) = blocked_at.get().unwrap();
    let blocked_in = blocked_instant - start;
    eprintln!("SPIKE-PHASE blocked_in virtual={blocked_in:?} real={blocked_real:?}");

    // Every slot goes on record with all of its notaries (a sample).
    let members: Vec<_> = all
        .iter()
        .map(|h| Member {
            commitment: unit(*h),
        })
        .collect();
    let sample: Vec<[u8; 32]> = [0, messages / 2, messages - 1]
        .iter()
        .map(|n| {
            rig.nodes[ALICE]
                .core
                .swarm_receipts(&ids[*n])
                .unwrap()
                .unwrap()[0]
                .ticket
        })
        .collect();
    let on_record = |r: &Rig| {
        sample.iter().all(|ticket| {
            notaries(ticket, &members).iter().all(|m| {
                let h = all
                    .iter()
                    .copied()
                    .find(|h| unit(*h) == m.commitment)
                    .unwrap();
                r.nodes[h]
                    .mailbox_holder
                    .notary_record(ticket)
                    .unwrap()
                    .is_some()
            })
        })
    };
    rig.run_until(SPIKE_STEPS, on_record).await;
    assert!(on_record(&rig));
    let recorded_in = rig.clock.instant() - start;
    eprintln!("SPIKE-PHASE recorded_in virtual={recorded_in:?}");
    let all_nodes: Vec<usize> = (0..rig.nodes.len()).collect();
    let sent_before_leaving: serde_json::Map<String, Value> =
        ["store", "notarize", "read", "summaries", "pull", "proofs"]
            .iter()
            .map(|kind| {
                (
                    kind.to_string(),
                    json!(requests(&rig, &all_nodes, "sent", kind) as f64 / messages as f64),
                )
            })
            .collect();
    let statements = requests(&rig, &all, "served", "notarizeStatements");
    let batches = requests(&rig, &all, "served", "notarize");

    // The sender leaves for good; the two holders it never reached get
    // every copy from the swarm. The one receipting as an unlisted unit is
    // no member of it.
    let left_at = rig.clock.instant();
    drop(rig.nodes.pop());
    rig.run_until(SPIKE_STEPS, |r| {
        let done = honest.iter().map(|h| count(r, *h)).min().unwrap_or(0);
        progress(r, "copied", done);
        done == messages
    })
    .await;
    assert!(same_copies(&rig, &honest, &to_bob, messages));
    // Pulled, as the sender never reached them (the count also takes the
    // double-spend entries).
    for holder in &hidden {
        assert!(
            replication(&rig, *holder).pulled >= messages as u64,
            "holder {holder}"
        );
    }
    let copied_in = rig.clock.instant() - left_at;
    eprintln!("SPIKE-PHASE copied_in virtual={copied_in:?}");

    // Nine holders lose their disks; the malicious one stays malicious.
    let survivor = Some(last_holder(&to_bob))
        .filter(|h| *h != malicious)
        .unwrap_or(honest[0]);
    let lost_at = rig.clock.instant();
    for holder in (0..HOLDERS).filter(|h| *h != survivor) {
        wipe(&mut rig, holder, &terms);
    }
    rig.nodes[malicious].mailbox_holder.set_unit([0x77; 32]);
    rig.run_until(SPIKE_STEPS, |r| {
        let done = honest.iter().map(|h| count(r, *h)).min().unwrap_or(0);
        progress(r, "repaired", done);
        done == messages
    })
    .await;
    assert!(same_copies(&rig, &honest, &to_bob, messages));
    let repaired_in = rig.clock.instant() - lost_at;
    eprintln!("SPIKE-PHASE repaired_in virtual={repaired_in:?}");

    // The recipient comes online and reads everything, in order.
    let online_at = rig.clock.instant();
    let reading_real = std::time::Instant::now();
    for holder in 0..HOLDERS {
        rig.connect(BOB, holder).await;
    }
    rig.run_until(SPIKE_STEPS, |r| {
        let done = r.nodes[BOB].mailbox_client.stats().received as usize;
        progress(r, "read", done);
        done == messages
    })
    .await;
    assert_eq!(incoming(&rig, BOB, &c), words);
    let read_in = rig.clock.instant() - online_at;
    eprintln!("SPIKE-PHASE read_in virtual={read_in:?}");
    let reader = rig.nodes[BOB].mailbox_client.info();

    let per = |n: u64| n as f64 / messages as f64;
    eprintln!(
        "SPIKE {}",
        json!({
            "messages": messages,
            "virtualSeconds": {
                "storedAtQuorum": stored_in.as_secs_f64(),
                "doubleSpendBlockedEverywhere": blocked_in.as_secs_f64(),
                "sampleOnRecordWithAllNotaries": recorded_in.as_secs_f64(),
                "copiedToUnreachedAfterSenderLeft": copied_in.as_secs_f64(),
                "repairedAfterLoss": repaired_in.as_secs_f64(),
                "readAfterOnline": read_in.as_secs_f64(),
            },
            "realSeconds": {
                "stored": stored_real.as_secs_f64(),
                "doubleSpendBlockedEverywhere": blocked_real.as_secs_f64(),
                "read": reading_real.elapsed().as_secs_f64(),
                "total": real.elapsed().as_secs_f64(),
            },
            "requestsPerMessageAllNodesUntilSenderLeft": sent_before_leaving,
            "notaryStatementsPerRequest": statements as f64 / batches.max(1) as f64,
            "senderFailureKinds": sender["failureKinds"],
            "readerRequestsPerMessage": reader["sent"].as_object().unwrap().iter()
                .map(|(k, v)| (k.clone(), json!(per(v.as_u64().unwrap()))))
                .collect::<serde_json::Map<_, _>>(),
            "steps": rig.trace.len(),
        })
    );
}

// --- books from the chain ----------------------------------------------------------

/// Alice sends `count` messages to Bob now.
fn send_alice(rig: &mut Rig, c: &str, words: &str, count: usize) -> Vec<agentic_core::Message> {
    let time = rig.clock.wall();
    (0..count)
        .map(|n| {
            rig.nodes[ALICE]
                .core
                .send_message(c, &format!("{words} {n}"), &format!("{words}-{n}"), time)
                .unwrap()
        })
        .collect()
}

fn to_bob(rig: &Rig, c: &str) -> [u8; 32] {
    rig.nodes[ALICE]
        .core
        .swarm_mailbox(c, false, rig.clock.wall())
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn holders_read_a_bought_book_from_the_chain_once() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    let sent = send_alice(&mut rig, &c, "bought", 3);
    rig.run_until(STEPS, |r| {
        sent.iter().all(|m| stored(r, ALICE, &m.id)) && incoming(r, BOB, &c).len() == 3
    })
    .await;
    assert!(
        sent.iter().all(|m| stored(&rig, ALICE, &m.id)),
        "{:?}",
        rig.trace
    );
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["bought 0", "bought 1", "bought 2"]
    );
    let mailbox = to_bob(&rig, &c);
    for m in &sent {
        receipted(&rig, ALICE, &m.id, &mailbox);
    }
    // One read per holder, however many stamps and notary statements named
    // the book.
    for holder in 0..HOLDERS {
        assert_eq!(
            rig.chain.book_reads(holder, &alice_book()),
            1,
            "holder {holder}"
        );
    }
}

/// While the chain is down, and then while the purchase is below the
/// confirmations, nothing is stored; holders read the chain about once a
/// minute, not on every retry. Once confirmed the message is stored.
#[tokio::test(flavor = "current_thread")]
async fn a_book_pays_only_once_the_chain_confirms_its_purchase() {
    let Chat {
        mut rig,
        conversation: c,
        terms,
        ..
    } = chat(Setup {
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    let book = alice_book();
    let (_, bought) = terms.iter().find(|(b, _)| *b == book).unwrap();
    rig.chain.buy_unconfirmed(book, record(bought));
    rig.chain.set_down(true);
    let sent = send_alice(&mut rig, &c, "after confirmation", 1);
    idle(&mut rig, 90).await;
    rig.chain.set_down(false);
    idle(&mut rig, 90).await;
    assert!(!stored(&rig, ALICE, &sent[0].id));
    let mailbox = to_bob(&rig, &c);
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &mailbox), 0, "holder {holder}");
        let reads = rig.chain.book_reads(holder, &book);
        assert!((1..=4).contains(&reads), "holder {holder}: {reads} reads");
    }
    rig.chain.confirm(&book);
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent[0].id) && incoming(r, BOB, &c).len() == 1
    })
    .await;
    assert!(stored(&rig, ALICE, &sent[0].id), "{:?}", rig.trace);
    assert_eq!(incoming(&rig, BOB, &c), ["after confirmation 0"]);
}

/// A sender stamping with someone else's book id: the chain recorded
/// another key for it, so the stamps pay for nothing, and the book is not
/// read again for every stamp.
#[tokio::test(flavor = "current_thread")]
async fn a_stamp_on_someone_elses_book_pays_for_nothing() {
    let Chat {
        mut rig,
        conversation: c,
        terms,
        ..
    } = chat(Setup {
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    let book = alice_book();
    let (_, bought) = terms.iter().find(|(b, _)| *b == book).unwrap();
    rig.chain.buy(
        book,
        crate::runtime::chain::BookRecord {
            key: [0x99; 20],
            ..record(bought)
        },
    );
    let sent = send_alice(&mut rig, &c, "someone else's book", 2);
    idle(&mut rig, 60).await;
    let mailbox = to_bob(&rig, &c);
    assert!(sent.iter().all(|m| !stored(&rig, ALICE, &m.id)));
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &mailbox), 0, "holder {holder}");
        assert_eq!(rig.chain.book_reads(holder, &book), 1, "holder {holder}");
    }
}

/// A holder still reading the book answers stores at once, so the quorum
/// forms without it; its replication waits for the read instead of
/// skipping the entries, and it gets every copy afterwards from the swarm.
#[tokio::test(flavor = "current_thread")]
async fn a_holder_reading_a_book_neither_stalls_a_store_nor_skips_its_copies() {
    let slow = 4;
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    rig.chain.hold(slow);
    let sent = send_alice(&mut rig, &c, "copies", 3);
    let mailbox = to_bob(&rig, &c);
    let others = |r: &Rig| {
        (0..HOLDERS)
            .filter(|h| *h != slow)
            .all(|h| held(r, h, &mailbox) == 3)
    };
    rig.run_until(STEPS, |r| {
        sent.iter().all(|m| stored(r, ALICE, &m.id)) && others(r)
    })
    .await;
    assert!(others(&rig), "{:?}", rig.trace);
    for m in &sent {
        assert!(!receipted(&rig, ALICE, &m.id, &mailbox).contains(&slow));
    }
    // Replication rounds while its read is pending: it takes nothing,
    // skips nothing and reads once.
    idle(&mut rig, 3 * SYNC_INTERVAL.as_secs()).await;
    assert_eq!(held(&rig, slow, &mailbox), 0);
    assert_eq!(rig.chain.book_reads(slow, &alice_book()), 1);
    rig.chain.release(slow);
    rig.run_until(STEPS, |r| held(r, slow, &mailbox) == 3).await;
    assert_eq!(held(&rig, slow, &mailbox), 3, "{:?}", rig.trace);
}

/// A holder whose RPC does not show the purchase yet keeps its replication
/// cursor before the book's entries and takes them once its RPC catches up.
#[tokio::test(flavor = "current_thread")]
async fn a_holder_whose_chain_lags_still_takes_every_copy() {
    let behind = 6;
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        books_on_chain: true,
        hidden_from_alice: vec![behind],
        ..Setup::default()
    })
    .await;
    rig.chain.lag(behind);
    let sent = send_alice(&mut rig, &c, "lagging behind", 3);
    let mailbox = to_bob(&rig, &c);
    rig.run_until(STEPS, |r| {
        sent.iter().all(|m| stored(r, ALICE, &m.id))
            && (0..HOLDERS)
                .filter(|h| *h != behind)
                .all(|h| held(r, h, &mailbox) == 3)
    })
    .await;
    idle(&mut rig, 3 * SYNC_INTERVAL.as_secs()).await;
    assert_eq!(held(&rig, behind, &mailbox), 0);
    rig.chain.catch_up(behind);
    rig.run_until(STEPS, |r| held(r, behind, &mailbox) == 3)
        .await;
    assert_eq!(held(&rig, behind, &mailbox), 3, "{:?}", rig.trace);
}

/// Entries of a book the chain never sold (a bad holder's) hold a
/// replication cursor back only while a lagging RPC could explain them:
/// after ten minutes they are passed over and the rest is taken.
#[tokio::test(flavor = "current_thread")]
async fn entries_of_a_book_nobody_bought_hold_back_a_mailbox_only_for_a_while() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        alice_without_directory: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let now = rig.clock.wall();
    let started = rig.clock.instant();
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x65; 32]).unwrap();
    // Holder 0 took stamps of a book nobody bought; everyone knows the other.
    let unbought = [0x67; 32];
    test_book(&mut rig, &[0], unbought, &key, 200);
    let all: Vec<usize> = (0..HOLDERS).collect();
    let book = [0x66; 32];
    test_book(&mut rig, &all, book, &key, 200);
    let mailbox = [0x5a; 32];
    let junk = paid(unbought, 0, &mailbox, b"junk", &key);
    let bought = paid(book, 0, &mailbox, b"bought", &key);
    for (envelope, stamp) in [(&b"junk"[..], &junk), (&b"bought"[..], &bought)] {
        rig.nodes[0]
            .mailbox_holder
            .store(mailbox, period(WALL), envelope, stamp, now)
            .unwrap();
    }
    let only_bought = |r: &Rig| {
        (1..HOLDERS).all(|h| {
            entries(r, h, &mailbox)
                .iter()
                .map(|e| e.stamp.clone())
                .collect::<Vec<_>>()
                == [bought.clone()]
        })
    };
    // Minutes a lagging RPC could take: nothing past the unread entry yet.
    idle(&mut rig, 5 * 60).await;
    assert!((1..HOLDERS).all(|h| entries(&rig, h, &mailbox).is_empty()));
    rig.run_until(STEPS, only_bought).await;
    assert!(only_bought(&rig), "{:?}", rig.trace);
    assert!(rig.clock.instant() - started <= Duration::from_secs(12 * 60));
    let reads: Vec<usize> = (1..HOLDERS)
        .map(|h| rig.chain.book_reads(h, &unbought))
        .collect();
    assert!(reads.iter().all(|r| (1..=12).contains(r)), "{reads:?}");
    // Passed over for good: not read again.
    idle(&mut rig, 120).await;
    assert!(only_bought(&rig));
    let again: Vec<usize> = (1..HOLDERS)
        .map(|h| rig.chain.book_reads(h, &unbought))
        .collect();
    assert_eq!(again, reads);
}

#[tokio::test(flavor = "current_thread")]
async fn a_grant_pays_within_the_cap_of_its_own_day() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Holders ask a grant's notaries, so they know the directory.
        sync: true,
        alice_granted: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let today = period(WALL);
    let server = identity_server().account();
    // The cap doubled today: yesterday a thirteenth book of 100 was over
    // it, today it is not.
    rig.chain.set_grants(test_support::FakeGrants {
        issuers: BTreeMap::from([(server, (0, u64::MAX))]),
        caps: BTreeMap::from([(0, 1_000), (today, 2_000)]),
        book_size: 100,
        max_validity_days: 30,
    });
    grant_alice(&mut rig, today, 12);
    let bob_key = rig.nodes[BOB].core.mailbox_book_account().unwrap();
    rig.nodes[BOB]
        .add_mailbox_grant(&grant(bob_key, today - 1, 12))
        .unwrap();
    let time = rig.clock.wall();
    let from_alice = rig.nodes[ALICE]
        .core
        .send_message(&c, "within the limit", "cap-a", time)
        .unwrap();
    let from_bob = rig.nodes[BOB]
        .core
        .send_message(&c, "over the limit", "cap-b", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &from_alice.id) && incoming(r, BOB, &c) == ["within the limit"]
    })
    .await;
    assert!(stored(&rig, ALICE, &from_alice.id), "{:?}", rig.trace);
    idle(&mut rig, 60).await;
    assert!(!stored(&rig, BOB, &from_bob.id));
    let to_alice = rig.nodes[BOB].core.swarm_mailbox(&c, false, time).unwrap();
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &to_alice), 0, "holder {holder}");
        // A past day's rules cannot change: read once.
        assert_eq!(
            rig.chain.grant_reads(holder, &server, today - 1),
            1,
            "holder {holder}"
        );
    }
}

/// The owner ends a stolen issuer key from today. Holders keep today's
/// rules at most ten minutes, then read them again, so a grant of today
/// shown after that is refused. A grant already learned still pays.
#[tokio::test(flavor = "current_thread")]
async fn an_issuer_ended_today_grants_nothing_new_once_its_rules_are_read_again() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Holders ask a grant's notaries, so they know the directory.
        sync: true,
        alice_granted: true,
        bob_without_book: true,
        ..Setup::default()
    })
    .await;
    let today = period(WALL);
    let server = identity_server().account();
    accept_grants(&mut rig);
    let alices = grant_alice(&mut rig, today, 0);
    let time = rig.clock.wall();
    let before = rig.nodes[ALICE]
        .core
        .send_message(&c, "before revocation", "r1", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &before.id) && learned(r, &alices) == HOLDERS
    })
    .await;
    assert!(stored(&rig, ALICE, &before.id), "{:?}", rig.trace);
    let read_before: Vec<usize> = (0..HOLDERS)
        .map(|h| rig.chain.grant_reads(h, &server, today))
        .collect();
    assert!(read_before.iter().all(|r| *r == 1), "{read_before:?}");
    // The owner ends the key from today.
    rig.chain.set_grants(test_support::FakeGrants {
        issuers: BTreeMap::from([(server, (0, today))]),
        caps: BTreeMap::from([(0, 1_000)]),
        book_size: 100,
        max_validity_days: 30,
    });
    idle(&mut rig, 11 * 60).await;
    let bob_key = rig.nodes[BOB].core.mailbox_book_account().unwrap();
    rig.nodes[BOB]
        .add_mailbox_grant(&grant(bob_key, today, 1))
        .unwrap();
    let after = rig.nodes[BOB]
        .core
        .send_message(&c, "after revocation", "r2", rig.clock.wall())
        .unwrap();
    let still = rig.nodes[ALICE]
        .core
        .send_message(&c, "on the learned grant", "r3", rig.clock.wall())
        .unwrap();
    idle(&mut rig, 120).await;
    assert!(!stored(&rig, BOB, &after.id));
    assert!(stored(&rig, ALICE, &still.id), "{:?}", rig.trace);
    let to_alice = rig.nodes[BOB]
        .core
        .swarm_mailbox(&c, false, rig.clock.wall())
        .unwrap();
    for holder in 0..HOLDERS {
        assert_eq!(held(&rig, holder, &to_alice), 0, "holder {holder}");
        assert_eq!(
            rig.chain.grant_reads(holder, &server, today),
            2,
            "holder {holder}"
        );
    }
}

// --- buying a book ---------------------------------------------------------------------

/// $1.00 a book, or 0.0004 ETH at $2500 an ETH.
fn shop_terms() -> crate::runtime::chain::ShopTerms {
    crate::runtime::chain::ShopTerms {
        address: [0x5b; 20],
        chain_id: 84_532,
        price_usdc: 1_000_000,
        usdc: [0x0c; 20],
        quote: Some(400_000_000_000_000),
        book_size: 100,
        validity: 30 * PERIOD_SECONDS,
    }
}

/// `coins_buy` until the shop's terms are read: until then no request is
/// made.
async fn buy_request(rig: &mut Rig, node: usize) -> Value {
    let requests = rig.nodes[node].core.mailbox_purchases().unwrap().len();
    let mut answer = Value::Null;
    for _ in 0..20 {
        answer = rig.nodes[node].command("coins_buy", json!({}));
        if answer.get("result").is_some() {
            return answer["result"].clone();
        }
        assert_eq!(answer["error"]["code"], "chain_pending", "{answer}");
        assert_eq!(
            rig.nodes[node].core.mailbox_purchases().unwrap().len(),
            requests
        );
        rig.step().await;
    }
    panic!("no payment request: {answer}");
}

/// `coins buy` quotes a payment for a fresh book of the profile's key; the
/// node notices the confirmed purchase itself, and the message waiting for
/// a book goes out paid by it.
#[tokio::test(flavor = "current_thread")]
async fn a_buyers_node_notices_its_paid_purchase_and_pays_with_it() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        books_on_chain: true,
        // Alice starts without any book.
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    rig.chain.set_shop(shop_terms());
    // The shop's terms are read first: until then there is no price, and
    // no request is made.
    let payment = buy_request(&mut rig, ALICE).await;
    let key = rig.nodes[ALICE].core.mailbox_book_account().unwrap();
    let salt: [u8; 32] = hex::decode(payment["salt"].as_str().unwrap().trim_start_matches("0x"))
        .unwrap()
        .try_into()
        .unwrap();
    let book = agentic_mailbox_swarm::stamp::book_id(&NETWORK_DOMAIN, &key, &salt);
    let (shop, key_hex, salt_hex) = (hex::encode([0x5b; 20]), hex::encode(key), hex::encode(salt));
    assert_eq!(
        payment["book"],
        format!("0x{}", hex::encode(book)),
        "{payment}"
    );
    assert_eq!(payment["key"], format!("0x{key_hex}"));
    assert_eq!(payment["shop"], format!("0x{shop}"));
    assert_eq!(payment["chainId"], 84_532);
    assert_eq!(payment["count"], 100);
    assert_eq!(payment["priceUsdc"], "1000000");
    // In ETH: the quote and one percent for the rate to move before the
    // payment lands; the shop gives back what is over.
    assert_eq!(
        payment["eth"],
        json!({
            "to": format!("0x{shop}"),
            "quote": "400000000000000",
            "value": "404000000000000",
            "calldata": format!("0x9058e228{key_hex:0>64}{salt_hex}"),
            "uri": format!(
                "ethereum:0x{shop}@84532/buy?address=0x{key_hex}&bytes32=0x{salt_hex}&value=404000000000000"
            ),
        })
    );
    // In USDC: allow the shop the price, then buy.
    let usdc = hex::encode([0x0c; 20]);
    assert_eq!(
        payment["usdc"],
        json!({
            "token": format!("0x{usdc}"),
            "amount": "1000000",
            "approve": {
                "to": format!("0x{usdc}"),
                "calldata": format!("0x095ea7b3{shop:0>64}{:064x}", 1_000_000),
                "uri": format!("ethereum:0x{usdc}@84532/approve?address=0x{shop}&uint256=1000000"),
            },
            "buy": {
                "to": format!("0x{shop}"),
                "calldata": format!("0xa10572fd{key_hex:0>64}{salt_hex}"),
                "uri": format!(
                    "ethereum:0x{shop}@84532/buyWithUsdc?address=0x{key_hex}&bytes32=0x{salt_hex}"
                ),
            },
        })
    );
    // Unpaid, a message waits for a book.
    let sent = send_alice(&mut rig, &c, "with own coins", 1);
    let started = rig.clock.instant();
    idle(&mut rig, 60).await;
    assert!(!stored(&rig, ALICE, &sent[0].id));
    let pending = rig.nodes[ALICE].command("coins_balance", json!({}))["result"].clone();
    assert_eq!(pending["remaining"], 0, "{pending}");
    assert_eq!(pending["pending"][0]["book"], payment["book"]);
    // Paid and confirmed: the node sees it and the message goes out.
    rig.chain.buy(
        book,
        crate::runtime::chain::BookRecord {
            key,
            count: 100,
            valid_until: rig.clock.wall() + 30 * PERIOD_SECONDS,
        },
    );
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent[0].id))
        .await;
    assert!(stored(&rig, ALICE, &sent[0].id), "{:?}", rig.trace);
    let balance = rig.nodes[ALICE].command("coins_balance", json!({}))["result"].clone();
    assert_eq!(balance["books"][0]["book"], payment["book"], "{balance}");
    assert_eq!(balance["books"][0]["kind"], "bought");
    assert_eq!(balance["books"][0]["used"], 1);
    assert!(balance["books"][0]["validUntil"].as_u64().unwrap() > rig.clock.wall());
    assert_eq!(balance["remaining"], 99);
    assert_eq!(balance["pending"], json!([]));
    // The node asked the chain about its purchase about every half minute,
    // and stops once it has the book.
    let elapsed = (rig.clock.instant() - started).as_secs();
    let reads = rig.chain.book_reads(ALICE, &book);
    assert!(
        (1..=elapsed / 30 + 2).contains(&(reads as u64)),
        "{reads} reads in {elapsed} s"
    );
    idle(&mut rig, 90).await;
    assert_eq!(rig.chain.book_reads(ALICE, &book), reads);
}

/// The ETH quote follows the rate: a quote is at most a minute old, then
/// read again. Without a fresh rate there is no ETH payment, only USDC.
#[tokio::test(flavor = "current_thread")]
async fn the_eth_quote_follows_the_rate_and_a_stale_rate_leaves_usdc() {
    let mut rig = Rig::new(1, WALL);
    rig.pass_serviced = true;
    rig.nodes[0].core.create_profile("Alice").unwrap();
    rig.chain.set_shop(shop_terms());
    let first = buy_request(&mut rig, 0).await;
    assert_eq!(first["eth"]["quote"], "400000000000000");
    // ETH at $2000 now: within the minute the node still quotes the read
    // rate, after it the new one.
    rig.chain.set_shop(crate::runtime::chain::ShopTerms {
        quote: Some(500_000_000_000_000),
        ..shop_terms()
    });
    idle(&mut rig, 30).await;
    assert_eq!(
        buy_request(&mut rig, 0).await["eth"]["quote"],
        "400000000000000"
    );
    idle(&mut rig, 31).await;
    let later = buy_request(&mut rig, 0).await;
    assert_eq!(later["eth"]["quote"], "500000000000000", "{later}");
    assert_eq!(later["eth"]["value"], "505000000000000");
    // The feed went stale: USDC only.
    rig.chain.set_shop(crate::runtime::chain::ShopTerms {
        quote: None,
        ..shop_terms()
    });
    idle(&mut rig, 61).await;
    let stale = buy_request(&mut rig, 0).await;
    assert_eq!(stale["eth"], Value::Null, "{stale}");
    assert_eq!(stale["usdc"]["amount"], "1000000");
    // A new quote is for the same unpaid book: a payment made with an
    // earlier quote still counts.
    for answer in [&later, &stale] {
        assert_eq!(answer["book"], first["book"]);
    }
    assert_eq!(rig.nodes[0].core.mailbox_purchases().unwrap().len(), 1);
}

/// A request paid more than an hour later is still noticed: the node keeps
/// reading it, every half minute for its first hour and every ten minutes
/// after that.
#[tokio::test(flavor = "current_thread")]
async fn a_purchase_paid_after_an_hour_is_still_noticed() {
    let mut rig = Rig::new(1, WALL);
    rig.pass_serviced = true;
    rig.nodes[0].core.create_profile("Alice").unwrap();
    rig.chain.set_shop(shop_terms());
    let payment = buy_request(&mut rig, 0).await;
    let book: [u8; 32] = hex::decode(payment["book"].as_str().unwrap().trim_start_matches("0x"))
        .unwrap()
        .try_into()
        .unwrap();
    idle(&mut rig, 61 * 60).await;
    let first_hour = rig.chain.book_reads(0, &book);
    assert!((115..=125).contains(&first_hour), "{first_hour} reads");
    // Still unpaid: every ten minutes now.
    idle(&mut rig, 25 * 60).await;
    let slower = rig.chain.book_reads(0, &book) - first_hour;
    assert!((2..=3).contains(&slower), "{slower} reads in 25 minutes");
    let first_hour = first_hour + slower;
    let key = rig.nodes[0].core.mailbox_book_account().unwrap();
    rig.chain.buy(
        book,
        crate::runtime::chain::BookRecord {
            key,
            count: 100,
            valid_until: rig.clock.wall() + 30 * PERIOD_SECONDS,
        },
    );
    // Noticed within one slow interval; nothing is scheduled after that.
    let paid = rig.clock.instant();
    rig.run_until(STEPS, |r| {
        r.nodes[0].core.mailbox_purchases().unwrap().is_empty()
    })
    .await;
    assert!(rig.clock.instant() - paid <= Duration::from_secs(11 * 60));
    let balance = rig.nodes[0].command("coins_balance", json!({}))["result"].clone();
    assert_eq!(balance["books"][0]["book"], payment["book"], "{balance}");
    assert_eq!(balance["books"][0]["kind"], "bought");
    assert_eq!(balance["pending"], json!([]));
    assert_eq!(balance["remaining"], 100);
    let later = rig.chain.book_reads(0, &book) - first_hour;
    assert!(
        (1..=2).contains(&later),
        "{later} reads after the first hour"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_node_without_chain_flags_sells_nothing() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut runtime = test_support::runtime(dir.path());
    runtime.core.create_profile("Alice").unwrap();
    let answer = runtime.command("coins_buy", json!({}));
    assert_eq!(answer["error"]["code"], "chain_not_configured", "{answer}");
    assert!(runtime.core.mailbox_purchases().unwrap().is_empty());
}

// --- claiming a grant ----------------------------------------------------------------

/// `coins_claim` until the identity server opened the claim: until then the
/// node answers `claim_pending`.
async fn claim_link(rig: &mut Rig, node: usize) -> Value {
    let mut answer = Value::Null;
    for _ in 0..20 {
        answer = rig.nodes[node].command("coins_claim", json!({}));
        if answer.get("result").is_some() {
            return answer["result"].clone();
        }
        assert_eq!(answer["error"]["code"], "claim_pending", "{answer}");
        rig.step().await;
    }
    panic!("no claim link: {answer}");
}

/// `coins claim`: the node asks the identity server with a request its book
/// key signed and hands out the login link; once the human signed in, the
/// node collects the grant itself and it pays like any book.
#[tokio::test(flavor = "current_thread")]
async fn a_claimed_grant_pays_once_the_human_signs_in() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let server = test_support::FakeIdentity::default();
    rig.nodes[ALICE].set_identity(server.server());
    let opened = claim_link(&mut rig, ALICE).await;
    let key = rig.nodes[ALICE].core.mailbox_book_account().unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].book, key);
    let claim = opened["claimId"].as_str().unwrap().to_owned();
    assert_eq!(
        opened["loginUrl"],
        format!("https://id.test/v1/claims/{claim}/login")
    );
    assert_eq!(opened["status"], "open");
    // Asked again, the same link, and nothing posted again.
    assert_eq!(
        rig.nodes[ALICE].command("coins_claim", json!({}))["result"],
        opened
    );
    assert_eq!(server.requests().len(), 1);
    // Until the human signs in the node asks every few seconds; no book.
    idle(&mut rig, 60).await;
    let polls = server.polls();
    assert!((6..=15).contains(&polls), "{polls} status reads");
    assert!(rig.nodes[ALICE].core.mailbox_books().unwrap().is_empty());
    let today = period(rig.clock.wall());
    server.sign_in(&claim, grant(key, today, 0));
    let sent = send_alice(&mut rig, &c, "on a request", 1);
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent[0].id))
        .await;
    assert!(stored(&rig, ALICE, &sent[0].id), "{:?}", rig.trace);
    let balance = rig.nodes[ALICE].command("coins_balance", json!({}))["result"].clone();
    assert_eq!(balance["books"][0]["kind"], "granted", "{balance}");
    assert_eq!(balance["claim"], Value::Null);
    assert_eq!(balance["lastClaim"]["status"], "granted");
    // Done: the claim is not read any more.
    let after = server.polls();
    idle(&mut rig, 30).await;
    assert_eq!(server.polls(), after);
}

/// A denied claim is reported and closed; the next `coins_claim` starts a
/// new one.
#[tokio::test(flavor = "current_thread")]
async fn a_denied_claim_is_reported_and_closed() {
    let mut rig = Rig::new(1, WALL);
    rig.pass_serviced = true;
    rig.nodes[0].core.create_profile("Alice").unwrap();
    let server = test_support::FakeIdentity::default();
    rig.nodes[0].set_identity(server.server());
    let opened = claim_link(&mut rig, 0).await;
    let claim = opened["claimId"].as_str().unwrap().to_owned();
    server.deny(&claim, "already_claimed");
    rig.run_until(STEPS, |r| {
        r.nodes[0].core.mailbox_claim().unwrap().is_none()
    })
    .await;
    let balance = rig.nodes[0].command("coins_balance", json!({}))["result"].clone();
    assert_eq!(balance["claim"], Value::Null, "{balance}");
    assert_eq!(
        balance["lastClaim"],
        json!({"status": "denied", "reason": "already_claimed"})
    );
    let again = claim_link(&mut rig, 0).await;
    assert_ne!(again["claimId"], opened["claimId"]);
    assert_eq!(server.requests().len(), 2);
}

/// The identity server is down longer than a request lives: the node keeps
/// trying on its own, and once the server is back its stale request is
/// refused and a fresh one opens the claim.
#[tokio::test(flavor = "current_thread")]
async fn a_claim_recovers_from_an_identity_server_outage() {
    let mut rig = Rig::new(1, WALL);
    rig.pass_serviced = true;
    rig.nodes[0].core.create_profile("Alice").unwrap();
    let server = test_support::FakeIdentity::default();
    rig.nodes[0].set_identity(server.server());
    server.set_down(true);
    let answer = rig.nodes[0].command("coins_claim", json!({}));
    assert_eq!(answer["error"]["code"], "claim_pending", "{answer}");
    idle(&mut rig, 20 * 60).await;
    assert!(server.requests().is_empty());
    server.set_down(false);
    let back = rig.clock.wall();
    let opened = claim_link(&mut rig, 0).await;
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    // Not stale when posted: signed within the server's claim TTL.
    assert!(
        requests[0].created_at + 900 >= back,
        "a stale request was opened"
    );
    let claim = rig.nodes[0].core.mailbox_claim().unwrap().unwrap();
    assert_eq!(claim.request, requests[0]);
    assert_eq!(opened["claimId"], claim.claim_id.unwrap());
}

#[tokio::test(flavor = "current_thread")]
async fn a_node_without_an_identity_server_claims_nothing() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut runtime = test_support::runtime(dir.path());
    runtime.core.create_profile("Alice").unwrap();
    let answer = runtime.command("coins_claim", json!({}));
    assert_eq!(
        answer["error"]["code"], "identity_not_configured",
        "{answer}"
    );
    assert!(runtime.core.mailbox_claim().unwrap().is_none());
}

// --- the directory -------------------------------------------------------------------

/// Whether `node` lists exactly `members`, each with its holder's peer,
/// receipt account and some address.
fn lists(rig: &Rig, node: usize, members: &[usize]) -> bool {
    let directory = &rig.nodes[node].mailbox_client.directory;
    directory.len() == members.len()
        && members.iter().all(|m| {
            directory
                .get(&rig.nodes[*m].own_commitment())
                .is_some_and(|holder| {
                    holder.peer == *rig.nodes[*m].swarm.local_peer_id()
                        && holder.account == rig.nodes[*m].mailbox_holder.account()
                        && !holder.addresses.is_empty()
                })
        })
}

/// Nobody is handed a directory: the registry lists the holders, each holds
/// as its own unit, and every node pulls the records; then a message goes
/// through.
#[tokio::test(flavor = "current_thread")]
async fn nodes_build_the_directory_from_the_registry_and_pulled_records() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        directory_from_chain: true,
        ..Setup::default()
    })
    .await;
    let started = rig.clock.instant();
    let all: Vec<usize> = (0..HOLDERS).collect();
    let everyone: Vec<usize> = (0..HOLDERS).chain([ALICE, BOB]).collect();
    rig.run_until(STEPS, |r| everyone.iter().all(|n| lists(r, *n, &all)))
        .await;
    for node in &everyone {
        assert!(lists(&rig, *node, &all), "node {node}: {:?}", rig.trace);
    }
    assert!(rig.clock.instant() - started <= Duration::from_secs(5 * 60));
    for holder in 0..HOLDERS {
        assert_eq!(
            rig.nodes[holder].mailbox_holder.unit(),
            Some(rig.nodes[holder].own_commitment())
        );
    }
    let sent = send_alice(&mut rig, &c, "from the catalog", 1);
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent[0].id) && incoming(r, BOB, &c).len() == 1
    })
    .await;
    assert!(stored(&rig, ALICE, &sent[0].id), "{:?}", rig.trace);
    assert_eq!(incoming(&rig, BOB, &c), ["from the catalog 0"]);
}

/// A unit that exits leaves every directory and stops holding; a node whose
/// commitment is bonded starts holding and every directory lists it.
#[tokio::test(flavor = "current_thread")]
async fn an_exiting_unit_leaves_and_a_bonded_node_joins_every_directory() {
    let Chat {
        mut rig,
        conversation,
        ..
    } = chat(Setup {
        sync: true,
        directory_from_chain: true,
        spare: true,
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    let watchers: Vec<usize> = (0..HOLDERS).chain([ALICE, BOB]).collect();
    rig.run_until(STEPS, |r| watchers.iter().all(|n| lists(r, *n, &all)))
        .await;
    assert_eq!(rig.nodes[SPARE].mailbox_holder.unit(), None);
    let after: Vec<usize> = (0..HOLDERS).filter(|h| *h != 3).chain([SPARE]).collect();
    let units = after
        .iter()
        .map(|h| rig.nodes[*h].own_commitment())
        .collect();
    rig.chain.set_units(units);
    let changed = rig.clock.instant();
    let watchers: Vec<usize> = after.iter().copied().chain([ALICE, BOB]).collect();
    rig.run_until(STEPS, |r| watchers.iter().all(|n| lists(r, *n, &after)))
        .await;
    for node in &watchers {
        assert!(lists(&rig, *node, &after), "node {node}: {:?}", rig.trace);
    }
    // Two registry reads and a pull.
    assert!(rig.clock.instant() - changed <= Duration::from_secs(25 * 60));
    assert_eq!(
        rig.nodes[SPARE].mailbox_holder.unit(),
        Some(rig.nodes[SPARE].own_commitment())
    );
    assert_eq!(rig.nodes[3].mailbox_holder.unit(), None);
    // The new swarm is used: the spare takes the message, holder 3 does not.
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&conversation, "new membership", "after-exit", time)
        .unwrap();
    let mailbox = rig.nodes[ALICE]
        .core
        .swarm_mailbox(&conversation, false, time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &sent.id) && held(r, SPARE, &mailbox) == 1
    })
    .await;
    assert!(stored(&rig, ALICE, &sent.id), "{:?}", rig.trace);
    assert_eq!(held(&rig, SPARE, &mailbox), 1);
    assert_eq!(held(&rig, 3, &mailbox), 0);
}

/// The registry does not answer for longer than a read interval: nodes keep
/// the directory and their units, and messages still go through.
#[tokio::test(flavor = "current_thread")]
async fn a_registry_outage_keeps_the_directory() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        directory_from_chain: true,
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    let everyone: Vec<usize> = (0..HOLDERS).chain([ALICE, BOB]).collect();
    rig.run_until(STEPS, |r| everyone.iter().all(|n| lists(r, *n, &all)))
        .await;
    rig.chain.set_down(true);
    idle(&mut rig, 15 * 60).await;
    for node in &everyone {
        assert!(lists(&rig, *node, &all), "node {node}");
    }
    for holder in 0..HOLDERS {
        assert_eq!(
            rig.nodes[holder].mailbox_holder.unit(),
            Some(rig.nodes[holder].own_commitment())
        );
    }
    let sent = send_alice(&mut rig, &c, "without the registry", 1);
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent[0].id))
        .await;
    assert!(stored(&rig, ALICE, &sent[0].id), "{:?}", rig.trace);
}

/// A holder whose first registry read fails — the testnet's node-10 after
/// the redeploy of 2026-09-30, whose RPC answered that read with an error —
/// reads again about a minute later, not ten, and keeps doing so while the
/// RPC is down; once a read succeeds it holds and every directory lists it
/// within minutes. A read that succeeded is not repeated at that pace.
#[tokio::test(flavor = "current_thread")]
async fn a_holder_whose_registry_read_failed_holds_within_minutes() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        directory_from_chain: true,
        ..Setup::default()
    })
    .await;
    let late = HOLDERS - 1;
    let others: Vec<usize> = (0..late).collect();
    let failures = |r: &Rig| r.nodes[late].chain.info()["failures"].as_u64().unwrap();
    // Its read is answered only after the others hold and list each other,
    // and with an error.
    rig.chain.hold(late);
    rig.run_until(STEPS, |r| others.iter().all(|n| lists(r, *n, &others)))
        .await;
    for node in &others {
        assert!(lists(&rig, *node, &others), "node {node}: {:?}", rig.trace);
    }
    rig.chain.set_down(true);
    rig.chain.release(late);
    rig.run_until(STEPS, |r| failures(r) == 1).await;
    assert_eq!(rig.chain.units_reads(late), 1);
    assert_eq!(rig.nodes[late].mailbox_holder.unit(), None);
    let failed = rig.clock.instant();
    // The RPC still fails the next read.
    rig.run_until(STEPS, |r| failures(r) == 2).await;
    let retried = rig.clock.instant() - failed;
    assert!(
        (Duration::from_secs(30)..=Duration::from_secs(2 * 60)).contains(&retried),
        "{retried:?}"
    );
    assert_eq!(rig.chain.units_reads(late), 2);
    assert_eq!(rig.nodes[late].mailbox_holder.unit(), None);
    rig.chain.set_down(false);
    let up = rig.clock.instant();
    let own = rig.nodes[late].own_commitment();
    rig.run_until(STEPS, |r| r.nodes[late].mailbox_holder.unit() == Some(own))
        .await;
    assert_eq!(
        rig.nodes[late].mailbox_holder.unit(),
        Some(own),
        "{:?}",
        rig.trace
    );
    let held_after = rig.clock.instant() - up;
    assert!(held_after <= Duration::from_secs(2 * 60), "{held_after:?}");
    let all: Vec<usize> = (0..HOLDERS).collect();
    let everyone: Vec<usize> = (0..HOLDERS).chain([ALICE, BOB]).collect();
    rig.run_until(STEPS, |r| everyone.iter().all(|n| lists(r, *n, &all)))
        .await;
    for node in &everyone {
        assert!(lists(&rig, *node, &all), "node {node}: {:?}", rig.trace);
    }
    let listed_after = rig.clock.instant() - up;
    assert!(
        listed_after <= Duration::from_secs(3 * 60),
        "{listed_after:?}"
    );
    // The others read once, before any of this; the late holder has read
    // twice in vain and once with success.
    for holder in &others {
        assert_eq!(rig.chain.units_reads(*holder), 1, "holder {holder}");
    }
    assert_eq!(rig.chain.units_reads(late), 3);
}

/// Of the records a peer sends, only the newest verified one of each active
/// unit is taken: forged, stale and unregistered ones are not.
#[tokio::test(flavor = "current_thread")]
async fn a_directory_page_takes_only_the_newest_verified_records_of_active_units() {
    let mut rig = Rig::new(3, WALL);
    rig.pass_serviced = true;
    let (a, b) = (rig.nodes[1].own_commitment(), rig.nodes[2].own_commitment());
    rig.chain.set_units(vec![a, b]);
    rig.run_until(STEPS, |r| r.nodes[0].directory_units() == 2)
        .await;
    assert_eq!(rig.nodes[0].directory_units(), 2);
    let time = rig.clock.wall();
    let first = rig.nodes[1].unit_record(time);
    let older = rig.nodes[2].unit_record(time - 10);
    let mut newer = rig.nodes[2].unit_record(time);
    newer.addresses = vec!["/ip4/127.0.0.1/tcp/4002".into()];
    let newer = agentic_mailbox_swarm::directory::UnitRecord::sign(
        &NETWORK_DOMAIN,
        newer.transport_key,
        newer.receipt,
        newer.addresses,
        newer.issued_at,
        |digest| {
            let key = rig.nodes[2].transport_key.clone();
            key.sign(digest).unwrap().try_into().unwrap()
        },
    );
    // Node 1's unit claimed by node 2's key, and node 1's key with another
    // receipt account (a commitment nobody bonded).
    let mut stolen = rig.nodes[2].unit_record(time + 5);
    stolen.transport_key = first.transport_key;
    let unbonded = agentic_mailbox_swarm::directory::UnitRecord::sign(
        &NETWORK_DOMAIN,
        first.transport_key,
        [0x99; 20],
        first.addresses.clone(),
        time + 5,
        |digest| {
            let key = rig.nodes[1].transport_key.clone();
            key.sign(digest).unwrap().try_into().unwrap()
        },
    );
    // A record dated beyond clock skew would beat every later correction.
    let ahead = agentic_mailbox_swarm::directory::UnitRecord::sign(
        &NETWORK_DOMAIN,
        first.transport_key,
        first.receipt,
        vec!["/ip4/127.0.0.1/tcp/4009".into()],
        time + 3_600,
        |digest| {
            let key = rig.nodes[1].transport_key.clone();
            key.sign(digest).unwrap().try_into().unwrap()
        },
    );
    let observer = rig.nodes[0].unit_record(time);
    let mut page: Vec<_> = [
        &newer, &first, &older, &stolen, &unbonded, &ahead, &observer,
    ]
    .map(mailbox_holder::UnitRecordWire::from)
    .to_vec();
    // One malformed record spoils nothing else.
    let mut broken = mailbox_holder::UnitRecordWire::from(&first);
    broken.transport_key.pop();
    page.insert(1, broken);
    rig.nodes[0].directory_page(page);
    let directory = &rig.nodes[0].mailbox_client.directory;
    assert_eq!(directory.keys().copied().collect::<Vec<_>>().len(), 2);
    let listed = |unit: &[u8; 32]| directory.get(unit).unwrap().clone();
    assert_eq!(listed(&a).peer, *rig.nodes[1].swarm.local_peer_id());
    assert_eq!(
        listed(&a).addresses,
        first
            .addresses
            .iter()
            .map(|a| a.parse::<Multiaddr>().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(listed(&a).account, rig.nodes[1].mailbox_holder.account());
    assert_eq!(
        listed(&b).addresses,
        vec!["/ip4/127.0.0.1/tcp/4002".parse::<Multiaddr>().unwrap()]
    );
    // And it serves what it took.
    let served = rig.nodes[0].directory_records();
    assert_eq!(served.len(), 2);
}

// --- held envelopes ------------------------------------------------------------------

/// Envelopes that reach the reader out of their MLS order are held and put
/// through in order once their predecessor arrives: each is tried a bounded
/// number of times, not again after every import (which stalled a daemon
/// reading 258 messages in the native spike).
#[tokio::test(flavor = "current_thread")]
async fn out_of_order_envelopes_are_imported_in_order_trying_each_a_few_times() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        alice_without_directory: true,
        ..Setup::default()
    })
    .await;
    const COUNT: usize = 60;
    let words: Vec<String> = (0..COUNT).map(|n| format!("out of order {n}")).collect();
    let deliveries = prepared(&mut rig, &c, &words);
    // Every holder took the newer half newest first, then the older half:
    // envelopes past a gap wait while earlier ones are still imported.
    let order = (COUNT / 2..COUNT).rev().chain(0..COUNT / 2);
    for index in order {
        for holder in 0..HOLDERS {
            store_at(&mut rig, holder, &deliveries[index]);
        }
    }
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == COUNT)
        .await;
    assert_eq!(incoming(&rig, BOB, &c), words, "{:?}", rig.trace);
    let attempts = rig.nodes[BOB].mailbox_client.info()["importAttempts"]
        .as_u64()
        .unwrap();
    assert!(
        attempts <= 3 * COUNT as u64,
        "{attempts} import attempts for {COUNT} messages"
    );
}

// --- direct delivery pays --------------------------------------------------------------

/// Direct delivery pays with the stamp of the message's swarm copy: the
/// recipient's node checks it like a holder, and one slot pays for both.
#[tokio::test(flavor = "current_thread")]
async fn a_direct_message_pays_with_the_stamp_of_its_swarm_copy() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        direct: true,
        books_on_chain: true,
        // Bob reads no holder: what he gets comes directly.
        bob_offline: true,
        // No quorum can form: "delivered" comes only from Bob's receipt.
        offline: vec![0, 1, 2, 3],
        ..Setup::default()
    })
    .await;
    let sent = send_alice(&mut rig, &c, "directly", 1);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 1)
        .await;
    assert_eq!(incoming(&rig, BOB, &c), ["directly 0"], "{:?}", rig.trace);
    assert_eq!(rig.nodes[BOB].mailbox_client.info()["received"], 0);
    // Bob's node read Alice's book to check the stamp.
    assert_eq!(rig.chain.book_reads(BOB, &alice_book()), 1);
    // Delivered by Bob's receipt; one slot for the direct and swarm copy.
    rig.run_until(STEPS, |r| {
        r.nodes[ALICE].core.snapshot().unwrap().conversations[0].messages[0]
            .delivery
            .phase
            == "delivered"
    })
    .await;
    let used = rig.nodes[ALICE]
        .core
        .mailbox_books()
        .unwrap()
        .into_iter()
        .find(|b| b.book == alice_book())
        .unwrap()
        .used;
    assert_eq!(used, 1);
    let _ = sent;
}

/// Whether each of `node`'s incoming messages in `conversation` is shown
/// with low trust.
fn low_trust(rig: &Rig, node: usize, conversation: &str) -> Vec<bool> {
    rig.nodes[node]
        .core
        .snapshot()
        .unwrap()
        .conversations
        .into_iter()
        .find(|c| c.id == conversation)
        .map(|c| {
            c.messages
                .into_iter()
                .filter(|m| !m.own)
                .map(|m| m.low_trust)
                .collect()
        })
        .unwrap_or_default()
}

/// The owner's inbox of `node` as its IPC answers it: the first page's items.
fn inbox_items(rig: &mut Rig, node: usize, conversation: &str) -> Vec<Value> {
    let answer = rig.nodes[node].command(
        "inbox_poll",
        json!({"conversationId": conversation, "limit": 10, "leaseSeconds": 60}),
    );
    answer["result"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("{answer}"))
}

/// The delivery phase of Alice's first message in `conversation`.
fn alice_phase(rig: &Rig, conversation: &str) -> String {
    rig.nodes[ALICE]
        .core
        .snapshot()
        .unwrap()
        .conversations
        .into_iter()
        .find(|c| c.id == conversation)
        .unwrap()
        .messages[0]
        .delivery
        .phase
        .clone()
}

/// A book of Alice's the chain shows once bought (`confirmed`), or does not
/// show yet.
fn alice_bought(rig: &mut Rig, confirmed: bool) -> [u8; 32] {
    let key = rig.nodes[ALICE].core.mailbox_book_account().unwrap();
    let book = [0xb7; 32];
    let valid_until = rig.clock.wall() + 30 * PERIOD_SECONDS;
    rig.nodes[ALICE]
        .core
        .add_mailbox_book(book, 100, valid_until)
        .unwrap();
    let record = crate::runtime::chain::BookRecord {
        key,
        count: 100,
        valid_until,
    };
    if confirmed {
        rig.chain.buy(book, record);
    } else {
        rig.chain.buy_unconfirmed(book, record);
    }
    book
}

/// The ticket of the stamp Alice's node pays her message `id` with.
fn alice_ticket(rig: &mut Rig, id: &str) -> [u8; 32] {
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .prepare_swarm_delivery(id, time)
        .unwrap()
        .stamp
        .ticket_id(&NETWORK_DOMAIN)
}

/// A stamp of a granted book, sent directly, is checked the way holders
/// check it: the sender shows the grant, the recipient's node reads its
/// issuer's rules and asks its notaries, never the shop, and takes the
/// message paid. Other chain reads failing meanwhile (the rig's registry
/// never answers) lower no trust.
#[tokio::test(flavor = "current_thread")]
async fn a_granted_stamp_sent_directly_pays_once_its_notaries_vouch() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        // Exact counts of this lane.
        silent_cards: true,
        direct: true,
        books_on_chain: true,
        alice_granted: true,
        // Alice reaches no holder: what Bob gets comes directly. Bob reaches
        // every holder, the grant's notaries among them.
        hidden_from_alice: (0..HOLDERS).collect(),
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let grant = grant_alice(&mut rig, period(WALL), 0);
    send_alice(&mut rig, &c, "on the grant", 1);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 1)
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["on the grant 0"],
        "{:?}",
        rig.trace
    );
    assert_eq!(low_trust(&rig, BOB, &c), [false]);
    let items = inbox_items(&mut rig, BOB, &c);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].get("lowTrust"), None, "{items:?}");
    assert!(rig.nodes[BOB].chain.info()["failures"].as_u64().unwrap() > 0);
    let mailbox = to_bob(&rig, &c);
    assert!((0..HOLDERS).all(|h| held(&rig, h, &mailbox) == 0));
    // Checked by its grant once; the shop was never asked for the book.
    assert_eq!(grant_checks(&rig, BOB), (1, 1, 0));
    assert_eq!(
        rig.nodes[BOB].mailbox_holder.granted(&grant.id()).unwrap(),
        Some(grant.clone())
    );
    assert_eq!(rig.chain.book_reads(BOB, &grant.id()), 0);
    // Delivered by Bob's receipt, for one slot of the grant.
    rig.run_until(STEPS, |r| alice_phase(r, &c) == "delivered")
        .await;
    assert_eq!(alice_phase(&rig, &c), "delivered", "{:?}", rig.trace);
    assert_eq!(used(&rig, ALICE), 1);
}

/// A grant its notaries first saw after its day pays nothing directly
/// either: while the chain answers, the recipient refuses it as holders do.
#[tokio::test(flavor = "current_thread")]
async fn a_granted_stamp_its_notaries_saw_late_is_refused_directly() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        silent_cards: true,
        direct: true,
        books_on_chain: true,
        alice_granted: true,
        hidden_from_alice: (0..HOLDERS).collect(),
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    // Two days old and never on record.
    let grant = grant_alice(&mut rig, period(WALL) - 2, 0);
    send_alice(&mut rig, &c, "backdated", 1);
    rig.run_until(STEPS, |r| grant_checks(r, BOB).2 == 1).await;
    assert_eq!(grant_checks(&rig, BOB), (1, 0, 1), "{:?}", rig.trace);
    idle(&mut rig, 61).await;
    assert!(incoming(&rig, BOB, &c).is_empty());
    assert_eq!(
        rig.nodes[BOB].mailbox_holder.granted(&grant.id()).unwrap(),
        None
    );
    assert_eq!(alice_phase(&rig, &c), "queued");
}

/// A user funded by a grant writes a contact on a LAN without the
/// Internet: no holder answers and the chain does not either. The
/// recipient's node cannot read the grant's issuer rules, so it takes the
/// message with low trust, and says so.
#[tokio::test(flavor = "current_thread")]
async fn an_unreachable_chain_takes_a_contacts_granted_stamp_with_low_trust() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        silent_cards: true,
        direct: true,
        books_on_chain: true,
        alice_granted: true,
        bob_offline: true,
        offline: (0..HOLDERS).collect(),
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let grant = grant_alice(&mut rig, period(WALL), 0);
    rig.chain.set_down(true);
    send_alice(&mut rig, &c, "offline", 1);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 1)
        .await;
    assert_eq!(incoming(&rig, BOB, &c), ["offline 0"], "{:?}", rig.trace);
    assert_eq!(low_trust(&rig, BOB, &c), [true]);
    let items = inbox_items(&mut rig, BOB, &c);
    assert_eq!(items[0]["lowTrust"], true, "{items:?}");
    // Bob's node tried the grant's issuer rules, not the shop, and learned
    // nothing it could not check.
    assert!(rig.chain.grant_reads(BOB, &grant.server, grant.day) >= 1);
    assert_eq!(rig.chain.book_reads(BOB, &grant.id()), 0);
    assert_eq!(
        rig.nodes[BOB].mailbox_holder.granted(&grant.id()).unwrap(),
        None
    );
    rig.run_until(STEPS, |r| alice_phase(r, &c) == "delivered")
        .await;
    assert_eq!(alice_phase(&rig, &c), "delivered", "{:?}", rig.trace);
}

/// A contact's direct message paid from a book the chain does not show is
/// refused while the chain answers, taken with low trust while it does not
/// (its slot is not taken as evidence), and checked again once it answers;
/// a book known here is checked without the chain.
#[tokio::test(flavor = "current_thread")]
async fn a_contacts_unknown_book_is_taken_with_low_trust_only_while_the_chain_is_unreachable() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        silent_cards: true,
        direct: true,
        books_on_chain: true,
        // Alice's only book is the one below.
        alice_granted: true,
        bob_offline: true,
        offline: (0..HOLDERS).collect(),
        ..Setup::default()
    })
    .await;
    // Bought, but below the confirmations: the chain answers it is absent.
    let book = alice_bought(&mut rig, false);
    let first = send_alice(&mut rig, &c, "unconfirmed", 1).remove(0);
    let unchecked = alice_ticket(&mut rig, &first.id);
    // Within the minute an absent book is not read again (`ABSENT_FOR`).
    idle(&mut rig, 50).await;
    assert!(incoming(&rig, BOB, &c).is_empty());
    assert_eq!(rig.chain.book_reads(BOB, &book), 1);
    // The chain stops answering: Bob's next read of the book fails.
    rig.chain.set_down(true);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 1)
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["unconfirmed 0"],
        "{:?}",
        rig.trace
    );
    assert_eq!(low_trust(&rig, BOB, &c), [true]);
    assert_eq!(
        rig.nodes[BOB]
            .mailbox_holder
            .notary_record(&unchecked)
            .unwrap(),
        None
    );
    rig.run_until(STEPS, |r| alice_phase(r, &c) == "delivered")
        .await;
    assert_eq!(alice_phase(&rig, &c), "delivered", "{:?}", rig.trace);
    // It answers again, with the purchase confirmed. Past the minute, the
    // message that finds it back may still be taken unchecked; once Bob's
    // node has read the book, its stamps are checked.
    rig.chain.set_down(false);
    rig.chain.confirm(&book);
    idle(&mut rig, 61).await;
    send_alice(&mut rig, &c, "back", 1);
    rig.run_until(STEPS, |r| {
        incoming(r, BOB, &c).len() == 2 && r.nodes[BOB].mailbox_holder.knows_book(&book)
    })
    .await;
    assert!(
        rig.nodes[BOB].mailbox_holder.knows_book(&book),
        "{:?}",
        rig.trace
    );
    let checked = send_alice(&mut rig, &c, "checked", 1).remove(0);
    let taken = alice_ticket(&mut rig, &checked.id);
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 3)
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["unconfirmed 0", "back 0", "checked 0"],
        "{:?}",
        rig.trace
    );
    assert!(!low_trust(&rig, BOB, &c)[2]);
    assert!(
        rig.nodes[BOB]
            .mailbox_holder
            .notary_record(&taken)
            .unwrap()
            .is_some()
    );
    // The chain goes again: a book known here needs none. Its stamps are
    // checked as ever, and a message pays with its own slot only.
    rig.chain.set_down(true);
    let known = send_alice(&mut rig, &c, "known", 1).remove(0);
    let time = rig.clock.wall();
    let paid = rig.nodes[ALICE]
        .core
        .prepare_swarm_delivery(&known.id, time)
        .unwrap();
    let mut forged = StampedDelivery::from(&paid);
    forged.stamp.index += 1;
    let alice = *rig.nodes[ALICE].swarm.local_peer_id();
    let delivery = Delivery {
        node_record: rig.nodes[ALICE].binding(time).unwrap(),
        envelope: vec![],
        stamped: Some(forged),
    };
    assert!(
        rig.nodes[BOB]
            .receive(&delivery, alice, None, time)
            .is_err()
    );
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 4)
        .await;
    assert_eq!(incoming(&rig, BOB, &c)[3], "known 0", "{:?}", rig.trace);
    assert!(!low_trust(&rig, BOB, &c)[3]);
}

/// Low trust is for contacts: a stranger's contact request is taken
/// directly only with a stamp this node checked.
#[tokio::test(flavor = "current_thread")]
async fn an_unreachable_chain_takes_no_strangers_request_unchecked() {
    let Chat { mut rig, .. } = chat(Setup {
        strangers: true,
        silent_cards: true,
        books_on_chain: true,
        bob_offline: true,
        alice_granted: true,
        offline: (0..HOLDERS).collect(),
        ..Setup::default()
    })
    .await;
    rig.chain.set_down(true);
    let time = rig.clock.wall();
    let bob = network_id_of(&rig, BOB);
    let card = rig.nodes[BOB].core.intro_card(vec![], time).unwrap();
    // The chain would show it, if it answered.
    let book = alice_bought(&mut rig, true);
    rig.nodes[ALICE]
        .core
        .request_contact("Bob", &bob, &card.envelope, "ask-1", time)
        .unwrap();
    let id = rig.nodes[ALICE].core.swarm_outbox(64).unwrap()[0]
        .message_id
        .clone();
    let paid = rig.nodes[ALICE]
        .core
        .prepare_swarm_delivery(&id, time)
        .unwrap();
    let delivery = Delivery {
        node_record: rig.nodes[ALICE].binding(time).unwrap(),
        envelope: vec![],
        stamped: Some(StampedDelivery::from(&paid)),
    };
    let alice = *rig.nodes[ALICE].swarm.local_peer_id();
    // Refused while the first read runs, and while its failed read is
    // remembered (within `ABSENT_FOR`): where a contact's message would be
    // taken with low trust.
    assert!(
        rig.nodes[BOB]
            .receive(&delivery, alice, None, time)
            .is_err()
    );
    idle(&mut rig, 1).await;
    assert!(
        rig.nodes[BOB]
            .receive(&delivery, alice, None, time)
            .is_err()
    );
    assert_eq!(rig.chain.book_reads(BOB, &book), 1);
    assert!(contacts(&rig, BOB).is_empty());
    // The same request is taken once the chain answers: it was paid.
    rig.chain.set_down(false);
    idle(&mut rig, 61).await;
    let mut taken = false;
    for _ in 0..50 {
        if rig.nodes[BOB].receive(&delivery, alice, None, time).is_ok() {
            taken = true;
            break;
        }
        rig.step().await;
    }
    assert!(taken, "{:?}", rig.trace);
    assert_eq!(contacts(&rig, BOB).len(), 1);
}

/// A bought book's stamp goes on the wire as it did before grants were
/// carried, both ways: nodes not updated yet refuse unknown fields, and
/// their deliveries carry no grant. (Any self-describing serde format; the
/// wire is CBOR.)
#[test]
fn a_bought_books_stamped_delivery_keeps_the_wire_of_nodes_before_grants() {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Before {
        conversation: String,
        #[serde(with = "serde_bytes")]
        mailbox: Vec<u8>,
        period: u64,
        #[serde(with = "serde_bytes")]
        envelope: Vec<u8>,
        stamp: mailbox_holder::StampWire,
    }
    let stamp = mailbox_holder::StampWire::from(&agentic_mailbox_swarm::stamp::Stamp {
        book: [0xb7; 32],
        index: 0,
        operation: [3; 32],
        signature: [0; 65],
        holders: None,
    });
    let bought = StampedDelivery {
        conversation: "c".into(),
        mailbox: vec![7; 32],
        period: 19_675,
        envelope: vec![1, 2, 3],
        stamp: stamp.clone(),
        grant: None,
    };
    serde_json::from_slice::<Before>(&serde_json::to_vec(&bought).unwrap()).unwrap();
    let before = Before {
        conversation: "c".into(),
        mailbox: vec![7; 32],
        period: 19_675,
        envelope: vec![1, 2, 3],
        stamp,
    };
    let decoded: StampedDelivery =
        serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
    assert!(decoded.grant.is_none());
}

/// A node that reads the chain takes no unpaid message, even directly; it
/// arrives once its sender has a book.
#[tokio::test(flavor = "current_thread")]
async fn a_paying_node_refuses_an_unpaid_direct_message() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        direct: true,
        books_on_chain: true,
        bob_offline: true,
        // No quorum can form: only the direct path can deliver.
        offline: vec![0, 1, 2, 3],
        // Alice has no book.
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    send_alice(&mut rig, &c, "without a book", 1);
    idle(&mut rig, 60).await;
    assert!(incoming(&rig, BOB, &c).is_empty());
    let key = rig.nodes[ALICE].core.mailbox_book_account().unwrap();
    let book = [0xb7; 32];
    let valid_until = rig.clock.wall() + 30 * PERIOD_SECONDS;
    rig.nodes[ALICE]
        .core
        .add_mailbox_book(book, 100, valid_until)
        .unwrap();
    rig.chain.buy(
        book,
        crate::runtime::chain::BookRecord {
            key,
            count: 100,
            valid_until,
        },
    );
    rig.run_until(STEPS, |r| incoming(r, BOB, &c).len() == 1)
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["without a book 0"],
        "{:?}",
        rig.trace
    );
}

/// A stamped delivery counts only for the recipient's own incoming mailbox,
/// with its envelope as stamped and a slot of the sender's book.
#[tokio::test(flavor = "current_thread")]
async fn a_stamped_delivery_counts_only_for_its_own_mailbox_and_stamp() {
    let Chat {
        mut rig,
        conversation: c,
        carol,
        terms,
        ..
    } = chat(Setup {
        carol: true,
        ..Setup::default()
    })
    .await;
    let from_carol = carol.unwrap();
    for (book, terms) in &terms {
        rig.nodes[BOB]
            .mailbox_holder
            .learn_book(*book, *terms)
            .unwrap();
    }
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "stamp check", "m1", time)
        .unwrap();
    let d = rig.nodes[ALICE]
        .core
        .prepare_swarm_delivery(&sent.id, time)
        .unwrap();
    let record = rig.nodes[ALICE].binding(time).unwrap();
    let alice = *rig.nodes[ALICE].swarm.local_peer_id();
    let delivery = |stamped: StampedDelivery| Delivery {
        node_record: record.clone(),
        envelope: vec![],
        stamped: Some(stamped),
    };
    let good = StampedDelivery::from(&d);
    let mut elsewhere = good.clone();
    elsewhere.conversation = from_carol;
    let mut altered = good.clone();
    altered.envelope[40] ^= 1;
    let mut other_slot = good.clone();
    other_slot.stamp.index += 1;
    for (label, bad) in [
        ("another conversation", elsewhere),
        ("an altered envelope", altered),
        ("another slot", other_slot),
    ] {
        assert!(
            rig.nodes[BOB]
                .receive(&delivery(bad), alice, None, time)
                .is_err(),
            "{label}"
        );
    }
    // Unstamped, an application message is not taken by a paying node.
    let wire = rig.nodes[ALICE]
        .core
        .outbox(100)
        .unwrap()
        .into_iter()
        .find(|item| item.message_id == sent.id)
        .unwrap()
        .wire;
    let unpaid = Delivery {
        node_record: record.clone(),
        envelope: wire,
        stamped: None,
    };
    let refused = rig.nodes[BOB].receive(&unpaid, alice, None, time);
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.to_string().contains("stamp")),
        "{:?}",
        refused.map(|_| ())
    );
    assert!(incoming(&rig, BOB, &c).is_empty());
    let reply = rig.nodes[BOB]
        .receive(&delivery(good), alice, None, time)
        .unwrap();
    assert!(reply.is_some());
    assert_eq!(incoming(&rig, BOB, &c), ["stamp check"]);
    // A node without chain flags takes a stamped delivery without checking
    // payment, even of a book it does not know.
    rig.nodes[BOB].chain = mailbox_chain::Lane::default();
    let fresh = tempfile::TempDir::new().unwrap();
    rig.nodes[BOB].mailbox_holder = mailbox_holder::Service::open(
        &fresh.path().join("profile.db"),
        &[31; 32],
        &identity::Keypair::generate_ed25519(),
        NETWORK_DOMAIN,
    )
    .unwrap();
    let next = rig.nodes[ALICE]
        .core
        .send_message(&c, "in the free network", "m2", time)
        .unwrap();
    let free = rig.nodes[ALICE]
        .core
        .prepare_swarm_delivery(&next.id, time)
        .unwrap();
    rig.nodes[BOB]
        .receive(&delivery(StampedDelivery::from(&free)), alice, None, time)
        .unwrap();
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["stamp check", "in the free network"]
    );
    // A free network checks nothing, so it lowers no trust either.
    assert_eq!(low_trust(&rig, BOB, &c), [false, false]);
}

/// A slot pays for one message: a second, different delivery on a ticket the
/// recipient already took is refused and proves the book spent twice. The
/// same stamp again (a retry, or the swarm copy) is no conflict.
#[tokio::test(flavor = "current_thread")]
async fn a_slot_paid_for_one_direct_message_pays_for_no_other() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup::default()).await;
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x68; 32]).unwrap();
    let book = [0x69; 32];
    let knowing: Vec<usize> = (0..HOLDERS).chain([BOB]).collect();
    test_book(&mut rig, &knowing, book, &key, 10);
    let time = rig.clock.wall();
    let mailbox = rig.nodes[BOB].core.swarm_mailbox(&c, true, time).unwrap();
    let record = rig.nodes[ALICE].binding(time).unwrap();
    let alice = *rig.nodes[ALICE].swarm.local_peer_id();
    let delivery = |envelope: &[u8]| Delivery {
        node_record: record.clone(),
        envelope: vec![],
        stamped: Some(StampedDelivery {
            conversation: c.clone(),
            mailbox: mailbox.to_vec(),
            period: period(WALL),
            envelope: envelope.to_vec(),
            stamp: mailbox_holder::StampWire::from(&paid(book, 0, &mailbox, envelope, &key)),
            grant: None,
        }),
    };
    // Whatever the envelope holds, its stamp took slot 0, and Bob put it on
    // record with the ticket's notaries: a reuse at another recipient is
    // proven there.
    let _ = rig.nodes[BOB].receive(&delivery(b"first"), alice, None, time);
    let ticket = agentic_mailbox_swarm::stamp::ticket_id(&NETWORK_DOMAIN, &book, 0);
    let noted = |r: &Rig| {
        (0..HOLDERS).any(|h| {
            r.nodes[h]
                .mailbox_holder
                .notary_record(&ticket)
                .unwrap()
                .is_some()
        })
    };
    rig.run_until(STEPS, noted).await;
    assert!(noted(&rig), "{:?}", rig.trace);
    let again = rig.nodes[BOB].receive(&delivery(b"first"), alice, None, time);
    assert!(
        !again
            .as_ref()
            .is_err_and(|error| error.to_string().contains("conflict")),
        "the same stamp again is no conflict"
    );
    let second = rig.nodes[BOB].receive(&delivery(b"second"), alice, None, time);
    assert!(
        second
            .as_ref()
            .is_err_and(|error| error.to_string().contains("conflict")),
        "{:?}",
        second.map(|_| ())
    );
    assert_eq!(
        rig.nodes[BOB].mailbox_holder.equivocations().unwrap().len(),
        1
    );
    assert!(
        rig.nodes[BOB]
            .mailbox_holder
            .blocked_books()
            .unwrap()
            .contains(&book)
    );
}

// --- contact by ID ---------------------------------------------------------------------

fn network_id_of(rig: &Rig, node: usize) -> String {
    rig.nodes[node]
        .core
        .snapshot()
        .unwrap()
        .identity
        .unwrap()
        .network_id
}

/// Holders keeping at least `count` entries of `mailbox`.
fn keeping(rig: &Rig, mailbox: &[u8; 32], count: usize) -> usize {
    (0..HOLDERS)
        .filter(|h| entries(rig, *h, mailbox).len() >= count)
        .count()
}

fn contacts(rig: &Rig, node: usize) -> Vec<(String, String)> {
    rig.nodes[node]
        .core
        .snapshot()
        .unwrap()
        .conversations
        .into_iter()
        .map(|c| (c.id, c.title))
        .collect()
}

fn used(rig: &Rig, node: usize) -> u32 {
    rig.nodes[node]
        .core
        .mailbox_books()
        .unwrap()
        .iter()
        .map(|b| b.used)
        .sum()
}

/// How a node took requests: `path` is `swarm` or `direct`, `outcome` is
/// `joined`, `pending` or `ignored`.
fn taken(rig: &Rig, node: usize, path: &str, outcome: &str) -> u64 {
    rig.nodes[node].mailbox_client.info()["intro"][path][outcome]
        .as_u64()
        .unwrap_or(0)
}

/// The name Alice gives Bob: not the one on his card.
const LOCAL_NAME: &str = "Bobby";

/// `request_contact` until the card lookup ends: the node's answer.
async fn ask(rig: &mut Rig, node: usize, id: &str, operation: &str) -> Value {
    let mut answer = Value::Null;
    for _ in 0..400 {
        answer = rig.nodes[node].command(
            "request_contact",
            json!({"networkId": id, "name": LOCAL_NAME, "operationId": operation}),
        );
        if answer["error"]["code"] != "card_pending" {
            return answer;
        }
        rig.step().await;
        if !rig.in_flight() {
            idle(rig, 1).await;
        }
    }
    panic!("the card lookup never ended: {answer}");
}

/// Store `envelope` in `mailbox` at every listed holder, paid by `payer`.
fn store_everywhere(rig: &mut Rig, payer: usize, mailbox: [u8; 32], envelope: Vec<u8>) {
    let time = rig.clock.wall();
    let stamp = rig.nodes[payer]
        .core
        .stamp_mailbox(&mailbox, period(time), &envelope, time)
        .unwrap();
    let d = SwarmDelivery {
        message_id: String::new(),
        conversation_id: String::new(),
        period: period(time),
        mailbox,
        envelope,
        stamp,
    };
    for holder in 0..HOLDERS {
        store_at(rig, holder, &d);
    }
}

/// Bob publishes his card once a period; Alice, who never met him, finds it
/// two days later among entries others paid for, and asks by his id while
/// he is away; her paid request waits in his intro swarm until he is back
/// days later and lets her in; then they talk.
#[tokio::test(flavor = "current_thread")]
async fn a_stranger_asks_by_id_while_the_recipient_is_away_and_they_talk() {
    let Chat { mut rig, .. } = chat(Setup {
        strangers: true,
        ..Setup::default()
    })
    .await;
    let bob = network_id_of(&rig, BOB);
    let day = rig.clock.wall();
    let first = rig.nodes[BOB].core.intro_mailbox(&bob, day).unwrap();
    // Anyone who pays can write there: another profile's card, and noise.
    let alices_card = rig.nodes[ALICE].core.intro_card(vec![], day).unwrap();
    store_everywhere(&mut rig, ALICE, first, alices_card.envelope);
    let mut noise = vec![2];
    noise.extend([0x5a; 120]);
    store_everywhere(&mut rig, ALICE, first, noise);
    rig.run_until(STEPS, |r| keeping(r, &first, 3) >= QUORUM)
        .await;
    assert!(keeping(&rig, &first, 3) >= QUORUM, "{:?}", rig.trace);
    // Once a period, with one stamp.
    idle(&mut rig, 600).await;
    assert_eq!(keeping(&rig, &first, 4), 0);
    assert_eq!(used(&rig, BOB), 1);

    // Bob is away; two days later Alice finds the card of the day he left.
    rig.paused.insert(BOB);
    rig.clock.advance(Duration::from_secs(2 * PERIOD_SECONDS));
    let asked = ask(&mut rig, ALICE, &bob, "ask-1").await;
    let c = asked["result"]["conversationId"]
        .as_str()
        .unwrap_or_else(|| panic!("{asked}"))
        .to_owned();
    assert_eq!(asked["result"]["name"], LOCAL_NAME);
    let again = rig.nodes[ALICE].command(
        "request_contact",
        json!({"networkId": bob, "name": LOCAL_NAME, "operationId": "ask-1"}),
    );
    assert_eq!(again["result"]["conversationId"], c.as_str());
    let asked_on = rig.nodes[BOB]
        .core
        .intro_mailbox(&bob, rig.clock.wall())
        .unwrap();
    rig.run_until(STEPS, |r| {
        r.nodes[ALICE].core.swarm_outbox(64).unwrap().is_empty()
    })
    .await;
    assert!(keeping(&rig, &asked_on, 1) >= QUORUM, "{:?}", rig.trace);
    assert!(contacts(&rig, BOB).is_empty());

    // Two more days on, Bob is back: he reads the day she asked, now
    // closed, lets her in and publishes today's card.
    rig.clock.advance(Duration::from_secs(2 * PERIOD_SECONDS));
    rig.paused.remove(&BOB);
    let today = rig.nodes[BOB]
        .core
        .intro_mailbox(&bob, rig.clock.wall())
        .unwrap();
    rig.run_until(STEPS, |r| {
        contacts(r, BOB) == [(c.clone(), "Alice".to_owned())] && keeping(r, &today, 1) >= QUORUM
    })
    .await;
    assert_eq!(
        contacts(&rig, BOB),
        [(c.clone(), "Alice".to_owned())],
        "{:?}",
        rig.trace
    );
    assert!(keeping(&rig, &today, 1) >= QUORUM, "{:?}", rig.trace);
    assert_eq!(taken(&rig, BOB, "swarm", "joined"), 1);
    // They talk through the swarm.
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&c, "hi by ID", "m1", time)
        .unwrap();
    rig.run_until(STEPS, |r| incoming(r, BOB, &c) == ["hi by ID"])
        .await;
    assert_eq!(incoming(&rig, BOB, &c), ["hi by ID"], "{:?}", rig.trace);
    rig.nodes[BOB]
        .core
        .send_message(&c, "you too", "m2", time)
        .unwrap();
    rig.run_until(STEPS, |r| incoming(r, ALICE, &c) == ["you too"])
        .await;
    assert_eq!(incoming(&rig, ALICE, &c), ["you too"], "{:?}", rig.trace);
}

/// Bob's card carries the addresses his node advertises, and Alice's node
/// sends her request there itself, paid with its swarm copy's stamp: with no
/// quorum possible, Bob takes it directly and his receipt ends her delivery.
#[tokio::test(flavor = "current_thread")]
async fn a_request_goes_directly_to_the_cards_addresses_paid_by_its_swarm_copy() {
    let Chat { mut rig, .. } = chat(Setup {
        strangers: true,
        direct: true,
        books_on_chain: true,
        // Six holders: a card is found, but no quorum can form.
        offline: vec![0, 1, 2, 3],
        ..Setup::default()
    })
    .await;
    let bob = network_id_of(&rig, BOB);
    let mailbox = rig.nodes[BOB]
        .core
        .intro_mailbox(&bob, rig.clock.wall())
        .unwrap();
    rig.run_until(STEPS, |r| keeping(r, &mailbox, 1) >= HOLDERS - 4)
        .await;
    let entry = entries(&rig, 4, &mailbox).remove(0).envelope;
    let card = rig.nodes[ALICE]
        .core
        .open_intro_card(&bob, &entry, rig.clock.wall())
        .unwrap();
    let advertised: Vec<String> = rig.nodes[BOB].listeners.iter().cloned().collect();
    assert!(!advertised.is_empty());
    assert_eq!(card.addresses, advertised);
    // Alice's node publishes her own card too; count her slots after it.
    let alices = rig.nodes[ALICE]
        .core
        .own_intro_mailbox(rig.clock.wall())
        .unwrap();
    rig.run_until(STEPS, |r| keeping(r, &alices, 1) >= HOLDERS - 4)
        .await;
    let before = used(&rig, ALICE);
    let asked = ask(&mut rig, ALICE, &bob, "ask-1").await;
    let c = asked["result"]["conversationId"]
        .as_str()
        .unwrap_or_else(|| panic!("{asked}"))
        .to_owned();
    rig.run_until(STEPS, |r| {
        r.nodes[ALICE].core.outbox(100).unwrap().is_empty()
    })
    .await;
    assert!(
        rig.nodes[ALICE].core.outbox(100).unwrap().is_empty(),
        "{:?}",
        rig.trace
    );
    assert_eq!(contacts(&rig, BOB), [(c, "Alice".to_owned())]);
    // Joined once, by whichever copy came first; the direct one was taken
    // (its receipt is what ended Alice's delivery without a quorum).
    assert_eq!(
        taken(&rig, BOB, "direct", "joined") + taken(&rig, BOB, "swarm", "joined"),
        1
    );
    assert!(taken(&rig, BOB, "direct", "joined") + taken(&rig, BOB, "direct", "ignored") >= 1);
    assert_eq!(rig.chain.book_reads(BOB, &alice_book()), 1);
    // One slot for both copies of her request.
    assert_eq!(used(&rig, ALICE), before + 1);
}

/// Directly, a node that reads the chain takes a request only paid, for its
/// own intro mailbox, with the stamp over those bytes, from the requester's
/// own node; the same request again is a receipt and no second contact.
#[tokio::test(flavor = "current_thread")]
async fn a_paying_node_takes_a_direct_request_only_paid_and_once() {
    let Chat { mut rig, .. } = chat(Setup {
        strangers: true,
        books_on_chain: true,
        // Only the direct path reaches Bob.
        bob_offline: true,
        // Alice has no book yet.
        alice_granted: true,
        // Carol's node delivers what Alice wrote.
        carol: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let bob = network_id_of(&rig, BOB);
    let card = rig.nodes[BOB].core.intro_card(vec![], time).unwrap();
    let c = rig.nodes[ALICE]
        .core
        .request_contact("Bob", &bob, &card.envelope, "ask-1", time)
        .unwrap()
        .id;
    let record = rig.nodes[ALICE].binding(time).unwrap();
    let alice = *rig.nodes[ALICE].swarm.local_peer_id();
    let delivery = |envelope: Vec<u8>, stamped: Option<StampedDelivery>| Delivery {
        node_record: record.clone(),
        envelope,
        stamped,
    };
    let wire = rig.nodes[ALICE].core.outbox(100).unwrap().remove(0).wire;
    let unpaid = rig.nodes[BOB]
        .receive(&delivery(wire, None), alice, None, time)
        .unwrap_err();
    assert!(unpaid.to_string().contains("needs a stamp"), "{unpaid}");
    let before = contacts(&rig, BOB).len();

    let key = rig.nodes[ALICE].core.mailbox_book_account().unwrap();
    let book = [0xb7; 32];
    let valid_until = time + 30 * PERIOD_SECONDS;
    rig.nodes[ALICE]
        .core
        .add_mailbox_book(book, 100, valid_until)
        .unwrap();
    rig.chain.buy(
        book,
        crate::runtime::chain::BookRecord {
            key,
            count: 100,
            valid_until,
        },
    );
    let id = rig.nodes[ALICE].core.swarm_outbox(64).unwrap()[0]
        .message_id
        .clone();
    let d = rig.nodes[ALICE]
        .core
        .prepare_swarm_delivery(&id, time)
        .unwrap();
    assert_eq!(
        d.mailbox,
        rig.nodes[BOB].core.intro_mailbox(&bob, time).unwrap()
    );
    // Now with a book, Alice's node also publishes her own card.
    let alices = rig.nodes[ALICE].core.own_intro_mailbox(time).unwrap();
    rig.run_until(STEPS, |r| keeping(r, &alices, 1) >= QUORUM)
        .await;
    let spent = used(&rig, ALICE);
    // Her request and her card.
    assert_eq!(spent, 2);
    let good = StampedDelivery::from(&d);
    let mut elsewhere = good.clone();
    elsewhere.mailbox = rig.nodes[BOB]
        .core
        .intro_mailbox(&network_id_of(&rig, ALICE), time)
        .unwrap()
        .to_vec();
    let mut altered = good.clone();
    altered.envelope[70] ^= 1;
    for (label, bad) in [("another mailbox", elsewhere), ("altered", altered)] {
        assert!(
            rig.nodes[BOB]
                .receive(&delivery(vec![], Some(bad)), alice, None, time)
                .is_err(),
            "{label}"
        );
    }
    // Delivered by another profile's node, it is not taken either.
    let carol_record = rig.nodes[CAROL].binding(time).unwrap();
    let carol_peer = *rig.nodes[CAROL].swarm.local_peer_id();
    let relayed = Delivery {
        node_record: carol_record,
        envelope: vec![],
        stamped: Some(good.clone()),
    };
    // Bob's node reads Alice's book on the chain, then takes the request.
    let mut taken_reply = None;
    for _ in 0..50 {
        assert!(
            rig.nodes[BOB]
                .receive(&relayed, carol_peer, None, time)
                .is_err()
        );
        assert_eq!(contacts(&rig, BOB).len(), before);
        match rig.nodes[BOB].receive(&delivery(vec![], Some(good.clone())), alice, None, time) {
            Ok(reply) => {
                taken_reply = Some(reply);
                break;
            }
            Err(_) => {
                rig.step().await;
            }
        }
    }
    let receipt = taken_reply
        .expect("the paid request is taken")
        .expect("a receipt");
    assert!(contacts(&rig, BOB).contains(&(c.clone(), "Alice".to_owned())));
    assert_eq!(contacts(&rig, BOB).len(), before + 1);
    assert_eq!(rig.chain.book_reads(BOB, &book), 1);
    rig.nodes[ALICE].core.receive(&receipt, time).unwrap();
    assert!(rig.nodes[ALICE].core.outbox(100).unwrap().is_empty());
    // Again: a receipt, one contact, one slot.
    let again = rig.nodes[BOB]
        .receive(&delivery(vec![], Some(good)), alice, None, time)
        .unwrap();
    assert!(again.is_some());
    assert_eq!(contacts(&rig, BOB).len(), before + 1);
    // The direct copy took no slot beyond its swarm copy's.
    assert_eq!(used(&rig, ALICE), spent);
}

/// An id without a published card ends the request, even with holders that
/// never answer: nothing is created. A malformed or one's own id is refused
/// at once.
#[tokio::test(flavor = "current_thread")]
async fn a_request_to_an_id_without_a_card_ends_without_a_conversation() {
    let Chat { mut rig, .. } = chat(Setup {
        strangers: true,
        // Bob has no book, so he publishes no card.
        bob_without_book: true,
        offline: vec![0, 1, 2],
        ..Setup::default()
    })
    .await;
    let bob = network_id_of(&rig, BOB);
    let answer = ask(&mut rig, ALICE, &bob, "ask-1").await;
    assert_eq!(answer["error"]["code"], "card_not_found", "{answer}");
    assert!(contacts(&rig, ALICE).is_empty());
    assert!(rig.nodes[ALICE].core.outbox(100).unwrap().is_empty());
    let alice = network_id_of(&rig, ALICE);
    for id in ["ain1xyz", alice.as_str()] {
        let refused = rig.nodes[ALICE].command(
            "request_contact",
            json!({"networkId": id, "name": LOCAL_NAME, "operationId": "ask-2"}),
        );
        assert_eq!(refused["error"]["code"], "invalid_request", "{refused}");
    }
}

/// The owner decides on requests over the owner IPC the CLI uses: a manual
/// policy holds requests, listed as the CLI prints them; one accepted joins
/// and they talk, one rejected is gone.
#[tokio::test(flavor = "current_thread")]
async fn the_owner_decides_on_held_requests_over_ipc() {
    let Chat { mut rig, .. } = chat(Setup {
        strangers: true,
        carol: true,
        ..Setup::default()
    })
    .await;
    let bob = network_id_of(&rig, BOB);
    let set = rig.nodes[BOB].command(
        "set_intro_policy",
        json!({"mode": "manual", "dailyLimit": 20, "allowed": []}),
    );
    assert_eq!(
        set["result"],
        json!({"mode": "manual", "dailyLimit": 20, "allowed": []}),
        "{set}"
    );
    assert_eq!(
        rig.nodes[BOB].command("intro_policy", json!({}))["result"]["mode"],
        "manual"
    );
    let today = rig.nodes[BOB]
        .core
        .intro_mailbox(&bob, rig.clock.wall())
        .unwrap();
    rig.run_until(STEPS, |r| keeping(r, &today, 1) >= QUORUM)
        .await;
    let requests =
        |r: &mut Rig| r.nodes[BOB].command("intro_requests", json!({}))["result"].clone();
    let asked = ask(&mut rig, ALICE, &bob, "ask-1").await;
    let c = asked["result"]["conversationId"]
        .as_str()
        .unwrap_or_else(|| panic!("{asked}"))
        .to_owned();
    let mut waiting = Value::Null;
    for _ in 0..100 {
        waiting = requests(&mut rig);
        if waiting.as_array().is_some_and(|w| !w.is_empty()) {
            break;
        }
        idle(&mut rig, 5).await;
    }
    let alice = network_id_of(&rig, ALICE);
    let entry = &waiting[0];
    assert_eq!(waiting.as_array().unwrap().len(), 1, "{waiting}");
    assert_eq!(
        (
            entry["networkId"].as_str(),
            entry["name"].as_str(),
            entry["receivedAt"].is_u64(),
        ),
        (Some(alice.as_str()), Some("Alice"), true),
        "{waiting}"
    );
    let request = entry["requestId"].as_str().unwrap().to_owned();
    assert_eq!(
        entry.as_object().unwrap().len(),
        4,
        "exactly requestId, networkId, name, receivedAt: {waiting}"
    );
    assert!(contacts(&rig, BOB).is_empty());
    let accepted = rig.nodes[BOB].command("accept_intro_request", json!({"requestId": request}));
    assert_eq!(
        accepted["result"],
        json!({"conversationId": c, "name": "Alice"}),
        "{accepted}"
    );
    assert_eq!(requests(&mut rig), json!([]));
    assert_eq!(contacts(&rig, BOB), [(c.clone(), "Alice".to_owned())]);
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&c, "accepted manually", "m1", time)
        .unwrap();
    rig.run_until(STEPS, |r| incoming(r, BOB, &c) == ["accepted manually"])
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["accepted manually"],
        "{:?}",
        rig.trace
    );

    // Carol's request is rejected: gone, and read again it stays gone.
    ask(&mut rig, CAROL, &bob, "ask-2").await;
    let mut waiting = Value::Null;
    for _ in 0..100 {
        waiting = requests(&mut rig);
        if waiting.as_array().is_some_and(|w| !w.is_empty()) {
            break;
        }
        idle(&mut rig, 5).await;
    }
    let carols = waiting[0]["requestId"].as_str().unwrap().to_owned();
    assert_eq!(waiting[0]["networkId"], network_id_of(&rig, CAROL));
    let rejected = rig.nodes[BOB].command("reject_intro_request", json!({"requestId": carols}));
    assert!(rejected.get("result").is_some(), "{rejected}");
    assert_eq!(requests(&mut rig), json!([]));
    let again = rig.nodes[BOB].command("accept_intro_request", json!({"requestId": carols}));
    assert_eq!(again["error"]["code"], "unknown_request", "{again}");
    idle(&mut rig, 120).await;
    assert_eq!(requests(&mut rig), json!([]));
    assert_eq!(contacts(&rig, BOB).len(), 1);
}

// --- groups ------------------------------------------------------------------------------

/// A commit wins a round only when seven of its notaries recorded it first;
/// when no commit can any more, the round is split.
#[test]
fn a_commit_wins_a_round_only_with_seven_first_records() {
    use crate::runtime::mailbox_groups::{RoundResult, round_result};
    let (a, b) = ([1u8; 32], [2u8; 32]);
    let firsts = |pairs: &[([u8; 32], usize)]| -> Vec<[u8; 32]> {
        pairs
            .iter()
            .flat_map(|(c, n)| std::iter::repeat_n(*c, *n))
            .collect()
    };
    assert_eq!(round_result(&firsts(&[(a, 7)]), 10), RoundResult::Winner(a));
    assert_eq!(
        round_result(&firsts(&[(a, 7), (b, 3)]), 10),
        RoundResult::Winner(a)
    );
    assert_eq!(
        round_result(&firsts(&[(a, 6), (b, 4)]), 10),
        RoundResult::Split
    );
    assert_eq!(
        round_result(&firsts(&[(a, 6), (b, 3)]), 10),
        RoundResult::Open
    );
    assert_eq!(
        round_result(&firsts(&[(a, 5), (b, 3)]), 10),
        RoundResult::Open
    );
    assert_eq!(
        round_result(&firsts(&[(a, 4), (b, 4)]), 10),
        RoundResult::Split
    );
    assert_eq!(round_result(&firsts(&[]), 10), RoundResult::Open);
    assert_eq!(round_result(&firsts(&[(a, 7)]), 9), RoundResult::Winner(a));
    assert_eq!(
        round_result(&firsts(&[(a, 5), (b, 4)]), 9),
        RoundResult::Split
    );
    // Fewer notaries than a quorum decide nothing.
    assert_eq!(round_result(&firsts(&[(a, 6)]), 6), RoundResult::Open);
}

/// Answers an owner IPC call once the node has every card it needs.
async fn when_cards_are_read(rig: &mut Rig, node: usize, method: &str, request: Value) -> Value {
    let mut answer = Value::Null;
    for _ in 0..400 {
        answer = rig.nodes[node].command(method, request.clone());
        if answer["error"]["code"] != "card_pending" {
            return answer;
        }
        // Reads waiting for a holder to let this node in wait for time to
        // pass, like an idle node.
        let progressed = rig.step().await;
        if !rig.in_flight() || !progressed {
            idle(rig, 1).await;
        }
    }
    panic!("the cards were never read: {answer}");
}

fn epoch_of(rig: &Rig, node: usize, g: &str) -> Option<u64> {
    rig.nodes[node].core.group(g).ok().map(|i| i.epoch)
}

fn members_of(rig: &Rig, node: usize, g: &str) -> Vec<String> {
    let mut members = rig.nodes[node]
        .core
        .group(g)
        .map(|i| i.members)
        .unwrap_or_default();
    members.sort();
    members
}

/// Alice makes a group, by their ids, of everyone else `setup` names; they
/// join through their intro mailboxes.
async fn made_group(setup: Setup) -> (Rig, String, Vec<usize>) {
    let dave = setup.dave;
    let Chat { mut rig, .. } = chat(Setup {
        strangers: true,
        carol: true,
        ..setup
    })
    .await;
    let others: Vec<usize> = if dave && !setup.dave_outside {
        vec![BOB, CAROL, DAVE]
    } else {
        vec![BOB, CAROL]
    };
    for node in others.iter().copied() {
        let mailbox = rig.nodes[node]
            .core
            .own_intro_mailbox(rig.clock.wall())
            .unwrap();
        rig.run_until(STEPS, |r| keeping(r, &mailbox, 1) >= QUORUM)
            .await;
    }
    let ids: Vec<String> = others.iter().map(|n| network_id_of(&rig, *n)).collect();
    let request = json!({"name": "Team", "members": ids, "operationId": "g-1"});
    let made = when_cards_are_read(&mut rig, ALICE, "create_group", request.clone()).await;
    let g = made["result"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned();
    assert_eq!(made["result"]["role"], "owner");
    // Safe to retry.
    assert_eq!(
        rig.nodes[ALICE].command("create_group", request)["result"]["id"],
        g.as_str()
    );
    rig.run_until(STEPS, |r| {
        others.iter().all(|n| epoch_of(r, *n, &g).is_some())
    })
    .await;
    let everyone = members_of(&rig, ALICE, &g);
    assert_eq!(everyone.len(), others.len() + 1, "{:?}", rig.trace);
    for n in &others {
        assert_eq!(members_of(&rig, *n, &g), everyone);
        assert_eq!(rig.nodes[*n].core.group(&g).unwrap().role, "member");
    }
    (rig, g, others)
}

/// A group made of ids reaches its members through their intro mailboxes,
/// and every message through the group's one mailbox.
#[tokio::test(flavor = "current_thread")]
async fn a_group_made_by_ids_is_read_by_every_member_through_the_swarm() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&g, "hello everyone", "m1", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        [BOB, CAROL]
            .iter()
            .all(|n| incoming(r, *n, &g) == ["hello everyone"])
    })
    .await;
    for n in [BOB, CAROL] {
        assert_eq!(incoming(&rig, n, &g), ["hello everyone"], "{:?}", rig.trace);
    }
    rig.nodes[CAROL]
        .core
        .send_message(&g, "hello from Carol", "m2", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        incoming(r, ALICE, &g) == ["hello from Carol"] && incoming(r, BOB, &g).len() == 2
    })
    .await;
    assert_eq!(
        incoming(&rig, ALICE, &g),
        ["hello from Carol"],
        "{:?}",
        rig.trace
    );
    let listed = rig.nodes[BOB].command("groups", json!({}));
    assert!(
        listed["result"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == g.as_str() && i["role"] == "member"),
        "{listed}"
    );
    let one = rig.nodes[BOB].command("group", json!({ "groupId": g }));
    assert_eq!(one["result"]["name"], "Team");
    let unknown = rig.nodes[BOB].command("group", json!({ "groupId": "ab".repeat(32) }));
    assert_eq!(unknown["error"]["code"], "unknown_group", "{unknown}");
    let malformed = rig.nodes[ALICE].command(
        "create_group",
        json!({"name": "X", "members": ["ain1xyz"], "operationId": "g-2"}),
    );
    assert_eq!(malformed["error"]["code"], "invalid_request", "{malformed}");
    // An id without a card: no group.
    let nobody = format!("ain1{}", "cd".repeat(32));
    let missing = when_cards_are_read(
        &mut rig,
        ALICE,
        "create_group",
        json!({"name": "X", "members": [nobody], "operationId": "g-3"}),
    )
    .await;
    assert_eq!(missing["error"]["code"], "card_not_found", "{missing}");
    assert_eq!(rig.nodes[ALICE].core.groups().unwrap().len(), 1);
}

/// A member's burst reads in the order it was written at every other
/// member, whatever order the holders stored it in.
#[tokio::test(flavor = "current_thread")]
async fn a_members_burst_reads_in_the_order_it_was_written() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    let counts = |rig: &Rig, n: usize| {
        let info = rig.nodes[n].mailbox_client.info();
        (
            info["importAttempts"].as_u64().unwrap(),
            info["received"].as_u64().unwrap(),
        )
    };
    let before = [counts(&rig, ALICE), counts(&rig, BOB)];
    let time = rig.clock.wall();
    let burst: Vec<String> = (1..=6).map(|i| format!("message {i}")).collect();
    for (i, text) in burst.iter().enumerate() {
        rig.nodes[CAROL]
            .core
            .send_message(&g, text, &format!("b-{i}"), time)
            .unwrap();
    }
    rig.run_until(STEPS, |r| {
        [ALICE, BOB]
            .iter()
            .all(|n| incoming(r, *n, &g).len() == burst.len())
    })
    .await;
    for n in [ALICE, BOB] {
        assert_eq!(incoming(&rig, n, &g), burst, "{:?}", rig.trace);
    }
    // The holders did answer out of order: some reader held an envelope.
    assert!(
        [ALICE, BOB]
            .iter()
            .zip(before)
            .any(|(n, (attempts, received))| {
                let (a, r) = counts(&rig, *n);
                a - attempts > r - received
            })
    );
}

/// Two members write at once and the holders take their messages out of
/// order: each member's messages read in the order written, and a missing
/// message holds back only its own sender's later ones.
#[tokio::test(flavor = "current_thread")]
async fn a_gap_in_one_members_messages_holds_back_only_that_member() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    // Their nodes stand still: the holders get only what is placed below.
    rig.paused.extend([BOB, CAROL]);
    let time = rig.clock.wall();
    let mut written: BTreeMap<usize, Vec<(String, SwarmDelivery)>> = BTreeMap::new();
    for (node, name) in [(CAROL, "Carol"), (BOB, "Bob")] {
        let core = &mut rig.nodes[node].core;
        let messages = (1..=3)
            .map(|n| {
                let text = format!("{name}: item {n}");
                let id = core
                    .send_message(&g, &text, &format!("{name}-{n}"), time)
                    .unwrap()
                    .id;
                (text, core.prepare_swarm_delivery(&id, time).unwrap())
            })
            .collect();
        written.insert(node, messages);
    }
    // The member Alice's node would try first misses its first message.
    let sender = |node: usize| {
        let (_, d) = &written[&node][0];
        rig.nodes[ALICE]
            .core
            .swarm_envelope_order(&g, d.period, &d.envelope, time)
            .unwrap()
            .sender
    };
    let (late, other) = if sender(CAROL) < sender(BOB) {
        (CAROL, BOB)
    } else {
        (BOB, CAROL)
    };
    let written_texts =
        |node: usize| -> Vec<String> { written[&node].iter().map(|(t, _)| t.clone()).collect() };
    for (node, index) in [(late, 1), (late, 2), (other, 1), (other, 2), (other, 0)] {
        for holder in 0..HOLDERS {
            store_at(&mut rig, holder, &written[&node][index].1);
        }
    }
    rig.run_until(STEPS, |r| incoming(r, ALICE, &g).len() >= 3)
        .await;
    idle(&mut rig, 60).await;
    assert_eq!(
        incoming(&rig, ALICE, &g),
        written_texts(other),
        "{:?}",
        rig.trace
    );
    for holder in 0..HOLDERS {
        store_at(&mut rig, holder, &written[&late][0].1);
    }
    rig.run_until(STEPS, |r| incoming(r, ALICE, &g).len() == 6)
        .await;
    let read = incoming(&rig, ALICE, &g);
    assert_eq!(read[..3], written_texts(other), "{:?}", rig.trace);
    assert_eq!(read[3..], written_texts(late), "{:?}", rig.trace);
}

/// A removal goes through the swarm and the notary: every member moves to
/// the next epoch, and the removed one reads nothing written after it.
#[tokio::test(flavor = "current_thread")]
async fn a_removal_goes_through_the_notary_and_the_removed_member_reads_nothing_new() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    let epoch = epoch_of(&rig, ALICE, &g).unwrap();
    let (alice, bob, carol) = (
        network_id_of(&rig, ALICE),
        network_id_of(&rig, BOB),
        network_id_of(&rig, CAROL),
    );
    // A plain member changes nothing.
    let denied = rig.nodes[BOB].command(
        "change_group",
        json!({"groupId": g, "add": [], "remove": [carol], "operationId": "x-1"}),
    );
    assert_eq!(denied["error"]["code"], "not_allowed", "{denied}");
    // Four holders are away: the commit is not stored at a quorum, so it is
    // not put on record, and nothing is decided until they are back.
    for h in 0..4 {
        rig.paused.insert(h);
    }
    let removal = json!({"groupId": g, "add": [], "remove": [carol], "operationId": "c-1"});
    let changed = rig.nodes[ALICE].command("change_group", removal.clone());
    assert_eq!(changed["result"]["epoch"], epoch, "{changed}");
    let commit = changed["result"]["commit"].as_str().unwrap().to_owned();
    assert!(changed["result"]["messageId"].is_string(), "{changed}");
    // The same operation is the same commit; another one waits its turn.
    assert_eq!(
        rig.nodes[ALICE].command("change_group", removal)["result"]["commit"],
        commit.as_str()
    );
    let busy = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "add": [], "remove": [bob], "operationId": "c-2"}),
    );
    assert_eq!(busy["error"]["code"], "group_busy", "{busy}");
    let claims = rig.nodes[ALICE].core.group_claims(&g).unwrap();
    let key =
        agentic_protocol::group::verify_claim(&claims[0].claim, NETWORK_DOMAIN, rig.clock.wall())
            .unwrap()
            .key;
    let mailbox = rig.nodes[ALICE]
        .core
        .group_mailboxes(&g, rig.clock.wall())
        .unwrap()[0];
    // Whenever any notary has the claim, the commit is stored at a quorum.
    let recorded = |r: &Rig| {
        (0..HOLDERS).any(|h| {
            r.nodes[h]
                .mailbox_holder
                .notary_record(&key)
                .ok()
                .flatten()
                .is_some()
        })
    };
    let violated = std::cell::Cell::new(false);
    let watch = |r: &Rig| {
        if recorded(r) && keeping(r, &mailbox, 1) < QUORUM {
            violated.set(true);
        }
    };
    idle(&mut rig, 60).await;
    assert_eq!(epoch_of(&rig, ALICE, &g), Some(epoch));
    assert!(!recorded(&rig));
    for h in 0..4 {
        rig.paused.remove(&h);
    }
    rig.run_until(STEPS, |r| {
        watch(r);
        [ALICE, BOB]
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(epoch + 1))
            && members_of(r, CAROL, &g).is_empty()
    })
    .await;
    assert!(
        !violated.get(),
        "a claim was on record before its commit was stored"
    );
    // Carol keeps the group, without members, and reads nothing new.
    assert!(rig.nodes[CAROL].core.group(&g).is_ok());
    let mut both = vec![alice, bob];
    both.sort();
    assert_eq!(members_of(&rig, ALICE, &g), both, "{:?}", rig.trace);
    assert_eq!(members_of(&rig, BOB, &g), both);
    assert!(members_of(&rig, CAROL, &g).is_empty());
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&g, "without Carol", "m3", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        incoming(r, BOB, &g).last().map(String::as_str) == Some("without Carol")
    })
    .await;
    assert_eq!(
        incoming(&rig, BOB, &g).last().map(String::as_str),
        Some("without Carol")
    );
    idle(&mut rig, 60).await;
    assert!(
        !incoming(&rig, CAROL, &g)
            .iter()
            .any(|t| t == "without Carol")
    );
}

/// The owner bans a member over the owner IPC: every member sees it gone
/// and banned, and it is not added back until the owner lifts the ban.
#[tokio::test(flavor = "current_thread")]
async fn a_banned_member_is_not_added_back_until_the_owner_unbans_it() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    let epoch = epoch_of(&rig, ALICE, &g).unwrap();
    let carol = network_id_of(&rig, CAROL);
    let banned = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "ban": [carol], "operationId": "b-1"}),
    );
    assert_eq!(banned["result"]["epoch"], epoch, "{banned}");
    rig.run_until(STEPS, |r| {
        [ALICE, BOB]
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(epoch + 1))
            && members_of(r, CAROL, &g).is_empty()
    })
    .await;
    for n in [ALICE, BOB] {
        let shown = rig.nodes[n].command("group", json!({ "groupId": g }));
        assert_eq!(
            shown["result"]["banned"],
            json!([{"id": carol, "byOwner": true}]),
            "{shown}"
        );
        assert!(!members_of(&rig, n, &g).contains(&carol), "{:?}", rig.trace);
    }
    let back = when_cards_are_read(
        &mut rig,
        ALICE,
        "change_group",
        json!({"groupId": g, "add": [carol], "operationId": "b-2"}),
    )
    .await;
    assert_eq!(back["error"]["code"], "banned", "{back}");
    let lifted = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "unban": [carol], "operationId": "b-3"}),
    );
    assert_eq!(lifted["result"]["epoch"], epoch + 1, "{lifted}");
    rig.run_until(STEPS, |r| {
        [ALICE, BOB]
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(epoch + 2))
    })
    .await;
    for n in [ALICE, BOB] {
        assert_eq!(
            rig.nodes[n].command("group", json!({ "groupId": g }))["result"]["banned"],
            json!([])
        );
    }
}

/// Two committers at one epoch: the notary names one, every member applies
/// that one, and the other change goes through on the next epoch when its
/// maker asks again.
#[tokio::test(flavor = "current_thread")]
async fn two_committers_at_one_epoch_agree_on_one_commit_and_the_other_follows() {
    let (mut rig, g, _) = made_group(Setup {
        dave: true,
        ..Setup::default()
    })
    .await;
    let (alice, bob, carol, dave) = (
        network_id_of(&rig, ALICE),
        network_id_of(&rig, BOB),
        network_id_of(&rig, CAROL),
        network_id_of(&rig, DAVE),
    );
    let e0 = epoch_of(&rig, ALICE, &g).unwrap();
    let promoted = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "add": [], "remove": [], "admins": [bob], "operationId": "c-1"}),
    );
    assert!(promoted.get("result").is_some(), "{promoted}");
    rig.run_until(STEPS, |r| {
        [ALICE, BOB, CAROL, DAVE]
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(e0 + 1))
    })
    .await;
    assert_eq!(
        rig.nodes[BOB].core.group(&g).unwrap().role,
        "admin",
        "{:?}",
        rig.trace
    );
    // At once: the owner removes Carol, the admin removes Dave.
    let e1 = e0 + 1;
    let by_alice = json!({"groupId": g, "add": [], "remove": [carol], "operationId": "race-a"});
    let by_bob = json!({"groupId": g, "add": [], "remove": [dave], "operationId": "race-b"});
    assert_eq!(
        rig.nodes[ALICE].command("change_group", by_alice.clone())["result"]["epoch"],
        e1
    );
    assert_eq!(
        rig.nodes[BOB].command("change_group", by_bob.clone())["result"]["epoch"],
        e1
    );
    // Round 0 splits: six notaries saw Alice's claim first, four Bob's.
    let time = rig.clock.wall();
    let claim_of = |rig: &Rig, node: usize| {
        rig.nodes[node].core.group_claims(&g).unwrap()[0]
            .claim
            .clone()
    };
    let (alices, bobs) = (claim_of(&rig, ALICE), claim_of(&rig, BOB));
    for h in 0..HOLDERS {
        let first = if h < 6 { &alices } else { &bobs };
        rig.nodes[h]
            .mailbox_holder
            .notarize(&mailbox_holder::Statement::Commit(first.clone()), time)
            .unwrap();
    }
    let verified = agentic_protocol::group::verify_claim(&alices, NETWORK_DOMAIN, time).unwrap();
    let group = agentic_protocol::group::group_ref(
        &NETWORK_DOMAIN,
        &verified.claim.owner,
        &verified.claim.group_id,
    );
    let round_key = |round| agentic_protocol::group::claim_key(&group, e1, round);
    let epoch_mailbox = rig.nodes[ALICE].core.group_mailboxes(&g, time).unwrap()[0];
    rig.run_until(STEPS, |r| {
        [ALICE, BOB]
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(e1 + 1))
    })
    .await;
    // Round 0 had no winner; a later round decided it: seven or more
    // notaries hold one claim first, and a renewed claim was stored in the
    // epoch's mailbox beside both commits.
    let best = |rig: &Rig, round: u32| {
        let mut firsts: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
        for h in 0..HOLDERS {
            if let Ok(Some(noted)) = rig.nodes[h].mailbox_holder.notary_record(&round_key(round)) {
                let mailbox_holder::Statement::Commit(claim) = noted.first else {
                    panic!("a commit claim");
                };
                *firsts.entry(claim).or_default() += 1;
            }
        }
        firsts.into_values().max().unwrap_or(0)
    };
    assert!(best(&rig, 0) < QUORUM);
    assert!(
        (1..=8).any(|round| best(&rig, round) >= QUORUM),
        "{}",
        rig.nodes[ALICE].mailbox_client.info()["groups"]
    );
    assert!(keeping(&rig, &epoch_mailbox, 3) >= QUORUM);
    let after = members_of(&rig, ALICE, &g);
    assert_eq!(members_of(&rig, BOB, &g), after, "{:?}", rig.trace);
    assert_eq!(after.len(), 3, "exactly one removal: {after:?}");
    let carol_out = !after.contains(&carol);
    assert!(carol_out ^ !after.contains(&dave));
    // The one still in follows the same epoch and members.
    let still = if carol_out { DAVE } else { CAROL };
    rig.run_until(STEPS, |r| epoch_of(r, still, &g) == Some(e1 + 1))
        .await;
    assert_eq!(members_of(&rig, still, &g), after);
    // The other change, asked again, goes through on the next epoch.
    let (node, again) = if carol_out {
        (BOB, by_bob)
    } else {
        (ALICE, by_alice)
    };
    let answer = rig.nodes[node].command("change_group", again);
    assert_eq!(answer["result"]["epoch"], e1 + 1, "{answer}");
    rig.run_until(STEPS, |r| {
        [ALICE, BOB]
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(e1 + 2))
    })
    .await;
    let mut two = vec![alice, bob];
    two.sort();
    assert_eq!(members_of(&rig, ALICE, &g), two, "{:?}", rig.trace);
    assert_eq!(members_of(&rig, BOB, &g), two);
}

/// The committer is gone right after storing its commit: its readers put the
/// claim on record, the epoch is decided without it, and it follows when it
/// is back.
#[tokio::test(flavor = "current_thread")]
async fn readers_decide_an_epoch_whose_committer_went_away() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    let epoch = epoch_of(&rig, ALICE, &g).unwrap();
    let carol = network_id_of(&rig, CAROL);
    rig.paused.insert(ALICE);
    let time = rig.clock.wall();
    let made = rig.nodes[ALICE]
        .core
        .change_group(
            &g,
            agentic_core::GroupChange {
                remove: vec![carol],
                ..agentic_core::GroupChange::default()
            },
            "c-1",
            time,
        )
        .unwrap();
    let d = rig.nodes[ALICE]
        .core
        .prepare_swarm_delivery(&made.message_id, time)
        .unwrap();
    for holder in 0..HOLDERS {
        store_at(&mut rig, holder, &d);
    }
    rig.run_until(STEPS, |r| {
        epoch_of(r, BOB, &g) == Some(epoch + 1) && members_of(r, CAROL, &g).is_empty()
    })
    .await;
    assert_eq!(epoch_of(&rig, BOB, &g), Some(epoch + 1), "{:?}", rig.trace);
    assert_eq!(epoch_of(&rig, ALICE, &g), Some(epoch));
    rig.paused.remove(&ALICE);
    rig.run_until(STEPS, |r| epoch_of(r, ALICE, &g) == Some(epoch + 1))
        .await;
    assert_eq!(
        epoch_of(&rig, ALICE, &g),
        Some(epoch + 1),
        "{:?}",
        rig.trace
    );
    assert_eq!(members_of(&rig, ALICE, &g), members_of(&rig, BOB, &g));
}

/// A message written in an epoch reaches the members who moved on before it
/// went out: their nodes read the previous epoch's mailbox too.
#[tokio::test(flavor = "current_thread")]
async fn a_message_written_before_a_commit_is_read_after_it() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    let epoch = epoch_of(&rig, ALICE, &g).unwrap();
    rig.paused.insert(BOB);
    let time = rig.clock.wall();
    rig.nodes[BOB]
        .core
        .send_message(&g, "before the commit", "m-1", time)
        .unwrap();
    let carol = network_id_of(&rig, CAROL);
    let changed = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "add": [], "remove": [carol], "operationId": "c-1"}),
    );
    assert!(changed.get("result").is_some(), "{changed}");
    rig.run_until(STEPS, |r| epoch_of(r, ALICE, &g) == Some(epoch + 1))
        .await;
    assert_eq!(
        epoch_of(&rig, ALICE, &g),
        Some(epoch + 1),
        "{:?}",
        rig.trace
    );
    rig.paused.remove(&BOB);
    rig.run_until(STEPS, |r| {
        incoming(r, ALICE, &g)
            .iter()
            .any(|t| t == "before the commit")
            && epoch_of(r, BOB, &g) == Some(epoch + 1)
    })
    .await;
    assert!(
        incoming(&rig, ALICE, &g)
            .iter()
            .any(|t| t == "before the commit"),
        "{:?}",
        rig.trace
    );
    assert_eq!(epoch_of(&rig, BOB, &g), Some(epoch + 1));
}

// --- access by book (Docs/V1_DISCOVERY_2026_09_27.md, part 1) ------------------

use crate::runtime::mailbox_holder::{AccessPassWire, CredentialWire, StatementWire};

fn peer_of_node(rig: &Rig, node: usize) -> PeerId {
    *rig.nodes[node].swarm.local_peer_id()
}

/// The pass `node` shows today, as its core signs it.
fn pass_request(rig: &Rig, node: usize) -> mailbox_holder::Request {
    let time = rig.clock.wall();
    let access = rig.nodes[node]
        .core
        .mailbox_access(rig.nodes[node].own_transport_key(), period(time), time)
        .unwrap()
        .expect("an active book");
    mailbox_holder::Request::Access {
        credential: CredentialWire::Pass {
            pass: AccessPassWire::from(&access.pass),
            grant: access.grant,
        },
    }
}

fn refused(code: &str) -> mailbox_holder::Response {
    mailbox_holder::Response::Refused { code: code.into() }
}

#[tokio::test(flavor = "current_thread")]
async fn a_node_serves_nothing_but_a_pass_to_a_peer_it_does_not_know() {
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        ..Setup::default()
    })
    .await;
    let (alice, bob, stranger) = (
        peer_of_node(&rig, ALICE),
        peer_of_node(&rig, BOB),
        PeerId::random(),
    );
    let read = mailbox_holder::Request::Read {
        mailbox: vec![0xa1; 32],
        after: 0,
        limit: 16,
    };
    let time = rig.clock.wall();
    let stamp = rig.nodes[ALICE]
        .core
        .stamp_mailbox(&[0xa1; 32], period(time), b"paid", time)
        .unwrap();
    accept_grants(&mut rig);
    let grant = grant(
        rig.nodes[ALICE].core.mailbox_book_account().unwrap(),
        period(time),
        0,
    );
    // Even requests that carry their own payment wait for a pass: a node
    // cannot tell them apart before reading them.
    let gated = [
        read.clone(),
        mailbox_holder::Request::Store {
            mailbox: vec![0xa1; 32],
            period: period(time),
            envelope: b"paid".to_vec(),
            stamp: mailbox_holder::StampWire::from(&stamp),
        },
        mailbox_holder::Request::Notarize {
            statements: vec![StatementWire::Ticket {
                stamp: mailbox_holder::StampWire::from(&stamp),
            }],
        },
        mailbox_holder::Request::Notarize {
            statements: vec![StatementWire::Grant {
                grant: grant.clone(),
            }],
        },
        mailbox_holder::Request::LearnGrant { grant },
        mailbox_holder::Request::Directory,
        mailbox_holder::Request::Proofs { after: 0 },
        mailbox_holder::Request::Notarize {
            statements: vec![StatementWire::Commit { claim: vec![1] }],
        },
        mailbox_holder::Request::Summaries { items: vec![] },
    ];
    for request in gated {
        assert_eq!(
            rig.nodes[0].serve_mailbox(stranger, request.clone()),
            refused("access_required"),
            "{request:?}"
        );
    }
    let show = pass_request(&rig, ALICE);
    assert!(matches!(
        rig.nodes[0].serve_mailbox(alice, show.clone()),
        mailbox_holder::Response::Access { .. }
    ));
    assert!(matches!(
        rig.nodes[0].serve_mailbox(alice, read.clone()),
        mailbox_holder::Response::Page { .. }
    ));
    assert!(matches!(
        rig.nodes[0].serve_mailbox(alice, mailbox_holder::Request::Directory),
        mailbox_holder::Response::Directory { .. }
    ));
    // Replication stays among units, whatever pass a peer shows.
    assert_eq!(
        rig.nodes[0].serve_mailbox(alice, mailbox_holder::Request::Summaries { items: vec![] }),
        refused("access_required")
    );
    // Alice's pass shown by Bob lets Bob nowhere.
    assert_eq!(rig.nodes[0].serve_mailbox(bob, show), refused("bad_pass"));
    assert_eq!(rig.nodes[0].access_gate.principal(&bob), None);
}

#[tokio::test(flavor = "current_thread")]
async fn a_reader_without_an_active_book_reads_nothing_until_it_has_one() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        bob_bookless: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let sent = rig.nodes[ALICE]
        .core
        .send_message(&c, "still without a book", "m1", time)
        .unwrap()
        .id;
    rig.run_until(STEPS, |r| stored(r, ALICE, &sent)).await;
    assert!(stored(&rig, ALICE, &sent), "{:?}", rig.trace);
    idle(&mut rig, 30).await;
    assert!(incoming(&rig, BOB, &c).is_empty());
    let info = rig.nodes[BOB].mailbox_client.info();
    assert_eq!(info["access"]["credential"], "none");
    // Without a pass to show, the node does not even ask.
    assert!(info["sent"]["read"].is_null(), "{info}");
    // Bob buys a book; holders find it on the chain when he shows it.
    let key = rig.nodes[BOB].core.mailbox_book_account().unwrap();
    let book = [0xe5; 32];
    let terms = BookTerms {
        key,
        count: 1_000,
        valid_until: time + 30 * PERIOD_SECONDS,
    };
    rig.nodes[BOB]
        .core
        .add_mailbox_book(book, terms.count, terms.valid_until)
        .unwrap();
    rig.chain.buy(book, record(&terms));
    let bought = rig.clock.instant();
    rig.run_until(STEPS, |r| incoming(r, BOB, &c) == ["still without a book"])
        .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["still without a book"],
        "{}",
        rig.nodes[BOB].mailbox_client.info()
    );
    assert!(
        rig.clock.instant() - bought <= Duration::from_secs(120),
        "an honest reader was kept waiting"
    );
    assert_eq!(
        rig.nodes[BOB].mailbox_client.info()["access"]["credential"],
        "book"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn holders_that_forgot_every_pass_are_shown_one_again() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup::default()).await;
    let time = rig.clock.wall();
    let first = rig.nodes[ALICE]
        .core
        .send_message(&c, "the first", "m1", time)
        .unwrap()
        .id;
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &first) && incoming(r, BOB, &c) == ["the first"]
    })
    .await;
    assert_eq!(incoming(&rig, BOB, &c), ["the first"], "{:?}", rig.trace);
    // Every holder restarts and keeps no pass.
    for holder in 0..HOLDERS {
        rig.nodes[holder].access_gate.forget_passes();
    }
    let restarted = rig.clock.instant();
    let second = rig.nodes[ALICE]
        .core
        .send_message(&c, "the second", "m2", time)
        .unwrap()
        .id;
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &second) && incoming(r, BOB, &c) == ["the first", "the second"]
    })
    .await;
    assert!(stored(&rig, ALICE, &second), "{:?}", rig.trace);
    assert_eq!(incoming(&rig, BOB, &c), ["the first", "the second"]);
    assert!(rig.clock.instant() - restarted <= Duration::from_secs(120));
}

#[tokio::test(flavor = "current_thread")]
async fn a_new_day_needs_a_new_pass_and_the_client_shows_it() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup::default()).await;
    let time = rig.clock.wall();
    let first = rig.nodes[ALICE]
        .core
        .send_message(&c, "yesterday", "m1", time)
        .unwrap()
        .id;
    rig.run_until(STEPS, |r| incoming(r, BOB, &c) == ["yesterday"])
        .await;
    assert!(stored(&rig, ALICE, &first));
    // Past yesterday's passes and their hour of grace.
    rig.clock
        .advance(Duration::from_secs(PERIOD_SECONDS + 2 * 3_600));
    let today = rig.clock.wall();
    let started = rig.clock.instant();
    let second = rig.nodes[ALICE]
        .core
        .send_message(&c, "today", "m2", today)
        .unwrap()
        .id;
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &second) && incoming(r, BOB, &c) == ["yesterday", "today"]
    })
    .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["yesterday", "today"],
        "{:?}",
        rig.trace
    );
    assert!(rig.clock.instant() - started <= Duration::from_secs(120));
}

#[tokio::test(flavor = "current_thread")]
async fn a_unit_missing_from_the_directory_introduces_itself_with_its_record() {
    let Chat { mut rig, .. } = chat(Setup {
        directory_from_chain: true,
        silent_cards: true,
        ..Setup::default()
    })
    .await;
    // Alice has read the registry but pulled no record yet.
    let units = (0..HOLDERS)
        .map(|h| rig.nodes[h].own_commitment())
        .collect();
    let instant = rig.clock.instant();
    rig.nodes[ALICE].units_read(Some(units), instant);
    let time = rig.clock.wall();
    let unit = |rig: &Rig, node: usize| mailbox_holder::Request::Access {
        credential: CredentialWire::Unit {
            record: mailbox_holder::UnitRecordWire::from(&rig.nodes[node].unit_record(time)),
        },
    };
    let (one, two) = (peer_of_node(&rig, 1), peer_of_node(&rig, 2));
    assert_eq!(rig.nodes[ALICE].access_gate.principal(&one), None);
    // Another unit's record is not this peer's.
    assert_eq!(
        {
            let shown = unit(&rig, 1);
            rig.nodes[ALICE].serve_mailbox(two, shown)
        },
        refused("bad_pass")
    );
    // A record of a node the registry does not list.
    let carol_like = peer_of_node(&rig, BOB);
    assert_eq!(
        {
            let shown = unit(&rig, BOB);
            rig.nodes[ALICE].serve_mailbox(carol_like, shown)
        },
        refused("unknown_unit")
    );
    assert!(matches!(
        {
            let shown = unit(&rig, 1);
            rig.nodes[ALICE].serve_mailbox(one, shown)
        },
        mailbox_holder::Response::Access { .. }
    ));
    assert_eq!(
        rig.nodes[ALICE].access_gate.principal(&one),
        Some(processing::Principal::Unit)
    );
}

/// Show `request` until the node's grant rules are read.
async fn shown(
    rig: &mut Rig,
    node: usize,
    from: PeerId,
    request: &mailbox_holder::Request,
) -> mailbox_holder::Response {
    let mut answer = rig.nodes[node].serve_mailbox(from, request.clone());
    for _ in 0..100 {
        if answer != pending() {
            break;
        }
        rig.step().await;
        idle(rig, 1).await;
        answer = rig.nodes[node].serve_mailbox(from, request.clone());
    }
    answer
}

#[tokio::test(flavor = "current_thread")]
async fn a_granted_newcomer_is_let_in_before_the_notaries_saw_its_grant() {
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let grant = grant_alice(&mut rig, period(WALL), 0);
    let alice = peer_of_node(&rig, ALICE);
    let show = pass_request(&rig, ALICE);
    let mailbox_holder::Request::Access {
        credential: CredentialWire::Pass { grant: carried, .. },
    } = &show
    else {
        panic!("a pass");
    };
    assert_eq!(carried.as_ref(), Some(&grant));
    let answer = shown(&mut rig, 0, alice, &show).await;
    assert!(
        matches!(answer, mailbox_holder::Response::Access { .. }),
        "{answer:?}"
    );
    assert_eq!(
        rig.nodes[0].access_gate.principal(&alice),
        Some(processing::Principal::Book(grant.id()))
    );
    // Let in, but the grant pays for nothing until its notaries vouch.
    assert_eq!(
        rig.nodes[0].mailbox_holder.granted(&grant.id()).unwrap(),
        None
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_grant_its_issuer_never_signed_lets_nobody_in() {
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    // Alice mints herself a grant with a key the registry never made an
    // issuer.
    let book = rig.nodes[ALICE].core.mailbox_book_account().unwrap();
    let forged = GrantBook::issue(
        GrantTerms {
            domain: NETWORK_DOMAIN,
            book,
            day: period(WALL),
            serial: 0,
            count: 100,
            expiry: (period(WALL) + 20) * PERIOD_SECONDS,
        },
        &SecpKey::from_secret(&[0x77; 32]).unwrap(),
    );
    rig.nodes[ALICE].add_mailbox_grant(&forged).unwrap();
    let alice = peer_of_node(&rig, ALICE);
    let show = pass_request(&rig, ALICE);
    assert_eq!(shown(&mut rig, 0, alice, &show).await, refused("grant"));
    assert_eq!(rig.nodes[0].access_gate.principal(&alice), None);
    // A fresh second, so only the penalty can keep her out.
    rig.clock.advance(Duration::from_secs(2));
    assert!(
        rig.nodes[0].access_gate.admit(alice, None).is_err(),
        "a forger waits before trying again"
    );
}

/// Grants travel openly (`LearnGrant`, pages of reads): whoever copies
/// Alice's cannot sign a pass for it.
#[tokio::test(flavor = "current_thread")]
async fn a_copied_grant_lets_in_only_its_owner() {
    // Bob has no book of his own to be let in with.
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        alice_granted: true,
        bob_bookless: true,
        ..Setup::default()
    })
    .await;
    accept_grants(&mut rig);
    let grant = grant_alice(&mut rig, period(WALL), 0);
    let bob = peer_of_node(&rig, BOB);
    let time = rig.clock.wall();
    let thief = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x66; 32]).unwrap();
    let pass = agentic_mailbox_swarm::access::AccessPass::sign(
        &NETWORK_DOMAIN,
        grant.id(),
        rig.nodes[BOB].own_transport_key(),
        period(time),
        &thief,
    );
    let show = mailbox_holder::Request::Access {
        credential: CredentialWire::Pass {
            pass: AccessPassWire::from(&pass),
            grant: Some(grant),
        },
    };
    assert_eq!(shown(&mut rig, 0, bob, &show).await, refused("bad_pass"));
    assert_eq!(rig.nodes[0].access_gate.principal(&bob), None);
}

#[tokio::test(flavor = "current_thread")]
async fn a_peer_showing_unknown_books_gets_one_chain_read_a_minute() {
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        bob_bookless: true,
        ..Setup::default()
    })
    .await;
    let bob = peer_of_node(&rig, BOB);
    let time = rig.clock.wall();
    rig.nodes[BOB].core.mailbox_book_account().unwrap();
    let mut books = Vec::new();
    for n in 0..3u8 {
        // Each new book lasts longest, so it is the one shown.
        let book = [0xe0 + n; 32];
        rig.nodes[BOB]
            .core
            .add_mailbox_book(book, 10, time + (10 + u64::from(n)) * PERIOD_SECONDS)
            .unwrap();
        let show = pass_request(&rig, BOB);
        assert_eq!(
            rig.nodes[0].serve_mailbox(bob, show),
            refused("unknown_book")
        );
        books.push(book);
    }
    idle(&mut rig, 5).await;
    let reads = |rig: &Rig| -> usize { books.iter().map(|b| rig.chain.book_reads(0, b)).sum() };
    assert_eq!(reads(&rig), 1);
    idle(&mut rig, 60).await;
    let show = pass_request(&rig, BOB);
    assert_eq!(
        rig.nodes[0].serve_mailbox(bob, show),
        refused("unknown_book")
    );
    idle(&mut rig, 5).await;
    assert_eq!(reads(&rig), 2);
}

// --- open-read groups (Docs/V1_DISCOVERY_2026_09_27.md, part 2) ---------------

/// How many rounds of reading the groups it follows a node has made (a
/// round reads every open follow's mailboxes once).
fn follow_reads(rig: &Rig, node: usize) -> u64 {
    rig.nodes[node].mailbox_client.info()["public"]["followReads"]
        .as_u64()
        .unwrap_or(0)
}

#[tokio::test(flavor = "current_thread")]
async fn a_follower_reads_an_open_group_from_the_swarm_and_stops_when_it_closes() {
    let (mut rig, g, _) = made_group(Setup {
        dave: true,
        dave_outside: true,
        ..Setup::default()
    })
    .await;
    let members = [ALICE, BOB, CAROL];
    let epoch = epoch_of(&rig, ALICE, &g).unwrap();
    let opened = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "add": [], "remove": [], "access": "public", "operationId": "o-1"}),
    );
    assert!(opened.get("result").is_some(), "{opened}");
    rig.run_until(STEPS, |r| {
        members
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(epoch + 1))
    })
    .await;
    let group = rig.nodes[BOB].command("group", json!({ "groupId": g }));
    assert_eq!(group["result"]["access"], "public", "{group}");
    let reference = group["result"]["groupRef"].as_str().unwrap().to_owned();
    let owner = network_id_of(&rig, ALICE);
    let followed = rig.nodes[DAVE].command(
        "follow_group",
        json!({"group": reference, "owner": owner, "name": "Team"}),
    );
    let follow = followed["result"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{followed}"))
        .to_owned();
    let time = rig.clock.wall();
    rig.nodes[BOB]
        .core
        .send_message(&g, "for everyone", "m1", time)
        .unwrap();
    let posted = rig.clock.instant();
    let reads_before = follow_reads(&rig, DAVE);
    rig.run_until(STEPS, |r| {
        incoming(r, DAVE, &follow) == ["for everyone"] && incoming(r, ALICE, &g) == ["for everyone"]
    })
    .await;
    assert_eq!(
        incoming(&rig, DAVE, &follow),
        ["for everyone"],
        "{:?}",
        rig.trace
    );
    assert_eq!(incoming(&rig, ALICE, &g), ["for everyone"]);
    assert_eq!(
        texts(&rig, BOB, &g),
        ["for everyone"],
        "the author keeps one copy"
    );
    // A follower reads about once a minute, not at a member's pace.
    let waited = rig.clock.instant() - posted;
    assert!(waited <= Duration::from_secs(180), "{waited:?}");
    let reads = follow_reads(&rig, DAVE) - reads_before;
    assert!(
        reads <= waited.as_secs() / 60 + 2,
        "{reads} follower reads in {waited:?}"
    );
    // Closed again: Dave sees it and reads nothing more.
    let closed = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "add": [], "remove": [], "access": "private", "operationId": "o-2"}),
    );
    assert!(closed.get("result").is_some(), "{closed}");
    rig.run_until(STEPS, |r| {
        r.nodes[DAVE]
            .core
            .follows()
            .unwrap()
            .first()
            .is_some_and(|f| f.closed)
    })
    .await;
    assert!(
        rig.nodes[DAVE].core.follows().unwrap()[0].closed,
        "{:?}",
        rig.trace
    );
    let stopped = follow_reads(&rig, DAVE);
    idle(&mut rig, 180).await;
    assert_eq!(follow_reads(&rig, DAVE), stopped);
}

// --- channels (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts 8–10c) -------

use agentic_protocol::directory::{Card, Request};

/// Archive parts a node laid and saw stored at a quorum.
fn archives_laid(rig: &Rig, node: usize) -> u64 {
    rig.nodes[node].mailbox_client.info()["public"]["archivesLaid"]
        .as_u64()
        .unwrap_or(0)
}

/// Archive parts a node read that it had not read before.
fn archives_read(rig: &Rig, node: usize) -> u64 {
    rig.nodes[node].mailbox_client.info()["public"]["archives"]
        .as_u64()
        .unwrap_or(0)
}

/// `node` follows the channel `reference` of Alice: the conversation.
fn follow(rig: &mut Rig, node: usize, reference: &str) -> String {
    let owner = network_id_of(rig, ALICE);
    let followed = rig.nodes[node].command(
        "follow_group",
        json!({"group": reference, "owner": owner, "name": "News"}),
    );
    followed["result"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{followed}"))
        .to_owned()
}

/// A channel's follower reads back what the mailboxes keep — a month, not
/// a day, once — then new posts every five minutes; its owner's node lays
/// the archive, where a later follower finds what no mailbox keeps.
#[tokio::test(flavor = "current_thread")]
async fn a_channels_follower_reads_its_month_back_every_five_minutes_and_the_archive_beyond() {
    const DAY: u64 = PERIOD_SECONDS;
    let Chat { mut rig, .. } = chat(Setup {
        carol: true,
        dave: true,
        strangers: true,
        ..Setup::default()
    })
    .await;
    // Bob has no part in it.
    rig.paused.insert(BOB);
    // Books that outlast the month, as a bought or granted one would.
    let until = rig.clock.wall() + 100 * DAY;
    for node in [ALICE, CAROL, DAVE] {
        let book = [0xc0 + u8::try_from(node).unwrap(); 32];
        let key = rig.nodes[node].core.mailbox_book_account().unwrap();
        rig.nodes[node]
            .core
            .add_mailbox_book(book, 1_000, until)
            .unwrap();
        let terms = BookTerms {
            key,
            count: 1_000,
            valid_until: until,
        };
        for holder in 0..HOLDERS {
            rig.nodes[holder]
                .mailbox_holder
                .learn_book(book, terms)
                .unwrap();
        }
    }
    let made = rig.nodes[ALICE].command(
        "create_group",
        json!({"name": "News", "members": [], "kind": "channel", "access": "public", "operationId": "c-1"}),
    );
    assert_eq!(
        (&made["result"]["kind"], &made["result"]["access"]),
        (&json!("channel"), &json!("public")),
        "{made}"
    );
    let c = made["result"]["id"].as_str().unwrap().to_owned();
    let reference = made["result"]["groupRef"].as_str().unwrap().to_owned();
    let kept = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": c, "add": [], "remove": [], "retention": 90, "operationId": "c-2"}),
    );
    assert!(kept.get("result").is_some(), "{kept}");
    rig.run_until(STEPS, |r| {
        r.nodes[ALICE].core.group(&c).unwrap().retention == Some(90)
    })
    .await;
    let group = rig.nodes[ALICE].command("group", json!({ "groupId": c }));
    assert_eq!(group["result"]["retention"], 90, "{group} {:?}", rig.trace);
    let start = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&c, "at the start of the month", "m1", start)
        .unwrap();
    let sent = |r: &Rig| r.nodes[ALICE].core.swarm_outbox(8).unwrap().is_empty();
    rig.run_until(STEPS, sent).await;
    assert!(sent(&rig), "{:?}", rig.trace);
    // Ten days on, Dave follows: he reads the channel's month back.
    rig.clock.advance(Duration::from_secs(10 * DAY));
    let dave = follow(&mut rig, DAVE, &reference);
    rig.run_until(STEPS, |r| {
        incoming(r, DAVE, &dave) == ["at the start of the month"]
    })
    .await;
    assert_eq!(
        incoming(&rig, DAVE, &dave),
        ["at the start of the month"],
        "{:?}",
        rig.trace
    );
    // A new post reaches him within minutes; he reads every five.
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&c, "fresh", "m2", time)
        .unwrap();
    let posted = rig.clock.instant();
    rig.run_until(STEPS, |r| incoming(r, DAVE, &dave).len() == 2)
        .await;
    assert_eq!(
        incoming(&rig, DAVE, &dave),
        ["at the start of the month", "fresh"],
        "{:?}",
        rig.trace
    );
    let waited = rig.clock.instant() - posted;
    assert!(waited <= Duration::from_secs(11 * 60), "{waited:?}");
    // Six minutes: one or two rounds, where a minute's pace makes six.
    let before = follow_reads(&rig, DAVE);
    idle(&mut rig, 6 * 60).await;
    let rounds = follow_reads(&rig, DAVE) - before;
    assert!((1..=2).contains(&rounds), "{rounds} rounds in 6 min");
    // The month was read once: only the last two days stay in his reading.
    let today = period(rig.clock.wall());
    let info = rig.nodes[DAVE].mailbox_client.info();
    let conversation = format!("public:{dave}");
    let reads: Vec<u64> = info["reads"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["conversationId"] == conversation.as_str())
        .map(|r| r["period"].as_u64().unwrap())
        .collect();
    assert!(
        !reads.is_empty() && reads.iter().all(|p| p + 1 >= today),
        "{reads:?} on day {today}"
    );
    // 26 days on, Alice's node lays the part of both posts.
    rig.clock.advance(Duration::from_secs(16 * DAY));
    rig.run_until(STEPS, |r| archives_laid(r, ALICE) >= 1).await;
    assert!(archives_laid(&rig, ALICE) >= 1, "{:?}", rig.trace);
    // Its card for the directory says it is a channel, and where it is.
    let card = rig.nodes[ALICE].command(
        "discover_card",
        json!({"kind": "group", "groupId": c, "about": "The week's main news", "tags": ["rust"]}),
    );
    let wire = hex::decode(
        card["result"]["card"]
            .as_str()
            .unwrap_or_else(|| panic!("{card}")),
    )
    .unwrap();
    let signed =
        agentic_protocol::VerifiedDocument::decode(&wire, NETWORK_DOMAIN, rig.clock.wall())
            .unwrap();
    let Ok(Request::Card(Card::Channel { group_id, name, .. })) = Request::decode(signed.body())
    else {
        panic!("{card}")
    };
    assert_eq!(
        (
            hex::encode(agentic_protocol::group::group_ref(
                &NETWORK_DOMAIN,
                signed.author(),
                &group_id
            )),
            name.as_str()
        ),
        (reference.clone(), "News")
    );
    // What keeping it costs, as the owner is shown.
    let storage = rig.nodes[ALICE].command("channel_storage", json!({ "groupId": c }));
    assert_eq!(
        (
            &storage["result"]["retention"],
            &storage["result"]["parts"],
            &storage["result"]["stampsPerMonth"]
        ),
        (&json!(90), &json!(1), &json!(2)),
        "{storage}"
    );
    // 36 days on, the first post is in no follower's month any more:
    // Carol, following then, finds it in the archive.
    rig.clock.advance(Duration::from_secs(10 * DAY));
    let carol = follow(&mut rig, CAROL, &reference);
    rig.run_until(STEPS, |r| {
        incoming(r, CAROL, &carol).len() == 2 && archives_read(r, CAROL) >= 1
    })
    .await;
    assert_eq!(
        incoming(&rig, CAROL, &carol),
        ["at the start of the month", "fresh"],
        "{:?}",
        rig.trace
    );
    assert_eq!(archives_read(&rig, CAROL), 1);
    let followed = rig.nodes[CAROL].command("follows", json!({}));
    assert_eq!(
        (
            &followed["result"][0]["kind"],
            &followed["result"][0]["retention"]
        ),
        (&json!("channel"), &json!(90)),
        "{followed}"
    );
    // Laid once: the part is due again only 25 days after its copy.
    assert_eq!(archives_laid(&rig, ALICE), 1);
}

/// A closed channel's key updates a node published and saw stored at a
/// quorum.
fn key_updates(rig: &Rig, node: usize) -> u64 {
    rig.nodes[node].mailbox_client.info()["public"]["keyUpdates"]
        .as_u64()
        .unwrap_or(0)
}

/// Closed channels' keys a node took, read from its intro mailbox or
/// delivered directly.
fn subscriptions(rig: &Rig, node: usize) -> u64 {
    let intro = &rig.nodes[node].mailbox_client.info()["intro"];
    ["swarm", "direct"]
        .iter()
        .map(|way| intro[way]["subscribed"].as_u64().unwrap_or(0))
        .sum()
}

/// Key updates a subscriber's node took: its channel's key moved on.
fn rekeyed(rig: &Rig, node: usize) -> u64 {
    rig.nodes[node].mailbox_client.info()["public"]["rekeyed"]
        .as_u64()
        .unwrap_or(0)
}

/// A closed channel reaches its subscribers through the swarm: each gets
/// its keys through its intro mailbox and reads the sealed posts, which its
/// team reads too; one removed reads nothing after the key update Alice's
/// node publishes, while Bob, in the team, derives the new key himself.
#[tokio::test(flavor = "current_thread")]
async fn a_closed_channels_subscribers_read_it_through_the_swarm_until_one_is_removed() {
    let Chat { mut rig, .. } = chat(Setup {
        carol: true,
        dave: true,
        strangers: true,
        ..Setup::default()
    })
    .await;
    for node in [BOB, CAROL, DAVE] {
        let mailbox = rig.nodes[node]
            .core
            .own_intro_mailbox(rig.clock.wall())
            .unwrap();
        rig.run_until(STEPS, |r| keeping(r, &mailbox, 1) >= QUORUM)
            .await;
    }
    let bob = network_id_of(&rig, BOB);
    let made = when_cards_are_read(
        &mut rig,
        ALICE,
        "create_group",
        json!({"name": "Club", "members": [bob], "kind": "channel", "access": "private", "operationId": "c-1"}),
    )
    .await;
    assert_eq!(made["result"]["access"], "private", "{made}");
    let c = made["result"]["id"].as_str().unwrap().to_owned();
    let reference = made["result"]["groupRef"].as_str().unwrap().to_owned();
    rig.run_until(STEPS, |r| epoch_of(r, BOB, &c).is_some())
        .await;
    assert_eq!(
        rig.nodes[BOB].core.group(&c).unwrap().role,
        "admin",
        "{:?}",
        rig.trace
    );
    let (carol, dave) = (network_id_of(&rig, CAROL), network_id_of(&rig, DAVE));
    let request = json!({"groupId": c, "members": [dave, carol], "operationId": "s-1"});
    let given = when_cards_are_read(&mut rig, ALICE, "channel_subscribe", request.clone()).await;
    assert!(given.get("result").is_some(), "{given}");
    // Safe to retry: the same answer, and no keys given twice.
    assert_eq!(
        rig.nodes[ALICE].command("channel_subscribe", request),
        given
    );
    // Written while the keys are on their way.
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&c, "members only", "m1", time)
        .unwrap();
    // Once Dave follows it, his node reads only the last two days: a
    // closed channel has no history to read back.
    let conversation = format!("public:{reference}");
    let read_periods = |r: &Rig| -> Vec<u64> {
        r.nodes[DAVE].mailbox_client.info()["reads"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|read| read["conversationId"] == conversation.as_str())
            .map(|read| read["period"].as_u64().unwrap())
            .collect()
    };
    rig.run_until(STEPS, |r| !read_periods(r).is_empty()).await;
    let today = period(rig.clock.wall());
    let reads = read_periods(&rig);
    assert!(
        !reads.is_empty() && reads.iter().all(|p| p + 1 >= today),
        "{reads:?} on day {today}"
    );
    rig.run_until(STEPS, |r| {
        [DAVE, CAROL]
            .iter()
            .all(|n| incoming(r, *n, &reference) == ["members only"])
            && incoming(r, BOB, &c) == ["members only"]
    })
    .await;
    for n in [DAVE, CAROL] {
        assert_eq!(
            incoming(&rig, n, &reference),
            ["members only"],
            "{:?}",
            rig.trace
        );
    }
    assert_eq!(incoming(&rig, BOB, &c), ["members only"]);
    for n in [DAVE, CAROL] {
        assert_eq!(subscriptions(&rig, n), 1);
    }
    // Carol is removed: Alice's node publishes the key update once, Dave's
    // takes it and reads on; Carol's reads nothing more.
    let removed = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": c, "unsubscribe": [carol], "operationId": "u-1"}),
    );
    assert!(removed.get("result").is_some(), "{removed}");
    rig.run_until(STEPS, |r| {
        rekeyed(r, DAVE) >= 1 && key_updates(r, ALICE) >= 1
    })
    .await;
    assert_eq!(rekeyed(&rig, DAVE), 1, "{:?}", rig.trace);
    // Stored at a quorum, it is sent no more.
    assert_eq!(key_updates(&rig, ALICE), 1);
    let wall = rig.clock.wall();
    assert!(
        rig.nodes[ALICE]
            .core
            .channel_key_docs(&c, wall)
            .unwrap()
            .is_empty()
    );
    // Bob applied the commit: he writes under the new key.
    let epoch = epoch_of(&rig, ALICE, &c);
    rig.run_until(STEPS, |r| epoch_of(r, BOB, &c) == epoch)
        .await;
    assert_eq!(epoch_of(&rig, BOB, &c), epoch, "{:?}", rig.trace);
    let time = rig.clock.wall();
    rig.nodes[BOB]
        .core
        .send_message(&c, "already without Carol", "m2", time)
        .unwrap();
    rig.run_until(STEPS, |r| incoming(r, DAVE, &reference).len() == 2)
        .await;
    assert_eq!(
        incoming(&rig, DAVE, &reference),
        ["members only", "already without Carol"],
        "{:?}",
        rig.trace
    );
    assert_eq!(incoming(&rig, CAROL, &reference), ["members only"]);
    assert_eq!(rekeyed(&rig, CAROL), 0);
}

/// Keys of their own subscribers published, as the owner's node took them.
fn subscriber_keys(rig: &Rig, node: usize) -> u64 {
    rig.nodes[node].mailbox_client.info()["public"]["subscriberKeys"]
        .as_u64()
        .unwrap_or(0)
}

/// Reseeds onto subscribers' own keys the owner's node made.
fn hard_reseeds(rig: &Rig, node: usize) -> u64 {
    rig.nodes[node].mailbox_client.info()["public"]["hardReseeds"]
        .as_u64()
        .unwrap_or(0)
}

/// A closed channel's owner moves it to new keys by itself once an admin
/// is out of its team: each subscriber published a key of its own once,
/// the owner's node takes it, and after the admin's removal makes a reseed
/// onto those keys in a commit the admin is not in. A subscriber whose
/// keys still wait for its decision published none: the owner's node gives
/// them again, finding its card itself. Both read on; the former admin
/// reads nothing new.
#[tokio::test(flavor = "current_thread")]
async fn a_closed_channels_owner_moves_it_to_new_keys_once_an_admin_is_out() {
    let Chat { mut rig, .. } = chat(Setup {
        carol: true,
        dave: true,
        strangers: true,
        ..Setup::default()
    })
    .await;
    for node in [BOB, CAROL, DAVE] {
        let mailbox = rig.nodes[node]
            .core
            .own_intro_mailbox(rig.clock.wall())
            .unwrap();
        rig.run_until(STEPS, |r| keeping(r, &mailbox, 1) >= QUORUM)
            .await;
    }
    rig.nodes[CAROL]
        .core
        .set_intro_policy(agentic_core::IntroPolicy {
            mode: agentic_core::IntroMode::Manual,
            daily_limit: 20,
            allowed: vec![],
        })
        .unwrap();
    let (bob, carol, dave) = (
        network_id_of(&rig, BOB),
        network_id_of(&rig, CAROL),
        network_id_of(&rig, DAVE),
    );
    let made = when_cards_are_read(
        &mut rig,
        ALICE,
        "create_group",
        json!({"name": "Club", "members": [bob], "kind": "channel", "access": "private", "operationId": "c-1"}),
    )
    .await;
    let c = made["result"]["id"].as_str().unwrap().to_owned();
    let reference = made["result"]["groupRef"].as_str().unwrap().to_owned();
    rig.run_until(STEPS, |r| epoch_of(r, BOB, &c).is_some())
        .await;
    let given = when_cards_are_read(
        &mut rig,
        ALICE,
        "channel_subscribe",
        json!({"groupId": c, "members": [dave, carol], "operationId": "s-1"}),
    )
    .await;
    assert!(given.get("result").is_some(), "{given}");
    rig.run_until(STEPS, |r| {
        subscriptions(r, DAVE) == 1
            && subscriber_keys(r, ALICE) >= 1
            && !r.nodes[CAROL].core.intro_requests().unwrap().is_empty()
    })
    .await;
    assert_eq!(subscriber_keys(&rig, ALICE), 1, "{:?}", rig.trace);
    assert_eq!(subscriptions(&rig, CAROL), 0);
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&c, "while Bob is on the team", "m1", time)
        .unwrap();
    rig.run_until(STEPS, |r| incoming(r, DAVE, &reference).len() == 1)
        .await;

    let removed = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": c, "remove": [bob], "operationId": "r-1"}),
    );
    assert!(removed.get("result").is_some(), "{removed}");
    // The reseed, then Carol's keys again: two requests wait at hers.
    rig.run_until(STEPS, |r| {
        hard_reseeds(r, ALICE) >= 1
            && rekeyed(r, DAVE) >= 1
            && r.nodes[CAROL].core.intro_requests().unwrap().len() >= 2
    })
    .await;
    assert_eq!(hard_reseeds(&rig, ALICE), 1, "{:?}", rig.trace);
    assert_eq!(hard_reseeds(&rig, BOB), 0);
    let time = rig.clock.wall();
    for request in rig.nodes[CAROL].core.intro_requests().unwrap() {
        rig.nodes[CAROL]
            .core
            .accept_intro_request(&request.request_id, time)
            .unwrap();
    }
    let time = rig.clock.wall();
    rig.nodes[ALICE]
        .core
        .send_message(&c, "already without Bob", "m2", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        incoming(r, DAVE, &reference).len() == 2
            && incoming(r, CAROL, &reference).contains(&"already without Bob".to_owned())
    })
    .await;
    assert_eq!(
        incoming(&rig, DAVE, &reference),
        ["while Bob is on the team", "already without Bob"],
        "{:?}",
        rig.trace
    );
    assert!(
        incoming(&rig, CAROL, &reference).contains(&"already without Bob".to_owned()),
        "{:?}",
        rig.trace
    );
    assert!(!incoming(&rig, BOB, &c).contains(&"already without Bob".to_owned()));
}

// --- the discovery service's node (spec/discovery-v1.md) ------------------------

fn stamp_json(stamp: &agentic_mailbox_swarm::stamp::Stamp) -> Value {
    json!({
        "book": hex::encode(stamp.book),
        "index": stamp.index,
        "operation": hex::encode(stamp.operation),
        "signature": hex::encode(stamp.signature),
    })
}

/// The node beside the discovery service checks what the service is paid
/// with: each slot once, against its book read from the chain; it also says
/// whether a searcher's book is active, and with which key.
#[tokio::test(flavor = "current_thread")]
async fn the_services_node_redeems_each_stamp_once_and_knows_the_books() {
    use agentic_mailbox_swarm::discover::lookup_operation;
    use agentic_mailbox_swarm::stamp::{BookKey, Stamp};
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let key = BookKey::from_bytes(&[9; 32]).unwrap();
    let book = [0xd1; 32];
    let terms = BookTerms {
        key: key.account(),
        count: 3,
        valid_until: time + PERIOD_SECONDS,
    };
    rig.chain.buy(book, record(&terms));
    let op = |n: u8| lookup_operation(&NETWORK_DOMAIN, "google", &[n; 32], 1);
    let stamp = |index: u32, n: u8| Stamp::sign(&NETWORK_DOMAIN, book, index, op(n), &key);
    let status = |rig: &mut Rig, book: [u8; 32]| {
        rig.nodes[0].command("book_status", json!({ "book": hex::encode(book) }))["result"].clone()
    };
    // Bought, but not read yet: read from the chain meanwhile.
    assert_eq!(
        redeem(&mut rig, 0, vec![stamp_json(&stamp(0, 1))], json!([])),
        ["unknown_book"]
    );
    assert_eq!(status(&mut rig, book)["state"], "unknown");
    idle(&mut rig, 5).await;
    assert_eq!(
        redeem(
            &mut rig,
            0,
            vec![stamp_json(&stamp(0, 1)), stamp_json(&stamp(1, 2))],
            json!([])
        ),
        ["ok", "ok"]
    );
    // The same stamp again is the same spend; a slot outside the book, or a
    // stamp of another key, pays nothing.
    let foreign = Stamp::sign(
        &NETWORK_DOMAIN,
        book,
        2,
        op(3),
        &BookKey::from_bytes(&[8; 32]).unwrap(),
    );
    assert_eq!(
        redeem(
            &mut rig,
            0,
            vec![
                stamp_json(&stamp(0, 1)),
                stamp_json(&stamp(3, 3)),
                stamp_json(&foreign)
            ],
            json!([])
        ),
        ["ok", "index", "signature"]
    );
    let active = status(&mut rig, book);
    assert_eq!(
        (active["state"].as_str(), active["key"].as_str()),
        (Some("active"), Some(hex::encode(key.account()).as_str()))
    );
    // Its day over, the book pays for nothing more.
    rig.clock.advance(Duration::from_secs(PERIOD_SECONDS));
    assert_eq!(status(&mut rig, book)["state"], "ended");
    assert_eq!(
        redeem(&mut rig, 0, vec![stamp_json(&stamp(2, 4))], json!([])),
        ["expired"]
    );
}

/// What `redeem_stamps` answers for each of `stamps` at `node`.
fn redeem(rig: &mut Rig, node: usize, stamps: Vec<Value>, grants: Value) -> Vec<String> {
    let answer = rig.nodes[node].command(
        "redeem_stamps",
        json!({ "stamps": stamps, "grants": grants }),
    );
    answer["result"]["results"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"))
        .iter()
        .map(|r| r.as_str().unwrap().to_owned())
        .collect()
}

/// A slot spent at the discovery service's node for one operation and at
/// another node for another is a double spend every node blocks; a
/// newcomer's grant, shown with its stamps, pays once its notaries vouch.
#[tokio::test(flavor = "current_thread")]
async fn a_slot_spent_twice_at_two_nodes_blocks_the_book_and_a_grant_pays_once_vouched() {
    use agentic_mailbox_swarm::discover::lookup_operation;
    use agentic_mailbox_swarm::stamp::{BookKey, Stamp};
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        sync: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let key = BookKey::from_bytes(&[9; 32]).unwrap();
    let book = [0xd1; 32];
    for holder in 0..HOLDERS {
        rig.nodes[holder]
            .mailbox_holder
            .learn_book(
                book,
                BookTerms {
                    key: key.account(),
                    count: 3,
                    valid_until: time + PERIOD_SECONDS,
                },
            )
            .unwrap();
    }
    let op = |n: u8| lookup_operation(&NETWORK_DOMAIN, "google", &[n; 32], 1);
    let slot = |n: u8| Stamp::sign(&NETWORK_DOMAIN, book, 0, op(n), &key);
    assert_eq!(
        redeem(&mut rig, 0, vec![stamp_json(&slot(1))], json!([])),
        ["ok"]
    );
    assert_eq!(
        redeem(&mut rig, 1, vec![stamp_json(&slot(2))], json!([])),
        ["ok"]
    );
    let blocked = |rig: &mut Rig, node: usize| {
        rig.nodes[node].command("book_status", json!({ "book": hex::encode(book) }))["result"]["state"]
            == "blocked"
    };
    rig.run_until(STEPS, |r| {
        [0, 1].iter().all(|n| {
            r.nodes[*n]
                .mailbox_holder
                .blocked_books()
                .unwrap()
                .contains(&book)
        })
    })
    .await;
    for node in [0, 1] {
        assert!(blocked(&mut rig, node), "node {node}: {:?}", rig.trace);
    }
    // A grant of the identity server, put on record on its day, pays at
    // the service's node once its notaries vouch for it; before, the node
    // lets its owner search on the issuer's rules alone.
    accept_grants(&mut rig);
    let newcomer = BookKey::from_bytes(&[10; 32]).unwrap();
    let granted = grant(newcomer.account(), period(time), 5);
    for holder in 0..HOLDERS {
        rig.nodes[holder]
            .mailbox_holder
            .notarize(&mailbox_holder::Statement::Grant(granted.clone()), time)
            .unwrap();
    }
    let stamp = Stamp::sign(&NETWORK_DOMAIN, granted.id(), 0, op(7), &newcomer);
    let grants = json!([granted]);
    let mut answer = redeem(&mut rig, 0, vec![stamp_json(&stamp)], grants.clone());
    for _ in 0..30 {
        if answer != ["unknown_book"] {
            break;
        }
        idle(&mut rig, 2).await;
        answer = redeem(&mut rig, 0, vec![stamp_json(&stamp)], grants.clone());
    }
    assert_eq!(answer, ["ok"]);
    let fresh_key = BookKey::from_bytes(&[11; 32]).unwrap();
    let fresh = grant(fresh_key.account(), period(time), 6);
    let mut state = Value::Null;
    for _ in 0..30 {
        state = rig.nodes[2].command(
            "book_status",
            json!({ "book": hex::encode(fresh.id()), "grant": fresh }),
        )["result"]
            .clone();
        if state["state"] == "active" {
            break;
        }
        idle(&mut rig, 1).await;
    }
    assert_eq!(state["state"], "active", "{state}");
    assert_eq!(state["key"], hex::encode(fresh_key.account()));
}

/// What a newcomer's node makes to pay the discovery service with carries
/// its grant, for the service's node that has not heard of the book; made
/// again for a retry it is the same, so a retry spends nothing more: the
/// same lookup the same day, the same card unchanged. The next day a lookup
/// is paid again, and a changed card is another card.
#[tokio::test(flavor = "current_thread")]
async fn a_newcomers_payment_carries_its_grant_and_a_retry_spends_nothing_more() {
    let Chat { mut rig, .. } = chat(Setup {
        silent_cards: true,
        alice_granted: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let granted = grant_alice(&mut rig, period(time), 1);
    let lookup = |rig: &mut Rig| {
        rig.nodes[ALICE].command(
            "discover_stamps",
            json!({"handles": [{"kind": "google", "digest": hex::encode([7; 32])}]}),
        )["result"]
            .clone()
    };
    let first = lookup(&mut rig);
    assert_eq!(first["grants"], json!([granted]), "{first}");
    assert_eq!(first["day"], time / 86_400);
    assert_eq!(lookup(&mut rig), first);
    let card = |rig: &mut Rig, about: &str| {
        rig.nodes[ALICE].command(
            "discover_card",
            json!({"kind": "profile", "about": about, "tags": ["rust"]}),
        )["result"]
            .clone()
    };
    let made = card(&mut rig, "I write in Rust");
    assert_eq!(made["grants"], json!([granted]), "{made}");
    rig.clock.advance(Duration::from_secs(60));
    assert_eq!(card(&mut rig, "I write in Rust"), made);
    // The retry took no slot: a changed card takes the next ones.
    let changed = card(&mut rig, "I write in Rust and Go");
    assert_eq!(
        changed["stamps"][0]["index"].as_u64(),
        made["stamps"][9]["index"].as_u64().map(|i| i + 1)
    );
    rig.clock.advance(Duration::from_secs(86_400));
    let next = lookup(&mut rig);
    assert_eq!(next["day"], time / 86_400 + 1);
    assert_ne!(next["stamps"][0]["index"], first["stamps"][0]["index"]);
}

/// Someone's card who is not on the rig: the invitation to it goes to an
/// intro mailbox nobody reads.
fn outsider_card(name: &str, time: u64) -> agentic_core::Invitee {
    let dir = tempfile::TempDir::new().unwrap();
    let mut core = agentic_core::AppCore::new(
        agentic_store::ProfileStore::open(dir.path().join("outsider.db"), &[5; 32]).unwrap(),
        NETWORK_DOMAIN,
    )
    .unwrap();
    core.create_profile(name).unwrap();
    agentic_core::Invitee {
        network_id: core.snapshot().unwrap().identity.unwrap().network_id,
        card: core.intro_card(vec![], time).unwrap().envelope,
    }
}

/// Alice adds 280 people at once (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
/// parts 1–3). The commit and the tree are too big for one envelope and go
/// in parts; Bob and Carol put the commit together and apply it. Dave, the
/// one of them on the rig, is let in by his invitation, reads the tree from
/// its own mailbox, joins and writes to everyone.
#[tokio::test(flavor = "current_thread")]
async fn a_newcomer_to_a_big_group_reads_the_tree_from_its_own_mailbox_and_joins() {
    let (mut rig, g, _) = made_group(Setup {
        dave: true,
        dave_outside: true,
        ..Setup::default()
    })
    .await;
    let time = rig.clock.wall();
    let mut add = vec![agentic_core::Invitee {
        network_id: network_id_of(&rig, DAVE),
        card: rig.nodes[DAVE]
            .core
            .intro_card(vec![], time)
            .unwrap()
            .envelope,
    }];
    add.extend((1..280).map(|i| outsider_card(&format!("P{i}"), time)));
    let e0 = epoch_of(&rig, ALICE, &g).unwrap();
    rig.nodes[ALICE]
        .core
        .change_group(
            &g,
            agentic_core::GroupChange {
                add,
                ..agentic_core::GroupChange::default()
            },
            "big-1",
            time,
        )
        .unwrap();
    // Let in, Dave waits for the tree before he is a member.
    let waiting = |r: &Rig| {
        !r.nodes[DAVE]
            .core
            .group_tree_mailboxes(r.clock.wall())
            .unwrap()
            .is_empty()
    };
    rig.run_until(STEPS, waiting).await;
    assert!(waiting(&rig), "{:?}", rig.trace);
    assert_eq!(epoch_of(&rig, DAVE, &g), None);
    let at_next = |r: &Rig| {
        [ALICE, BOB, CAROL, DAVE]
            .iter()
            .all(|n| epoch_of(r, *n, &g) == Some(e0 + 1))
    };
    rig.run_until(STEPS, at_next).await;
    assert!(at_next(&rig), "{:?}", rig.trace);
    for n in [ALICE, BOB, CAROL, DAVE] {
        assert_eq!(rig.nodes[n].core.group(&g).unwrap().members.len(), 283);
    }
    let wall = rig.clock.wall();
    assert!(
        rig.nodes[DAVE]
            .core
            .group_tree_mailboxes(wall)
            .unwrap()
            .is_empty()
    );
    // The newcomer writes; the members read it through the one mailbox,
    // within a minute of its storing — a wide read meets its quorum — and
    // one read interval.
    let sent = rig.nodes[DAVE]
        .core
        .send_message(&g, "thanks for having me", "m-dave", wall)
        .unwrap();
    rig.run_until(STEPS, |r| {
        r.nodes[DAVE]
            .core
            .swarm_receipts(&sent.id)
            .unwrap()
            .is_some()
    })
    .await;
    let stored = rig.clock.instant();
    let read_by = |r: &Rig, nodes: &[usize]| {
        nodes
            .iter()
            .all(|n| incoming(r, *n, &g).last().map(String::as_str) == Some("thanks for having me"))
    };
    // Bob and Carol only read; Alice's node also still sends invitations.
    rig.run_until(STEPS, |r| read_by(r, &[BOB, CAROL])).await;
    assert!(read_by(&rig, &[BOB, CAROL]), "{:?}", rig.trace);
    assert!(
        rig.clock.instant() <= stored + Duration::from_secs(75),
        "read {:?} after storing",
        rig.clock.instant() - stored
    );
    rig.run_until(STEPS, |r| read_by(r, &[ALICE])).await;
    assert!(read_by(&rig, &[ALICE]), "{:?}", rig.trace);
    // A big group is read gently (doc part 4): each poll from one holder,
    // from four once a minute, and less often the bigger it is — not from
    // all ten every five seconds.
    let asked = |r: &Rig| -> u64 {
        r.nodes[BOB].mailbox_client.info()["asked"][g.as_str()]
            .as_u64()
            .unwrap_or(0)
    };
    let before = asked(&rig);
    assert!(before > 0, "Bob has read the group");
    let start = rig.clock.instant();
    let two_minutes = |r: &Rig| r.clock.instant() >= start + Duration::from_secs(120);
    rig.run_until(STEPS, two_minutes).await;
    assert!(two_minutes(&rig));
    let during = asked(&rig) - before;
    assert!(
        (12..100).contains(&during),
        "{during} reads of a group of 283 in two minutes"
    );
}

/// A group's door at the node (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md,
/// part 5): Alice and Bob, both admins, open their group; Dave, a stranger
/// to it who lets no stranger in by himself, knocks by its reference and is
/// in within a few minutes — their nodes read the door and batch, the notary
/// ordering them — and his node takes the invitation. A second group of
/// hers, by request, keeps his application for her decision.
#[tokio::test(flavor = "current_thread")]
async fn a_stranger_knocks_at_a_groups_door_and_is_let_in_at_once_or_after_a_decision() {
    let (mut rig, g, _) = made_group(Setup {
        dave: true,
        dave_outside: true,
        ..Setup::default()
    })
    .await;
    let manual = rig.nodes[DAVE].command(
        "set_intro_policy",
        json!({"mode": "manual", "dailyLimit": 20, "allowed": []}),
    );
    assert!(manual.get("result").is_some(), "{manual}");
    let bob = network_id_of(&rig, BOB);
    for (op, change) in [
        ("admin", json!({"admins": [bob]})),
        ("open", json!({"access": "public"})),
    ] {
        let e = epoch_of(&rig, ALICE, &g).unwrap();
        let mut request = change;
        request["groupId"] = json!(g);
        request["operationId"] = json!(op);
        let answer = rig.nodes[ALICE].command("change_group", request);
        assert!(answer.get("result").is_some(), "{answer}");
        let next = |r: &Rig| {
            [ALICE, BOB, CAROL]
                .iter()
                .all(|n| epoch_of(r, *n, &g) == Some(e + 1))
        };
        rig.run_until(STEPS, next).await;
        assert!(next(&rig), "{op}: {:?}", rig.trace);
    }
    let group_ref = rig.nodes[ALICE].core.group(&g).unwrap().group_ref;
    // The owner's and the admin's nodes publish the door card; one finds
    // the group once it is there.
    let door_of =
        |group_ref: &str| -> [u8; 32] { hex::decode(group_ref).unwrap().try_into().unwrap() };
    let door = rig.nodes[DAVE]
        .core
        .door_mailbox(&door_of(&group_ref), rig.clock.wall());
    rig.run_until(STEPS, |r| keeping(r, &door, 1) >= QUORUM)
        .await;
    assert!(keeping(&rig, &door, 1) >= QUORUM, "{:?}", rig.trace);
    let knock = json!({"groupRef": group_ref, "note": "may I join?", "operationId": "knock-1"});
    let answer = when_cards_are_read(&mut rig, DAVE, "join_group", knock.clone()).await;
    assert_eq!(answer["result"]["groupId"], g.as_str(), "{answer}");
    // Asked again: the same application, not a second one paid.
    assert_eq!(
        rig.nodes[DAVE].command("join_group", knock)["result"]["messageId"],
        answer["result"]["messageId"]
    );
    let knocked = rig.clock.instant();
    let dave_in = |r: &Rig| {
        [ALICE, BOB, CAROL, DAVE].iter().all(|n| {
            members_of(r, *n, &g)
                .iter()
                .filter(|m| **m == network_id_of(r, DAVE))
                .count()
                == 1
        })
    };
    rig.run_until(STEPS, dave_in).await;
    assert!(dave_in(&rig), "{:?}", rig.trace);
    assert!(
        rig.clock.instant() <= knocked + Duration::from_secs(180),
        "let in {:?} after knocking",
        rig.clock.instant() - knocked
    );
    let settled = |r: &Rig| {
        let e = epoch_of(r, ALICE, &g);
        [BOB, CAROL, DAVE].iter().all(|n| epoch_of(r, *n, &g) == e)
    };
    rig.run_until(STEPS, settled).await;
    assert!(settled(&rig), "{:?}", rig.trace);
    // Any admin's node reads the door, not only the owner's.
    assert!(
        rig.nodes[BOB].mailbox_client.info()["door"]["admitted"].as_u64() >= Some(1),
        "{}",
        rig.nodes[BOB].mailbox_client.info()["door"]
    );
    let g_epoch = epoch_of(&rig, ALICE, &g);

    // A group by request: the application waits for Alice.
    let made = when_cards_are_read(
        &mut rig,
        ALICE,
        "create_group",
        json!({"name": "Club", "members": [bob], "operationId": "h-1"}),
    )
    .await;
    let h = made["result"]["id"].as_str().unwrap().to_owned();
    let by_request = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": h, "access": "request", "operationId": "h-door"}),
    );
    assert!(by_request.get("result").is_some(), "{by_request}");
    let request_mode = |r: &Rig| {
        r.nodes[ALICE]
            .core
            .group(&h)
            .is_ok_and(|info| info.access == "request")
    };
    rig.run_until(STEPS, request_mode).await;
    assert!(request_mode(&rig), "{:?}", rig.trace);
    let h_ref = rig.nodes[ALICE].core.group(&h).unwrap().group_ref;
    let h_door = rig.nodes[DAVE]
        .core
        .door_mailbox(&door_of(&h_ref), rig.clock.wall());
    rig.run_until(STEPS, |r| keeping(r, &h_door, 1) >= QUORUM)
        .await;
    assert!(keeping(&rig, &h_door, 1) >= QUORUM, "{:?}", rig.trace);
    let answer = when_cards_are_read(
        &mut rig,
        DAVE,
        "join_group",
        json!({"groupRef": h_ref, "note": "and to the club", "operationId": "knock-2"}),
    )
    .await;
    assert_eq!(answer["result"]["groupId"], h.as_str(), "{answer}");
    let application = answer["result"]["messageId"].as_str().unwrap().to_owned();
    rig.run_until(STEPS, |r| {
        r.nodes[DAVE]
            .core
            .swarm_receipts(&application)
            .unwrap()
            .is_some()
    })
    .await;
    // Her node reads the door about every 20 s.
    let stored = rig.clock.instant();
    let waiting = |r: &Rig| {
        r.nodes[ALICE]
            .core
            .door_requests(&h)
            .is_ok_and(|list| !list.is_empty())
    };
    rig.run_until(STEPS, waiting).await;
    assert!(waiting(&rig), "{:?}", rig.trace);
    assert!(
        rig.clock.instant() <= stored + Duration::from_secs(30),
        "read {:?} after storing",
        rig.clock.instant() - stored
    );
    // Nothing happens before her decision, however often the door is read.
    let e = epoch_of(&rig, ALICE, &h).unwrap();
    let start = rig.clock.instant();
    rig.run_until(STEPS, |r| {
        r.clock.instant() >= start + Duration::from_secs(150)
    })
    .await;
    assert_eq!(epoch_of(&rig, ALICE, &h), Some(e));
    assert!(!members_of(&rig, ALICE, &h).contains(&network_id_of(&rig, DAVE)));
    // Nor did the first group take more batches once Dave was in.
    assert_eq!(epoch_of(&rig, ALICE, &g), g_epoch);
    let listed = rig.nodes[ALICE].command("door_requests", json!({"groupId": h}));
    let list = listed["result"].as_array().unwrap();
    assert_eq!(list.len(), 1, "{listed}");
    assert_eq!(
        (list[0]["networkId"].as_str(), list[0]["note"].as_str()),
        (
            Some(network_id_of(&rig, DAVE).as_str()),
            Some("and to the club")
        ),
        "{listed}"
    );
    let decided = rig.nodes[ALICE].command(
        "door_decide",
        json!({"groupId": h, "requestId": list[0]["requestId"], "accept": true}),
    );
    assert!(decided.get("result").is_some(), "{decided}");
    let in_h = |r: &Rig| epoch_of(r, DAVE, &h).is_some();
    rig.run_until(STEPS, in_h).await;
    assert!(in_h(&rig), "{:?}", rig.trace);
}

/// Carol's node is off forty days, past what the mailboxes keep
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 7), while Alice and
/// Bob go on reading. Back, her node finds the group stale by itself — even
/// once its reading skipped the lost days; the group has no door, so it asks
/// the owner, and Alice's node gives Carol her place again at the next
/// batch, once, without anyone's action. Carol then reads the group again.
#[tokio::test(flavor = "current_thread")]
async fn a_member_away_longer_than_the_mailboxes_keep_takes_its_place_again_by_itself() {
    let (mut rig, g, _) = made_group(Setup::default()).await;
    // Books that outlast the absence, as a bought or granted one would.
    let until = rig.clock.wall() + 100 * PERIOD_SECONDS;
    for node in [ALICE, BOB, CAROL] {
        let book = [0xc0 + u8::try_from(node).unwrap(); 32];
        let key = rig.nodes[node].core.mailbox_book_account().unwrap();
        rig.nodes[node]
            .core
            .add_mailbox_book(book, 1_000, until)
            .unwrap();
        let terms = BookTerms {
            key,
            count: 1_000,
            valid_until: until,
        };
        for holder in 0..HOLDERS {
            rig.nodes[holder]
                .mailbox_holder
                .learn_book(book, terms)
                .unwrap();
        }
    }
    let e0 = epoch_of(&rig, ALICE, &g).unwrap();
    rig.paused.insert(CAROL);
    // Twenty days, then twenty more: Alice and Bob keep reading the group.
    for _ in 0..2 {
        rig.clock.advance(Duration::from_secs(20 * PERIOD_SECONDS));
        // Read through the day before yesterday: yesterday is still written to.
        let caught_up = |r: &Rig| {
            let closed = period(r.clock.wall()) - 2;
            [ALICE, BOB].iter().all(|n| {
                r.nodes[*n]
                    .core
                    .swarm_read_through(&g)
                    .unwrap()
                    .is_some_and(|through| through >= closed)
            })
        };
        rig.run_until(STEPS, caught_up).await;
        assert!(
            caught_up(&rig),
            "day {}: Alice {:?}, Bob {:?}",
            period(rig.clock.wall()),
            rig.nodes[ALICE].core.swarm_read_through(&g).unwrap(),
            rig.nodes[BOB].core.swarm_read_through(&g).unwrap(),
        );
    }
    let wall = rig.clock.wall();
    assert_eq!(rig.nodes[CAROL].core.stale_groups(wall).unwrap().len(), 1);
    assert!(rig.nodes[BOB].core.stale_groups(wall).unwrap().is_empty());
    rig.paused.remove(&CAROL);
    let back = rig.clock.instant();
    let in_again = |r: &Rig| {
        epoch_of(r, CAROL, &g).is_some_and(|e| e > e0)
            && epoch_of(r, CAROL, &g) == epoch_of(r, ALICE, &g)
            && epoch_of(r, BOB, &g) == epoch_of(r, ALICE, &g)
    };
    rig.run_until(STEPS, in_again).await;
    assert!(in_again(&rig), "{:?}", rig.trace);
    assert!(
        rig.clock.instant() <= back + Duration::from_secs(600),
        "back in {:?}",
        rig.clock.instant() - back
    );
    // Her place given back once: one commit, the same three members.
    assert_eq!(epoch_of(&rig, ALICE, &g), Some(e0 + 1));
    let mut everyone = vec![
        network_id_of(&rig, ALICE),
        network_id_of(&rig, BOB),
        network_id_of(&rig, CAROL),
    ];
    everyone.sort();
    assert_eq!(members_of(&rig, BOB, &g), everyone);
    assert!(
        rig.nodes[CAROL]
            .core
            .stale_groups(rig.clock.wall())
            .unwrap()
            .is_empty()
    );
    let wall = rig.clock.wall();
    rig.nodes[BOB]
        .core
        .send_message(&g, "Carol, welcome back", "m-back", wall)
        .unwrap();
    let read =
        |r: &Rig| incoming(r, CAROL, &g).last().map(String::as_str) == Some("Carol, welcome back");
    rig.run_until(STEPS, read).await;
    assert!(read(&rig), "{:?}", rig.trace);
    // Back, she does not ask again, not even the next day.
    rig.clock.advance(Duration::from_secs(PERIOD_SECONDS));
    let caught_up = |r: &Rig| {
        let closed = period(r.clock.wall()) - 2;
        [ALICE, BOB, CAROL].iter().all(|n| {
            r.nodes[*n]
                .core
                .swarm_read_through(&g)
                .unwrap()
                .is_some_and(|through| through >= closed)
        })
    };
    rig.run_until(STEPS, caught_up).await;
    assert!(caught_up(&rig), "{:?}", rig.trace);
    assert_eq!(epoch_of(&rig, ALICE, &g), Some(e0 + 1));
}

// --- a newcomer on the public testnet (2026-09-29) -----------------------------

/// Alice opens her group to everyone: its door card is published, and the
/// reference Dave knocks with.
async fn opened_lobby(rig: &mut Rig, g: &str) -> String {
    let e = epoch_of(rig, ALICE, g).unwrap();
    let opened = rig.nodes[ALICE].command(
        "change_group",
        json!({"groupId": g, "access": "public", "operationId": "lobby-open"}),
    );
    assert!(opened.get("result").is_some(), "{opened}");
    rig.run_until(STEPS, |r| epoch_of(r, ALICE, g) == Some(e + 1))
        .await;
    let group_ref = rig.nodes[ALICE].core.group(g).unwrap().group_ref;
    let reference: [u8; 32] = hex::decode(&group_ref).unwrap().try_into().unwrap();
    let door = rig.nodes[DAVE]
        .core
        .door_mailbox(&reference, rig.clock.wall());
    rig.run_until(STEPS, |r| keeping(r, &door, 1) >= QUORUM)
        .await;
    assert!(keeping(rig, &door, 1) >= QUORUM, "{:?}", rig.trace);
    group_ref
}

/// Dave is a member at Alice's and his node joined the group.
fn dave_joined(rig: &Rig, g: &str) -> bool {
    epoch_of(rig, DAVE, g).is_some()
        && members_of(rig, ALICE, g).contains(&network_id_of(rig, DAVE))
}

/// The testnet on 2026-09-29: ten holders on one host with eleven docker
/// bridges. A newcomer's preset names four of them; the other six it finds
/// only in the directory, at the addresses their records list. The records
/// list the routes the operators named, not the bridges their listeners also
/// bind, so the newcomer reaches every holder: its own card is stored at a
/// quorum, and knocking at the lobby's door it is let in and joins.
#[tokio::test(flavor = "current_thread")]
async fn a_newcomer_bootstrapped_to_four_holders_reaches_the_other_six_and_joins_the_lobby() {
    let (mut rig, g, _) = made_group(Setup {
        dave: true,
        dave_outside: true,
        sync: true,
        directory_from_chain: true,
        bridged_host: true,
        dave_bootstrap: Some(4),
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    rig.run_until(STEPS, |r| lists(r, DAVE, &all)).await;
    assert!(lists(&rig, DAVE, &all), "{:?}", rig.trace);
    for holder in all {
        let listed = &rig.nodes[DAVE].mailbox_client.directory[&rig.nodes[holder].own_commitment()];
        assert!(
            listed
                .addresses
                .iter()
                .all(|a| a.to_string().starts_with("/ip4/127.0.0.1/tcp/")),
            "holder {holder} is listed at {:?}",
            listed.addresses
        );
    }
    let card_stored =
        |r: &Rig| !r.nodes[DAVE].mailbox_client.info()["intro"]["published"].is_null();
    rig.run_until(STEPS, card_stored).await;
    assert!(
        card_stored(&rig),
        "{}",
        rig.nodes[DAVE].mailbox_client.info()["failureKinds"]
    );
    let group_ref = opened_lobby(&mut rig, &g).await;
    let knock =
        json!({"groupRef": group_ref, "note": "Hi, I'm new here", "operationId": "knock-1"});
    let answer = when_cards_are_read(&mut rig, DAVE, "join_group", knock).await;
    assert_eq!(answer["result"]["groupId"], g.as_str(), "{answer}");
    let application = answer["result"]["messageId"].as_str().unwrap().to_owned();
    let knocked = rig.clock.instant();
    let knock_stored = |r: &Rig| {
        r.nodes[DAVE]
            .core
            .swarm_receipts(&application)
            .unwrap()
            .is_some()
    };
    rig.run_until(STEPS, |r| knock_stored(r) && dave_joined(r, &g))
        .await;
    assert!(knock_stored(&rig), "the knock is stored at a quorum");
    assert!(dave_joined(&rig, &g), "{:?}", rig.trace);
    assert!(
        rig.clock.instant() <= knocked + Duration::from_secs(180),
        "joined {:?} after knocking",
        rig.clock.instant() - knocked
    );
}

/// A newcomer's agent buys a book and at once asks to join the lobby. No
/// holder has read that book yet: each refuses the newcomer's pass at first,
/// reads the book and lets it in seconds later. The lobby's door is still
/// found on that first ask, not answered `card_not_found`, which the agent
/// takes as final.
#[tokio::test(flavor = "current_thread")]
async fn a_newcomer_whose_book_no_holder_has_read_finds_the_lobby_door_on_the_first_ask() {
    let (mut rig, g, _) = made_group(Setup {
        dave: true,
        dave_outside: true,
        dave_bookless: true,
        ..Setup::default()
    })
    .await;
    let group_ref = opened_lobby(&mut rig, &g).await;
    let time = rig.clock.wall();
    let terms = BookTerms {
        key: rig.nodes[DAVE].core.mailbox_book_account().unwrap(),
        count: 1_000,
        valid_until: time + 30 * PERIOD_SECONDS,
    };
    let book = [0xbd; 32];
    rig.nodes[DAVE]
        .core
        .add_mailbox_book(book, terms.count, terms.valid_until)
        .unwrap();
    rig.chain.buy(book, record(&terms));
    let knock =
        json!({"groupRef": group_ref, "note": "Hi, I'm new here", "operationId": "knock-1"});
    let answer = when_cards_are_read(&mut rig, DAVE, "join_group", knock).await;
    assert_eq!(answer["result"]["groupId"], g.as_str(), "{answer}");
    // The holders did not know the book at first (the refusals may have
    // come to his card's store as well as to the lookup's reads).
    assert!(
        rig.nodes[DAVE].mailbox_client.info()["access"]["refused"]["unknown_book"].as_u64()
            >= Some(1),
        "{}",
        rig.nodes[DAVE].mailbox_client.info()["access"]
    );
    rig.run_until(STEPS, |r| dave_joined(r, &g)).await;
    assert!(dave_joined(&rig, &g), "{:?}", rig.trace);
}

/// Bought moments ago, Dave's book is still below the confirmations the
/// holders' chain reads wait for, for longer than a lookup lasts: they
/// refuse his pass the whole time. His ask ends within the lookup's minute
/// as one to repeat, not `card_not_found`; asked again once the purchase is
/// confirmed, it finds the door.
#[tokio::test(flavor = "current_thread")]
async fn a_newcomer_whose_book_the_holders_cannot_see_yet_is_told_to_ask_again() {
    let (mut rig, g, _) = made_group(Setup {
        dave: true,
        dave_outside: true,
        dave_bookless: true,
        ..Setup::default()
    })
    .await;
    let group_ref = opened_lobby(&mut rig, &g).await;
    let time = rig.clock.wall();
    let terms = BookTerms {
        key: rig.nodes[DAVE].core.mailbox_book_account().unwrap(),
        count: 1_000,
        valid_until: time + 30 * PERIOD_SECONDS,
    };
    let book = [0xbd; 32];
    rig.nodes[DAVE]
        .core
        .add_mailbox_book(book, terms.count, terms.valid_until)
        .unwrap();
    rig.chain.buy_unconfirmed(book, record(&terms));
    let knock =
        json!({"groupRef": group_ref, "note": "Hi, I'm new here", "operationId": "knock-1"});
    let asked = rig.clock.instant();
    let answer = when_cards_are_read(&mut rig, DAVE, "join_group", knock.clone()).await;
    assert_eq!(answer["error"]["code"], "network_unavailable", "{answer}");
    assert!(
        rig.clock.instant() <= asked + Duration::from_secs(75),
        "answered {:?} after asking",
        rig.clock.instant() - asked
    );
    // Confirmed now; a holder reads a book it found absent again within a
    // minute, so the agent may be told to ask again once more.
    rig.chain.confirm(&book);
    let mut answer = Value::Null;
    for _ in 0..3 {
        answer = when_cards_are_read(&mut rig, DAVE, "join_group", knock.clone()).await;
        if answer["error"]["code"] != "network_unavailable" {
            break;
        }
    }
    assert_eq!(answer["result"]["groupId"], g.as_str(), "{answer}");
}

/// Without a public address, a node tells others what its wildcard
/// listeners bind: on the testnet host (and on a desktop with IPv6), its
/// public addresses first, then the bridges, loopback and link-local last —
/// whatever their order as text.
#[tokio::test(flavor = "current_thread")]
async fn a_holder_without_a_public_address_advertises_its_public_interfaces_first() {
    let dir = TempDir::new().unwrap();
    let mut node = test_support::runtime(dir.path());
    let peer = *node.swarm.local_peer_id();
    let mut interfaces: Vec<String> = (0..=10)
        .map(|bridge| format!("ip4/10.0.{bridge}.1"))
        .collect();
    interfaces.extend(
        [
            "ip4/127.0.0.1",
            "ip4/51.91.126.3",
            "ip6/fe80::1",
            "ip6/::1",
            "ip6/2001:41d0:304:200::1",
        ]
        .map(str::to_owned),
    );
    for ip in &interfaces {
        for transport in ["tcp/4101", "udp/4101/quic-v1"] {
            node.event(SwarmEvent::NewListenAddr {
                listener_id: ListenerId::next(),
                address: format!("/{ip}/{transport}").parse().unwrap(),
            });
        }
    }
    let advertised = node.advertised();
    assert_eq!(advertised.len(), 8, "{advertised:?}");
    let public: BTreeSet<String> = ["ip4/51.91.126.3", "ip6/2001:41d0:304:200::1"]
        .iter()
        .flat_map(|ip| {
            ["tcp/4101", "udp/4101/quic-v1"]
                .map(|transport| format!("/{ip}/{transport}/p2p/{peer}"))
        })
        .collect();
    assert_eq!(
        advertised[..4].iter().cloned().collect::<BTreeSet<_>>(),
        public,
        "{advertised:?}"
    );
    assert!(
        advertised[4..].iter().all(|a| a.starts_with("/ip4/10.0.")),
        "{advertised:?}"
    );
}

/// A public address is an IP address others dial: a wildcard IP, port zero,
/// UDP without QUIC, a DNS name or a peer id is refused. The node tells
/// others exactly the routes named, not its listeners'.
#[tokio::test(flavor = "current_thread")]
async fn public_addresses_are_dialable_routes_told_instead_of_the_listeners() {
    let dir = TempDir::new().unwrap();
    let mut node = test_support::runtime(dir.path());
    let peer = *node.swarm.local_peer_id();
    for bad in [
        "/ip4/0.0.0.0/tcp/4101",
        "/ip4/51.91.126.3/tcp/0",
        "/ip4/51.91.126.3/udp/4101",
        "/dns4/kaikichat.com/tcp/4101",
        &format!("/ip4/51.91.126.3/tcp/4101/p2p/{peer}"),
    ] {
        assert!(
            node.set_public_addresses(&[bad.to_owned()]).is_err(),
            "{bad}"
        );
    }
    node.event(SwarmEvent::NewListenAddr {
        listener_id: ListenerId::next(),
        address: "/ip4/10.0.0.1/tcp/4101".parse().unwrap(),
    });
    node.set_public_addresses(&[
        "/ip4/51.91.126.3/udp/4101/quic-v1".to_owned(),
        "/ip4/51.91.126.3/tcp/4101".to_owned(),
    ])
    .unwrap();
    assert_eq!(
        node.advertised(),
        [
            format!("/ip4/51.91.126.3/udp/4101/quic-v1/p2p/{peer}"),
            format!("/ip4/51.91.126.3/tcp/4101/p2p/{peer}"),
        ]
    );
}

// --- operator payouts (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md) -------------------

use crate::runtime::chain::{PoolTerms, RegistryUnit, TicketClaim};
use crate::runtime::mailbox_holder::TicketState;

const POOL: [u8; 20] = [0x9e; 20];
/// Every draw wins.
const EVERY: [u8; 32] = [0xff; 32];

fn owner(holder: usize) -> [u8; 20] {
    [0xc0 + u8::try_from(holder).unwrap(); 20]
}

/// The shop and the pool as the payout scenarios read them. `chat` funds a
/// book that ends 30 days from now: a validity of 31 days makes yesterday
/// its purchase day, 30 days today. Prizes are $0.10 and tickets live 360
/// days; every draw wins. The registry lists each holder's unit (and the
/// spare's) at the holder's index, owned by `owner`.
fn payouts(rig: &Rig, validity_days: u64) {
    rig.chain.set_shop(crate::runtime::chain::ShopTerms {
        validity: validity_days * PERIOD_SECONDS,
        ..shop_terms()
    });
    rig.chain.set_pool(PoolTerms {
        address: POOL,
        win_threshold: EVERY,
        prize_usdc: 100_000,
        ticket_lifetime: 360 * PERIOD_SECONDS,
    });
    let registry = (0..rig.nodes.len().min(SPARE + 1))
        .filter(|h| *h != BOB && *h != ALICE)
        .map(|h| {
            let unit = rig.nodes[h].mailbox_holder.unit().unwrap_or(unit(h));
            (
                unit,
                RegistryUnit {
                    index: u32::try_from(h).unwrap(),
                    owner: owner(h),
                },
            )
        })
        .collect();
    rig.chain.set_registry(registry);
}

fn earnings(rig: &mut Rig, node: usize) -> Value {
    let answer = rig.nodes[node].command("operator_earnings", json!({}));
    answer["result"].clone()
}

/// `node`'s earnings once `check` holds, polled every few seconds.
async fn until_earnings(rig: &mut Rig, node: usize, check: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..120 {
        let shown = earnings(rig, node);
        if check(&shown) {
            return shown;
        }
        idle(rig, 5).await;
    }
    panic!("earnings never matched: {}", earnings(rig, node));
}

/// Tickets of `holder` in `state`.
fn tickets_in(rig: &Rig, holder: usize, state: TicketState) -> usize {
    rig.nodes[holder]
        .mailbox_holder
        .tickets()
        .unwrap()
        .iter()
        .filter(|t| t.state == state)
        .count()
}

/// The claims the pool took: sender and decoded call.
fn claims(rig: &Rig) -> Vec<(Account, u32, [u8; 32], Vec<TicketClaim>)> {
    rig.chain
        .transactions()
        .into_iter()
        .filter(|tx| tx.to == POOL && tx.data[..4] == [0xe4, 0x05, 0x93, 0xb5])
        .map(|tx| {
            let (unit, transport, tickets) = test_support::decode_claim(&tx.data);
            (tx.from, unit, transport, tickets)
        })
        .collect()
}

/// Every holder a paid stamp names keeps a ticket and shows its prize in
/// dollars once drawn; the recipient reads the named messages as any other.
/// The operator withdraws by hand: its node claims every won ticket in one
/// call from its receipt key, as its registry unit, naming where each stamp
/// names it; the pool takes the call as the contract does, the prizes show
/// as claimed and nothing is claimed twice.
#[tokio::test(flavor = "current_thread")]
async fn an_operator_withdraws_its_won_tickets_in_one_claim() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        directory_from_chain: true,
        silent_cards: true,
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    let everyone: Vec<usize> = (0..HOLDERS).chain([ALICE, BOB]).collect();
    rig.run_until(STEPS, |r| everyone.iter().all(|n| lists(r, *n, &all)))
        .await;
    payouts(&rig, 31);
    rig.chain.set_seed(period(rig.clock.wall()) - 1, [0x5e; 32]);
    let sent = send_alice(&mut rig, &c, "withdrawal", 3);
    let mailbox = to_bob(&rig, &c);
    rig.run_until(STEPS, |r| {
        sent.iter().all(|m| stored(r, ALICE, &m.id))
            && incoming(r, BOB, &c).len() == 3
            && (0..HOLDERS).all(|h| tickets_in(r, h, TicketState::Won) == 3)
    })
    .await;
    assert_eq!(
        incoming(&rig, BOB, &c),
        ["withdrawal 0", "withdrawal 1", "withdrawal 2"]
    );
    for h in 0..HOLDERS {
        let shown = earnings(&mut rig, h);
        assert_eq!(
            shown["won"],
            json!({"tickets": 3, "usdc": "300000"}),
            "holder {h}: {shown}"
        );
        assert_eq!(shown["drawing"]["tickets"], 0, "{shown}");
        assert_eq!(shown["claimed"], json!({"tickets": 0, "usdc": "0"}));
        assert_eq!(shown["held"], json!({"messages": 3, "paid": 3}));
        assert_eq!(shown["prizeUsdc"], "100000");
        assert_eq!(
            shown["account"],
            format!("0x{}", hex::encode(rig.nodes[h].mailbox_holder.account()))
        );
        assert!(shown["gasWei"].is_string(), "{shown}");
    }
    // Nothing leaves the node before the operator asks.
    idle(&mut rig, 120).await;
    assert!(claims(&rig).is_empty());
    const H: usize = 4;
    let started = rig.nodes[H].command("operator_withdraw", json!({}));
    assert_eq!(started["result"]["tickets"], 3, "{started}");
    assert_eq!(started["result"]["usdc"], "300000", "{started}");
    rig.run_until(STEPS, |r| {
        !claims(r).is_empty() && tickets_in(r, H, TicketState::Won) == 0
    })
    .await;
    let claimed = claims(&rig);
    assert_eq!(claimed.len(), 1, "{claimed:?}");
    let (from, index, transport, tickets) = &claimed[0];
    assert_eq!(*from, rig.nodes[H].mailbox_holder.account());
    assert_eq!(*index, u32::try_from(H).unwrap());
    assert_eq!(*transport, rig.nodes[H].own_transport_key());
    let members: Vec<_> = (0..HOLDERS)
        .map(|h| Member {
            commitment: rig.nodes[h].own_commitment(),
        })
        .collect();
    let swarm: Vec<[u8; 32]> = rendezvous(&mailbox, &members, SWARM_SIZE)
        .into_iter()
        .map(|m| m.commitment)
        .collect();
    let place = swarm
        .iter()
        .position(|u| *u == rig.nodes[H].own_commitment())
        .unwrap();
    assert_eq!(tickets.len(), 3);
    for ticket in tickets {
        assert_eq!((ticket.book, ticket.mailbox), (alice_book(), mailbox));
        assert_eq!(ticket.holders.to_vec(), swarm);
        assert_eq!(usize::from(ticket.position), place);
    }
    let shown = earnings(&mut rig, H);
    assert_eq!(shown["won"], json!({"tickets": 0, "usdc": "0"}), "{shown}");
    assert_eq!(shown["claimed"], json!({"tickets": 3, "usdc": "300000"}));
    assert_eq!(shown["owed"], "0", "{shown}");
    let again = rig.nodes[H].command("operator_withdraw", json!({}));
    assert_eq!(again["error"]["code"], "nothing_to_withdraw", "{again}");
    idle(&mut rig, 60).await;
    assert_eq!(claims(&rig).len(), 1);
    // Another holder's tickets are its own.
    assert_eq!(earnings(&mut rig, 5)["won"]["tickets"], 3);
}

/// A mailbox keeps naming the holders it named first: after a holder leaves
/// and the spare joins, a new message to the same mailbox still pays the
/// nine that stay, the leaver keeps what it earned, and the spare holds its
/// copies without a ticket.
#[tokio::test(flavor = "current_thread")]
async fn a_mailbox_keeps_paying_the_holders_it_named_after_one_left() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        spare: true,
        silent_cards: true,
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    payouts(&rig, 31);
    rig.chain.set_seed(period(rig.clock.wall()) - 1, [0x5e; 32]);
    let first = send_alice(&mut rig, &c, "before", 2);
    let mailbox = to_bob(&rig, &c);
    rig.run_until(STEPS, |r| {
        first.iter().all(|m| stored(r, ALICE, &m.id))
            && (0..HOLDERS).all(|h| tickets_in(r, h, TicketState::Won) == 2)
    })
    .await;
    const LEAVING: usize = 3;
    let mut all: Vec<usize> = (0..HOLDERS).chain([SPARE]).collect();
    all.extend([ALICE, BOB]);
    move_unit(&mut rig, &all, LEAVING, SPARE);
    let later = send_alice(&mut rig, &c, "after", 1);
    let stay: Vec<usize> = (0..HOLDERS).filter(|h| *h != LEAVING).collect();
    rig.run_until(STEPS, |r| {
        stored(r, ALICE, &later[0].id)
            && held(r, SPARE, &mailbox) == 3
            && stay
                .iter()
                .all(|h| tickets_in(r, *h, TicketState::Won) == 3)
    })
    .await;
    assert_eq!(held(&rig, SPARE, &mailbox), 3, "{:?}", rig.trace);
    for h in &stay {
        assert_eq!(earnings(&mut rig, *h)["won"]["tickets"], 3, "holder {h}");
    }
    assert_eq!(earnings(&mut rig, LEAVING)["won"]["tickets"], 2);
    let spare = earnings(&mut rig, SPARE);
    assert_eq!(spare["won"]["tickets"], 0, "{spare}");
    assert_eq!(spare["drawing"]["tickets"], 0, "{spare}");
    assert_eq!(spare["held"], json!({"messages": 3, "paid": 3}));
}

/// A book bought today is drawn by today's seed: holders keep its tickets
/// until the day is over; then one of them arms the day's seed, one
/// captures it from a later block, and every holder draws.
#[tokio::test(flavor = "current_thread")]
async fn holders_fix_the_seed_of_a_day_once_it_is_over_and_draw_by_it() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        silent_cards: true,
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    payouts(&rig, 30);
    let today = period(rig.clock.wall());
    let sent = send_alice(&mut rig, &c, "today", 1);
    rig.run_until(STEPS, |r| {
        sent.iter().all(|m| stored(r, ALICE, &m.id))
            && (0..HOLDERS).all(|h| tickets_in(r, h, TicketState::Drawing) == 1)
    })
    .await;
    // A holder whose RPC does not show today's purchase yet keeps its
    // ticket: the book was bought today.
    rig.chain.lag(3);
    idle(&mut rig, 120).await;
    assert_eq!(tickets_in(&rig, 3, TicketState::Drawing), 1);
    rig.chain.catch_up(3);
    // Up to a minute before midnight nothing is armed.
    let midnight = (today + 1) * PERIOD_SECONDS;
    rig.clock
        .advance(Duration::from_secs(midnight - 60 - rig.clock.wall()));
    idle(&mut rig, 30).await;
    for h in 0..HOLDERS {
        let shown = earnings(&mut rig, h);
        assert_eq!(shown["drawing"]["tickets"], 1, "holder {h}: {shown}");
        assert_eq!(shown["won"]["tickets"], 0, "holder {h}: {shown}");
    }
    assert!(rig.chain.transactions().is_empty());
    rig.clock.advance(Duration::from_secs(60));
    rig.run_until(STEPS, |r| {
        (0..HOLDERS).all(|h| tickets_in(r, h, TicketState::Won) == 1)
    })
    .await;
    assert!(
        rig.chain.seed_state(today).seed.is_some(),
        "{:?}",
        rig.trace
    );
    let txs = rig.chain.transactions();
    let accounts: BTreeMap<Account, usize> = holders(&rig);
    let arms: Vec<_> = txs
        .iter()
        .filter(|tx| tx.data[..4] == [0xc9, 0xbc, 0x7a, 0x8c])
        .collect();
    let captures: Vec<_> = txs
        .iter()
        .filter(|tx| tx.data[..4] == [0x06, 0x28, 0x82, 0x9b])
        .collect();
    assert_eq!((arms.len(), captures.len()), (1, 1), "{txs:?}");
    for tx in arms.iter().chain(&captures) {
        assert_eq!(tx.to, POOL);
        assert!(
            accounts.contains_key(&tx.from),
            "sent by a holder's receipt key"
        );
        assert_eq!(
            &tx.data[4..],
            &crate::runtime::chain::arm_seed_calldata(today)[4..]
        );
    }
    for h in 0..HOLDERS {
        assert_eq!(earnings(&mut rig, h)["won"]["tickets"], 1, "holder {h}");
    }
}

/// A withdrawal that cannot go through leaves the tickets to claim: without
/// ETH for gas nothing is sent and the operator sees the account to fund; a
/// ticket the pool refuses (its place was paid already) is left out and the
/// rest are claimed; what the pool credits but cannot pay yet shows as owed
/// and is paid out by the next withdrawal.
#[tokio::test(flavor = "current_thread")]
async fn withdrawals_that_cannot_go_through_leave_what_is_earned() {
    let Chat {
        mut rig,
        conversation: c,
        ..
    } = chat(Setup {
        sync: true,
        directory_from_chain: true,
        silent_cards: true,
        books_on_chain: true,
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    let everyone: Vec<usize> = (0..HOLDERS).chain([ALICE, BOB]).collect();
    rig.run_until(STEPS, |r| everyone.iter().all(|n| lists(r, *n, &all)))
        .await;
    payouts(&rig, 31);
    rig.chain.set_seed(period(rig.clock.wall()) - 1, [0x5e; 32]);
    let sent = send_alice(&mut rig, &c, "unsuccessfully", 3);
    const H: usize = 6;
    rig.run_until(STEPS, |r| {
        sent.iter().all(|m| stored(r, ALICE, &m.id)) && tickets_in(r, H, TicketState::Won) == 3
    })
    .await;
    let account = rig.nodes[H].mailbox_holder.account();
    // No ETH for gas.
    rig.chain.set_gas(account, 0);
    rig.nodes[H].command("operator_withdraw", json!({}));
    let shown = until_earnings(&mut rig, H, |e| e["withdrawal"]["state"] == "failed").await;
    assert_eq!(shown["withdrawal"]["error"], "no_gas", "{shown}");
    assert_eq!(shown["account"], format!("0x{}", hex::encode(account)));
    assert_eq!(shown["won"]["tickets"], 3);
    assert!(claims(&rig).is_empty());
    // Funded; one ticket's place was paid already, and the pool is short.
    rig.chain.set_gas(account, 1_000_000_000_000_000_000);
    let first = rig.nodes[H].mailbox_holder.tickets().unwrap()[0]
        .claim
        .clone();
    rig.chain.pay_place(&first);
    rig.chain.set_short(true);
    rig.nodes[H].command("operator_withdraw", json!({}));
    rig.run_until(STEPS, |r| {
        !claims(r).is_empty() && tickets_in(r, H, TicketState::Won) == 0
    })
    .await;
    let claimed = claims(&rig);
    assert_eq!(claimed.len(), 1, "{claimed:?}");
    assert_eq!(claimed[0].3.len(), 2);
    assert!(!claimed[0].3.contains(&first));
    let shown = earnings(&mut rig, H);
    assert_eq!(
        shown["claimed"],
        json!({"tickets": 2, "usdc": "200000"}),
        "{shown}"
    );
    assert_eq!(shown["refused"]["tickets"], 1, "{shown}");
    assert_eq!(shown["owed"], "200000", "{shown}");
    // The pool is funded again: the next withdrawal pays out what is owed.
    rig.chain.set_short(false);
    let started = rig.nodes[H].command("operator_withdraw", json!({}));
    assert_eq!(started["result"]["owed"], "200000", "{started}");
    until_earnings(&mut rig, H, |e| e["owed"] == "0").await;
    let payouts: Vec<_> = rig
        .chain
        .transactions()
        .into_iter()
        .filter(|tx| tx.data[..4] == [0xf6, 0xa0, 0x81, 0x39])
        .collect();
    assert_eq!(payouts.len(), 1);
    assert_eq!(payouts[0].from, account);
}

/// A book the holders learned before the network moved to another shop still
/// pays for messages, but the pool pays prizes only for books its own shop
/// sold: holders drop such tickets once the shop is read, fix no seed for
/// them and never draw them, while a book of the shop wins by the same seed.
/// A chain that does not answer drops nothing.
#[tokio::test(flavor = "current_thread")]
async fn stamps_of_a_book_the_pools_shop_never_sold_earn_nothing() {
    // Holders know both books (as learned from the former shop); the
    // chain's current shop sold neither yet.
    let Chat {
        mut rig,
        conversation: c,
        carol,
        terms,
    } = chat(Setup {
        sync: true,
        silent_cards: true,
        carol: true,
        ..Setup::default()
    })
    .await;
    payouts(&rig, 31);
    idle(&mut rig, 5).await;
    for h in 0..HOLDERS {
        assert_eq!(
            earnings(&mut rig, h)["prizeUsdc"],
            "100000",
            "holder {h}: terms read"
        );
    }
    let yesterday = period(rig.clock.wall()) - 1;
    let tickets = |r: &Rig, h: usize| r.nodes[h].mailbox_holder.tickets().unwrap();
    // While the chain does not answer, nothing earned is dropped.
    rig.chain.set_down(true);
    let old = send_alice(&mut rig, &c, "the old way", 2);
    rig.run_until(STEPS, |r| {
        old.iter().all(|m| stored(r, ALICE, &m.id))
            && (0..HOLDERS).all(|h| tickets(r, h).len() == 2)
    })
    .await;
    idle(&mut rig, 120).await;
    for h in 0..HOLDERS {
        assert_eq!(earnings(&mut rig, h)["drawing"]["tickets"], 2, "holder {h}");
    }
    // Read: the shop never sold Alice's book; its purchase day is over.
    rig.chain.set_down(false);
    idle(&mut rig, 120).await;
    for h in 0..HOLDERS {
        assert!(
            tickets(&rig, h).is_empty(),
            "holder {h}: {:?}",
            tickets(&rig, h)
        );
        assert_eq!(
            earnings(&mut rig, h)["held"],
            json!({"messages": 2, "paid": 2})
        );
    }
    assert!(
        rig.chain.transactions().is_empty(),
        "{:?}",
        rig.chain.transactions()
    );
    // The shop sold Bob's book, and yesterday's seed is known: Bob's stamp
    // wins; Carol's, of a book the shop never sold and nobody read yet, sent
    // after it with the seed at hand, are never drawn.
    let bob_book = [0xb0 + u8::try_from(BOB).unwrap(); 32];
    let (_, bob_terms) = terms.iter().find(|(book, _)| *book == bob_book).unwrap();
    rig.chain.buy(bob_book, record(bob_terms));
    rig.chain.set_seed(yesterday, [0x5e; 32]);
    let time = rig.clock.wall();
    let reply = rig.nodes[BOB]
        .core
        .send_message(&c, "the new way", "b-new", time)
        .unwrap();
    rig.run_until(STEPS, |r| {
        stored(r, BOB, &reply.id) && (0..HOLDERS).all(|h| tickets_in(r, h, TicketState::Won) == 1)
    })
    .await;
    let carol = carol.unwrap();
    let later: Vec<_> = (0..2)
        .map(|n| {
            rig.nodes[CAROL]
                .core
                .send_message(&carol, &format!("Carol {n}"), &format!("c-{n}"), time)
                .unwrap()
        })
        .collect();
    rig.run_until(STEPS, |r| later.iter().all(|m| stored(r, CAROL, &m.id)))
        .await;
    idle(&mut rig, 60).await;
    for h in 0..HOLDERS {
        let shown = earnings(&mut rig, h);
        assert_eq!(shown["won"]["tickets"], 1, "holder {h}: {shown}");
        assert_eq!(shown["drawing"]["tickets"], 0, "holder {h}: {shown}");
        assert_eq!(shown["held"]["paid"], 5, "holder {h}: {shown}");
        assert!(
            tickets(&rig, h).iter().all(|t| t.claim.book == bob_book),
            "holder {h}"
        );
    }
    assert!(rig.chain.transactions().is_empty());
}

/// A newcomer's agent asks by id the moment its node starts, before the node
/// has read the registry and pulled the holders' records: it is asked to
/// wait (`card_pending`, which the command line waits out), not told the
/// network is unavailable; once the directory is known the same ask goes
/// through.
#[tokio::test(flavor = "current_thread")]
async fn an_ask_before_the_directory_is_read_is_asked_to_wait() {
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        directory_from_chain: true,
        strangers: true,
        ..Setup::default()
    })
    .await;
    assert!(rig.nodes[ALICE].mailbox_client.directory.is_empty());
    let bob = network_id_of(&rig, BOB);
    let ask = json!({"networkId": bob, "name": "Bob", "operationId": "ask-1"});
    let first = rig.nodes[ALICE].command("request_contact", ask.clone());
    assert_eq!(first["error"]["code"], "card_pending", "{first}");
    let card = rig.nodes[BOB]
        .core
        .own_intro_mailbox(rig.clock.wall())
        .unwrap();
    let all: Vec<usize> = (0..HOLDERS).collect();
    rig.run_until(STEPS, |r| {
        lists(r, ALICE, &all) && keeping(r, &card, 1) >= QUORUM
    })
    .await;
    assert!(lists(&rig, ALICE, &all), "{:?}", rig.trace);
    let answer = when_cards_are_read(&mut rig, ALICE, "request_contact", ask).await;
    assert!(answer["result"]["conversationId"].is_string(), "{answer}");
}

// --- identity penalties (Docs/V1_IDENTITY_PENALTIES_2026_09_30.md) ------------

/// Holders read the issuer's rules for each grant's day and learn the
/// grants, first seen now.
fn hold_grants(rig: &mut Rig, holders: &[usize], grants: &[&GrantBook]) {
    let time = rig.clock.wall();
    for holder in holders {
        let service = &mut rig.nodes[*holder].mailbox_holder;
        for grant in grants {
            service.learn_grant_day(
                grant.server,
                grant.day,
                crate::runtime::chain::GrantDay {
                    active: true,
                    cap_coins: 1_000,
                    book_size: 100,
                    max_validity_days: 30,
                    today: grant.day,
                },
            );
            service.learn_grant(grant, time, time).unwrap();
        }
    }
}

/// Holder 0 sees slot 0 of `book` spent on two operations.
fn spend_twice(rig: &mut Rig, book: [u8; 32], key: &agentic_mailbox_swarm::stamp::BookKey) {
    let time = rig.clock.wall();
    let (m1, m2) = ([0x21; 32], [0x22; 32]);
    let service = &mut rig.nodes[0].mailbox_holder;
    service
        .store(
            m1,
            period(WALL),
            b"one",
            &paid(book, 0, &m1, b"one", key),
            time,
        )
        .unwrap();
    assert_eq!(
        service.store(
            m2,
            period(WALL),
            b"two",
            &paid(book, 0, &m2, b"two", key),
            time
        ),
        Err(mailbox_holder::Refusal::Conflict)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_double_spent_grant_is_reported_and_its_owners_revoked_grants_pay_nowhere() {
    use agentic_grant_book::GrantRevocation;
    use agentic_mailbox_swarm::proof::SenderEquivocation;
    let Chat { mut rig, .. } = chat(Setup {
        sync: true,
        ..Setup::default()
    })
    .await;
    let all: Vec<usize> = (0..HOLDERS).collect();
    let server = test_support::FakeIdentity::default();
    server.set_issuer(identity_server().account());
    for holder in &all {
        rig.nodes[*holder].set_identity(server.server());
    }
    // Mallory's identity got two grants, and her key a grant of another
    // issuer too; every holder learned all three.
    let key = agentic_mailbox_swarm::stamp::BookKey::from_bytes(&[0x6d; 32]).unwrap();
    let today = period(WALL);
    let spent = grant(key.account(), today, 3);
    let other = grant(key.account(), today, 4);
    let foreign = GrantBook::issue(spent.terms(), &SecpKey::from_secret(&[0x32; 32]).unwrap());
    hold_grants(&mut rig, &all, &[&spent, &other, &foreign]);
    let blocked = |r: &Rig, book: [u8; 32]| {
        all.iter().all(|h| {
            r.nodes[*h]
                .mailbox_holder
                .blocked_books()
                .unwrap()
                .contains(&book)
        })
    };
    // The identity server is down when holder 0 sees a slot of each of
    // the first and the foreign grant spent twice.
    server.set_down(true);
    spend_twice(&mut rig, spent.id(), &key);
    spend_twice(&mut rig, foreign.id(), &key);
    rig.run_until(STEPS, |r| {
        blocked(r, spent.id()) && blocked(r, foreign.id())
    })
    .await;
    assert!(blocked(&rig, foreign.id()), "{:?}", rig.trace);
    // Holders retry about once a minute while it is down, no faster.
    let calls = server.report_calls();
    idle(&mut rig, 120).await;
    assert!(server.report_calls() - calls <= HOLDERS * 2 * 3);
    assert!(server.reports().is_empty());
    // Back up, it hears of the double spend from every holder, once each,
    // bans the identity and revokes both its grants; each holder reads the
    // revocations as soon as its report is answered. The foreign grant's
    // report is refused, and a refusal is final.
    let at = rig.clock.wall();
    server.revoke_on_report(
        [&spent, &other]
            .map(|grant| GrantRevocation::issue(grant, at, &identity_server()))
            .to_vec(),
    );
    server.set_down(false);
    let (up, reads) = (rig.clock.instant(), server.revocation_reads());
    let done = |r: &Rig| {
        server.reports().len() >= HOLDERS
            && server.refused().len() >= HOLDERS
            && blocked(r, other.id())
    };
    rig.run_until(STEPS, done).await;
    assert!(done(&rig), "{:?}", rig.trace);
    assert!(rig.clock.instant() - up <= Duration::from_secs(120));
    idle(&mut rig, 120).await;
    let reports = server.reports();
    assert_eq!(reports.len(), HOLDERS);
    for (grant, first, second) in reports {
        assert_eq!(grant, spent);
        let proof = SenderEquivocation { first, second };
        assert_eq!(proof.verify(&NETWORK_DOMAIN, &key.account()), Ok(()));
    }
    assert_eq!(server.refused(), vec![foreign.clone(); HOLDERS]);
    // Reads stay paced: the round the revocations came in, and no more.
    assert!(server.revocation_reads() - reads <= HOLDERS * 5);
    // The other grant, never spent twice, pays at no holder either.
    let m3 = [0x23; 32];
    for holder in all {
        assert_eq!(
            rig.nodes[holder].mailbox_holder.store(
                m3,
                period(WALL),
                b"three",
                &paid(other.id(), 0, &m3, b"three", &key),
                rig.clock.wall()
            ),
            Err(mailbox_holder::Refusal::Blocked),
            "holder {holder}"
        );
    }
}
