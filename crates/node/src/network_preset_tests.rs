//! The signed network preset (Docs/V1_NETWORK_PRESET_2026_09_28_RU.md)
//! against a local HTTP server: what a starting daemon takes from it, what it
//! keeps, what it only offers, and what it refuses.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::host::DaemonFlags;
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

const SEED: [u8; 32] = [7; 32];
const OTHER_SEED: [u8; 32] = [9; 32];

/// What the server answers: a status, extra header lines, a body, and
/// whether it says the body's length.
#[derive(Clone)]
struct Reply {
    status: u16,
    headers: String,
    body: String,
    length: bool,
}

/// A server answering every request with the reply set last.
struct Server {
    url: String,
    reply: Arc<Mutex<Reply>>,
    hits: Arc<AtomicUsize>,
}

impl Server {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/network.json", listener.local_addr().unwrap());
        let reply = Arc::new(Mutex::new(Reply {
            status: 404,
            headers: String::new(),
            body: String::new(),
            length: true,
        }));
        let hits = Arc::new(AtomicUsize::new(0));
        let (answer, count) = (reply.clone(), hits.clone());
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                    request.push(byte[0]);
                }
                count.fetch_add(1, Ordering::SeqCst);
                let reply = answer.lock().unwrap().clone();
                let length = if reply.length {
                    format!("Content-Length: {}\r\n", reply.body.len())
                } else {
                    String::new()
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {} X\r\nContent-Type: application/json\r\n{}{length}Connection: close\r\n\r\n{}",
                    reply.status, reply.headers, reply.body
                );
            }
        });
        Self { url, reply, hits }
    }
    fn serve(&self, status: u16, body: String) {
        *self.reply.lock().unwrap() = Reply {
            status,
            headers: String::new(),
            body,
            length: true,
        };
    }
    fn set(&self, reply: Reply) {
        *self.reply.lock().unwrap() = reply;
    }
    fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
    fn source(&self) -> PresetSource {
        PresetSource::new(&self.url, &public_key(&SEED)).unwrap()
    }
}

/// A source nobody answers.
fn gone() -> PresetSource {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/network.json", listener.local_addr().unwrap());
    PresetSource::new(&url, &public_key(&SEED)).unwrap()
}

/// A bootstrap route of a peer made from `n`.
fn route(n: u8) -> String {
    let key = libp2p::identity::Keypair::ed25519_from_bytes([n; 32]).unwrap();
    format!(
        "/ip4/127.0.0.1/udp/{}/quic-v1/p2p/{}",
        4100 + u16::from(n),
        key.public().to_peer_id()
    )
}

fn address(n: u8) -> String {
    format!("0x{}", hex::encode([n; 20]))
}

fn preset(network: &str, serial: u64) -> Preset {
    Preset {
        network: network.into(),
        name: format!("Network {network}"),
        serial,
        min_version: None,
        bootstrap: (1..=6).map(route).collect(),
        chain_rpc: Some(format!("https://rpc.{network}.example")),
        chain_id: Some(84532),
        book_shop: Some(address(1)),
        grant_issuer: Some(address(2)),
        registry: Some(address(3)),
        chain_confirmations: Some(5),
        operator_pool: None,
        identity_server: Some(format!("https://id.{network}.example")),
        directory: None,
        directory_key: None,
        welcome: None,
        release: None,
    }
}

/// A welcome agent and its lobby, as a preset names them.
fn welcome() -> Welcome {
    Welcome {
        agent: format!("ain1{}", "ab".repeat(32)),
        name: "Net welcome".into(),
        lobby: "cd".repeat(32),
        lobby_name: "Net lobby".into(),
    }
}

fn signed(preset: &Preset) -> String {
    sign(&serde_json::to_string(preset).unwrap(), &SEED).unwrap()
}

/// An envelope made by hand, as the doc says: `text` signed with `seed`
/// over `kaiki-network-preset/v1\n` and the text, whatever it says.
fn envelope(text: &str, seed: &[u8; 32]) -> String {
    use ed25519_dalek::{Signer, SigningKey};
    let mut message = b"kaiki-network-preset/v1\n".to_vec();
    message.extend_from_slice(text.as_bytes());
    let signature = SigningKey::from_bytes(seed).sign(&message);
    serde_json::json!({"preset": text, "signature": hex::encode(signature.to_bytes())}).to_string()
}

fn profile() -> TempDir {
    let dir = tempfile::Builder::new()
        .prefix("ain-preset-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

/// The flags a preset gives: all of its network, up to four of its
/// bootstrap routes.
fn assert_flags_of(flags: &DaemonFlags, preset: &Preset) {
    assert_eq!(flags.chain_rpc, preset.chain_rpc);
    assert_eq!(flags.chain_id, preset.chain_id);
    assert_eq!(flags.book_shop, preset.book_shop);
    assert_eq!(flags.grant_issuer, preset.grant_issuer);
    assert_eq!(flags.registry, preset.registry);
    assert_eq!(flags.chain_confirmations, preset.chain_confirmations);
    assert_eq!(flags.identity_server, preset.identity_server);
    assert_eq!(flags.bootstrap.len(), preset.bootstrap.len().min(4));
    let distinct: BTreeSet<_> = flags.bootstrap.iter().collect();
    assert_eq!(distinct.len(), flags.bootstrap.len());
    assert!(flags.bootstrap.iter().all(|r| preset.bootstrap.contains(r)));
}

fn without_network(flags: &DaemonFlags) -> bool {
    flags.bootstrap.is_empty()
        && flags.chain_rpc.is_none()
        && flags.chain_id.is_none()
        && flags.book_shop.is_none()
        && flags.grant_issuer.is_none()
        && flags.registry.is_none()
        && flags.chain_confirmations.is_none()
        && flags.identity_server.is_none()
}

#[tokio::test]
async fn a_new_profile_takes_the_signed_preset_and_keeps_it_when_the_server_fails() {
    let server = Server::start();
    let dir = profile();
    let a = preset("net-a", 1);
    server.serve(200, signed(&a));
    let flags = resolve(dir.path(), Some(&server.source())).await;
    assert_flags_of(&flags, &a);
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (
            seen.source,
            seen.state,
            seen.network.as_deref(),
            seen.name.as_deref(),
            seen.serial
        ),
        (
            Source::Preset,
            State::Current,
            Some("net-a"),
            Some("Network net-a"),
            Some(1)
        )
    );
    assert!(seen.checked_at.is_some() && seen.error.is_none() && seen.offered.is_none());
    let json = serde_json::to_value(&seen).unwrap();
    for key in [
        "source",
        "state",
        "network",
        "name",
        "serial",
        "checkedAt",
        "offered",
        "required",
        "error",
        "welcome",
    ] {
        assert!(json.get(key).is_some(), "{key} in {json}");
    }
    let kept = dir.path().join("network-preset.json");
    assert_eq!(
        fs::metadata(&kept).unwrap().permissions().mode() & 0o777,
        0o600
    );

    // A newer preset of the same network, but not with 200: not taken.
    let mut newer = preset("net-a", 2);
    newer.identity_server = Some("https://id.newer.example".into());
    server.serve(503, signed(&newer));
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &a);
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.network.as_deref()),
        (State::Cached, Some("net-a"))
    );
    assert!(seen.error.is_some());

    // Nor behind a redirect.
    let other = Server::start();
    other.serve(200, signed(&newer));
    server.set(Reply {
        status: 302,
        headers: format!("Location: {}\r\n", other.url),
        body: String::new(),
        length: true,
    });
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &a);
    assert_eq!(other.hits(), 0);

    assert_flags_of(&resolve(dir.path(), Some(&gone())).await, &a);
    assert_eq!(status(dir.path(), Some(&gone())).state, State::Cached);
}

#[tokio::test]
async fn the_preset_is_the_signed_text_and_unknown_fields_are_passed_over() {
    let server = Server::start();
    let dir = profile();
    let mut value = serde_json::to_value(preset("net-a", 1)).unwrap();
    value["futureField"] = serde_json::json!({"anything": [1, 2]});
    // Nor does a field a later version adds to `welcome` lose the network.
    value["welcome"] = serde_json::to_value(welcome()).unwrap();
    value["welcome"]["futureField"] = serde_json::json!(true);
    let text = format!("  {}\n", serde_json::to_string_pretty(&value).unwrap());
    server.serve(200, envelope(&text, &SEED));
    assert_flags_of(
        &resolve(dir.path(), Some(&server.source())).await,
        &preset("net-a", 1),
    );
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.welcome),
        (State::Current, Some(welcome()))
    );
    // `sign` makes the same envelope as the doc's recipe.
    let payload = serde_json::to_string(&preset("net-a", 1)).unwrap();
    let ours: serde_json::Value = serde_json::from_str(&sign(&payload, &SEED).unwrap()).unwrap();
    let recipe: serde_json::Value = serde_json::from_str(&envelope(&payload, &SEED)).unwrap();
    assert_eq!(ours, recipe);
}

#[tokio::test]
async fn a_new_profile_without_the_server_starts_without_a_network() {
    let dir = profile();
    let flags = resolve(dir.path(), Some(&gone())).await;
    assert!(without_network(&flags), "{flags:?}");
    let seen = status(dir.path(), Some(&gone()));
    assert_eq!(
        (seen.source, seen.state),
        (Source::Preset, State::Unavailable)
    );
    assert!(seen.network.is_none() && seen.error.is_some());

    // A server that never answers is given up on.
    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/network.json", silent.local_addr().unwrap());
    let held = thread::spawn(move || {
        let (stream, _) = silent.accept().unwrap();
        thread::sleep(Duration::from_secs(3));
        drop(stream);
    });
    let slow = PresetSource::new(&url, &public_key(&SEED))
        .unwrap()
        .with_timeout(Duration::from_millis(200));
    let dir = profile();
    let started = Instant::now();
    let flags = resolve(dir.path(), Some(&slow)).await;
    assert!(without_network(&flags));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(status(dir.path(), Some(&slow)).state, State::Unavailable);
    drop(held);
}

#[tokio::test]
async fn presets_not_signed_by_the_key_or_not_valid_are_refused() {
    let server = Server::start();
    let a = preset("net-a", 1);
    let text = serde_json::to_string(&a).unwrap();
    let tampered = {
        let good: serde_json::Value = serde_json::from_str(&signed(&a)).unwrap();
        let text = good["preset"]
            .as_str()
            .unwrap()
            .replace("https://id.net-a.example", "https://id.evil.example");
        serde_json::json!({"preset": text, "signature": good["signature"]}).to_string()
    };
    let invalid = |change: fn(&mut Preset)| {
        let mut preset = preset("net-a", 1);
        change(&mut preset);
        envelope(&serde_json::to_string(&preset).unwrap(), &SEED)
    };
    let refused = [
        ("another key", envelope(&text, &OTHER_SEED)),
        ("a changed preset", tampered),
        ("not an envelope", "<html>hello</html>".to_owned()),
        (
            "no signature",
            serde_json::json!({"preset": text}).to_string(),
        ),
        (
            "identity server over plain http",
            invalid(|p| p.identity_server = Some("http://id.example".into())),
        ),
        (
            "chain rpc over plain http",
            invalid(|p| p.chain_rpc = Some("http://rpc.example".into())),
        ),
        (
            "a host that starts like loopback",
            invalid(|p| p.identity_server = Some("http://127.0.0.1.evil.example".into())),
        ),
        (
            "loopback as a user name",
            invalid(|p| p.identity_server = Some("http://localhost@evil.example".into())),
        ),
        (
            "a route without a peer id",
            invalid(|p| p.bootstrap = vec!["/ip4/1.2.3.4/udp/1/quic-v1".into()]),
        ),
        (
            "a route twice",
            invalid(|p| p.bootstrap = vec![route(1), route(1)]),
        ),
        ("no routes", invalid(|p| p.bootstrap.clear())),
        (
            "seventeen routes",
            invalid(|p| p.bootstrap = (1..=17).map(route).collect()),
        ),
        (
            "a network id with capitals",
            invalid(|p| p.network = "Net-A".into()),
        ),
        (
            "a network id of 65 characters",
            invalid(|p| p.network = "a".repeat(65)),
        ),
        (
            "a name of 81 characters",
            invalid(|p| p.name = "n".repeat(81)),
        ),
        (
            "a contract that is not an address",
            invalid(|p| p.book_shop = Some("0x1234".into())),
        ),
        (
            "an app version that is not X.Y.Z",
            invalid(|p| p.min_version = Some("soon".into())),
        ),
        (
            "more than 64 KiB",
            format!("{}{}", signed(&a), " ".repeat(65 * 1024)),
        ),
        (
            "two routes of one peer",
            invalid(|p| {
                p.bootstrap = vec![route(1), route(1).replace("/udp/4101/quic-v1", "/tcp/4101")]
            }),
        ),
        (
            "a chain without its registry",
            invalid(|p| p.registry = None),
        ),
        (
            "confirmations without a chain",
            invalid(|p| {
                p.chain_rpc = None;
                p.chain_id = None;
                p.book_shop = None;
                p.grant_issuer = None;
                p.registry = None;
            }),
        ),
        (
            "no confirmations",
            invalid(|p| p.chain_confirmations = Some(0)),
        ),
        ("a serial below the first", invalid(|p| p.serial = 0)),
        (
            "a directory over plain http",
            invalid(|p| {
                p.directory = Some("http://directory.example".into());
                p.directory_key = Some("ab".repeat(32));
            }),
        ),
        (
            "a directory key that is not 32 bytes",
            invalid(|p| {
                p.directory = Some("https://directory.example".into());
                p.directory_key = Some("ab".repeat(31));
            }),
        ),
        (
            "a directory key without a directory",
            invalid(|p| p.directory_key = Some("ab".repeat(32))),
        ),
        (
            "a welcome agent with capital hex",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    agent: format!("ain1{}", "AB".repeat(32)),
                    ..welcome()
                })
            }),
        ),
        (
            "a welcome agent that is not a network id",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    agent: format!("ain2{}", "ab".repeat(32)),
                    ..welcome()
                })
            }),
        ),
        (
            "a welcome agent id too short",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    agent: format!("ain1{}", "ab".repeat(31)),
                    ..welcome()
                })
            }),
        ),
        (
            "a lobby that is not 32 bytes of hex",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    lobby: "cd".repeat(31),
                    ..welcome()
                })
            }),
        ),
        (
            "a lobby of capital hex",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    lobby: "CD".repeat(32),
                    ..welcome()
                })
            }),
        ),
        (
            "a welcome agent without a name",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    name: String::new(),
                    ..welcome()
                })
            }),
        ),
        (
            "a lobby name of 81 characters",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    lobby_name: "n".repeat(81),
                    ..welcome()
                })
            }),
        ),
        (
            "a lobby without a name",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    lobby_name: String::new(),
                    ..welcome()
                })
            }),
        ),
        (
            "a welcome agent name of 81 characters",
            invalid(|p| {
                p.welcome = Some(Welcome {
                    name: "n".repeat(81),
                    ..welcome()
                })
            }),
        ),
    ];
    for (case, body) in refused {
        let dir = profile();
        server.serve(200, body);
        let flags = resolve(dir.path(), Some(&server.source())).await;
        assert!(without_network(&flags), "{case}: {flags:?}");
        let seen = status(dir.path(), Some(&server.source()));
        assert_eq!(seen.state, State::Unavailable, "{case}");
        assert!(seen.error.is_some(), "{case}");
    }
    // More than 64 KiB without a length said.
    let dir = profile();
    server.set(Reply {
        status: 200,
        headers: String::new(),
        body: format!("{}{}", signed(&a), " ".repeat(65 * 1024)),
        length: false,
    });
    assert!(without_network(
        &resolve(dir.path(), Some(&server.source())).await
    ));

    // The limits themselves are allowed.
    let dir = profile();
    let mut widest = preset("a".repeat(64).as_str(), 1);
    widest.name = "n".repeat(80);
    widest.bootstrap = (1..=16).map(route).collect();
    // Names are counted in characters, not bytes.
    widest.welcome = Some(Welcome {
        name: "ß".repeat(80),
        lobby_name: "界".repeat(80),
        ..welcome()
    });
    server.serve(200, signed(&widest));
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &widest);

    // A profile that has a preset keeps it.
    let dir = profile();
    server.serve(200, signed(&a));
    resolve(dir.path(), Some(&server.source())).await;
    server.serve(
        200,
        envelope(
            &serde_json::to_string(&preset("net-a", 2)).unwrap(),
            &OTHER_SEED,
        ),
    );
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &a);
    assert_eq!(
        status(dir.path(), Some(&server.source())).state,
        State::Cached
    );
}

#[test]
fn only_valid_presets_are_signed() {
    let mut plain = preset("net-a", 1);
    plain.identity_server = Some("http://id.example".into());
    assert!(sign(&serde_json::to_string(&plain).unwrap(), &SEED).is_err());
    assert!(sign("not json", &SEED).is_err());
    let mut local = preset("net-a", 1);
    local.identity_server = Some("http://127.0.0.1:8080".into());
    assert!(sign(&serde_json::to_string(&local).unwrap(), &SEED).is_ok());
    let mut lobby = preset("net-a", 1);
    lobby.welcome = Some(Welcome {
        lobby: "zz".repeat(32),
        ..welcome()
    });
    assert!(sign(&serde_json::to_string(&lobby).unwrap(), &SEED).is_err());
}

#[tokio::test]
async fn a_preset_names_the_networks_welcome_agent_and_lobby() {
    let server = Server::start();
    let dir = profile();
    // Without one, the status says so.
    server.serve(200, signed(&preset("net-a", 1)));
    resolve(dir.path(), Some(&server.source())).await;
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(seen.welcome, None);
    assert_eq!(
        serde_json::to_value(&seen).unwrap()["welcome"],
        serde_json::Value::Null
    );

    // A newer preset of the same network brings one, quietly; `kaiki
    // network` shows it as it is written.
    let mut with = preset("net-a", 2);
    with.welcome = Some(welcome());
    server.serve(200, signed(&with));
    let flags = resolve(dir.path(), Some(&server.source())).await;
    assert_flags_of(&flags, &with);
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.serial, seen.welcome.clone()),
        (State::Current, Some(2), Some(welcome()))
    );
    assert_eq!(
        serde_json::to_value(&seen).unwrap()["welcome"],
        serde_json::json!({
            "agent": format!("ain1{}", "ab".repeat(32)),
            "name": "Net welcome",
            "lobby": "cd".repeat(32),
            "lobbyName": "Net lobby",
        })
    );

    // It stays while the server is away.
    resolve(dir.path(), Some(&gone())).await;
    let seen = status(dir.path(), Some(&gone()));
    assert_eq!((seen.state, seen.welcome), (State::Cached, Some(welcome())));

    // A later preset without one takes it away.
    server.serve(200, signed(&preset("net-a", 3)));
    resolve(dir.path(), Some(&server.source())).await;
    assert_eq!(status(dir.path(), Some(&server.source())).welcome, None);

    // Another network's welcome is only offered with it: the profile's
    // status names the welcome of the network it is on.
    let mut again = preset("net-a", 4);
    again.welcome = Some(welcome());
    server.serve(200, signed(&again));
    resolve(dir.path(), Some(&server.source())).await;
    let mut other = preset("net-b", 5);
    other.welcome = Some(Welcome {
        name: "Other welcome".into(),
        ..welcome()
    });
    server.serve(200, signed(&other));
    resolve(dir.path(), Some(&server.source())).await;
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.network.as_deref(), seen.welcome),
        (State::Switch, Some("net-a"), Some(welcome()))
    );
    // Taken, the other network brings its own.
    accept_offer(dir.path(), &server.source()).unwrap();
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.network.as_deref(), seen.welcome.map(|w| w.name)),
        (Some("net-b"), Some("Other welcome".to_owned()))
    );
}

#[tokio::test]
async fn serials_only_go_forward() {
    let server = Server::start();
    let dir = profile();
    let current = preset("net-a", 5);
    server.serve(200, signed(&current));
    resolve(dir.path(), Some(&server.source())).await;

    let mut older = preset("net-a", 4);
    older.identity_server = Some("https://id.old.example".into());
    server.serve(200, signed(&older));
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &current);
    assert_eq!(
        status(dir.path(), Some(&server.source())).state,
        State::Cached
    );

    // The same serial again is current.
    server.serve(200, signed(&current));
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &current);
    assert_eq!(
        status(dir.path(), Some(&server.source())).state,
        State::Current
    );

    // Another network at an older or the same serial is not offered.
    for serial in [3, 5] {
        server.serve(200, signed(&preset("net-b", serial)));
        assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &current);
        let seen = status(dir.path(), Some(&server.source()));
        assert_eq!(
            (seen.state, seen.offered),
            (State::Cached, None),
            "serial {serial}"
        );
    }
}

#[tokio::test]
async fn the_same_network_updates_quietly() {
    let server = Server::start();
    let dir = profile();
    server.serve(200, signed(&preset("net-a", 1)));
    resolve(dir.path(), Some(&server.source())).await;
    let mut moved = preset("net-a", 2);
    moved.bootstrap = (20..=22).map(route).collect();
    moved.chain_rpc = Some("https://rpc2.net-a.example".into());
    server.serve(200, signed(&moved));
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &moved);
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!((seen.state, seen.serial), (State::Current, Some(2)));
}

#[tokio::test]
async fn another_network_is_offered_and_taken_only_when_asked() {
    let server = Server::start();
    let dir = profile();
    let a = preset("net-a", 1);
    let b = preset("net-b", 2);
    server.serve(200, signed(&a));
    resolve(dir.path(), Some(&server.source())).await;
    assert!(
        accept_offer(dir.path(), &gone()).is_err(),
        "nothing is offered yet"
    );

    server.serve(200, signed(&b));
    assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &a);
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.network.as_deref()),
        (State::Switch, Some("net-a"))
    );
    assert_eq!(
        seen.offered,
        Some(Offer {
            network: "net-b".into(),
            name: "Network net-b".into(),
            serial: 2
        })
    );
    // The offer stays while the server is away.
    assert_flags_of(&resolve(dir.path(), Some(&gone())).await, &a);
    assert_eq!(
        status(dir.path(), Some(&gone())).offered.map(|o| o.network),
        Some("net-b".into())
    );

    // What the owner saw is what is taken, whatever the server says now.
    server.serve(200, signed(&preset("net-c", 3)));
    let hits = server.hits();
    assert_eq!(accept_offer(dir.path(), &server.source()).unwrap(), b);
    assert_eq!(server.hits(), hits);
    let seen = status(dir.path(), Some(&gone()));
    assert_eq!(
        (seen.state, seen.network.as_deref(), seen.offered),
        (State::Current, Some("net-b"), None)
    );
    assert_flags_of(&resolve(dir.path(), Some(&gone())).await, &b);
    assert!(
        accept_offer(dir.path(), &gone()).is_err(),
        "the offer was taken"
    );
}

#[tokio::test]
async fn a_changed_saved_offer_is_not_taken() {
    let server = Server::start();
    let dir = profile();
    server.serve(200, signed(&preset("net-a", 1)));
    resolve(dir.path(), Some(&server.source())).await;
    server.serve(200, signed(&preset("net-b", 2)));
    resolve(dir.path(), Some(&server.source())).await;
    let kept = dir.path().join("network-preset.json");
    let mut saved: serde_json::Value = serde_json::from_slice(&fs::read(&kept).unwrap()).unwrap();
    let text = saved["offered"]["preset"]
        .as_str()
        .unwrap()
        .replace("https://id.net-b.example", "https://id.evil.example");
    saved["offered"]["preset"] = text.into();
    fs::write(&kept, saved.to_string()).unwrap();
    assert!(accept_offer(dir.path(), &gone()).is_err());
    assert_flags_of(
        &resolve(dir.path(), Some(&gone())).await,
        &preset("net-a", 1),
    );
}

#[tokio::test]
async fn a_preset_for_a_newer_app_waits_for_an_update() {
    let server = Server::start();
    let mut future = preset("net-a", 1);
    future.min_version = Some("99.0.0".into());

    let dir = profile();
    server.serve(200, signed(&future));
    assert!(without_network(
        &resolve(dir.path(), Some(&server.source())).await
    ));
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.required.as_deref()),
        (State::Update, Some("99.0.0"))
    );

    for version in ["0.0.1", env!("CARGO_PKG_VERSION")] {
        let dir = profile();
        let mut today = preset("net-a", 1);
        today.min_version = Some(version.into());
        server.serve(200, signed(&today));
        assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &today);
        assert_eq!(
            status(dir.path(), Some(&server.source())).state,
            State::Current,
            "{version}"
        );
        future.serial = 2;
        server.serve(200, signed(&future));
        assert_flags_of(&resolve(dir.path(), Some(&server.source())).await, &today);
        assert_eq!(
            status(dir.path(), Some(&server.source())).state,
            State::Update
        );
    }
}

#[tokio::test]
async fn any_saved_network_flag_makes_a_profile_manual_and_nothing_is_fetched() {
    let server = Server::start();
    server.serve(200, signed(&preset("net-a", 1)));
    let by_hand: [fn(&mut DaemonFlags); 8] = [
        |f| f.bootstrap = vec![route(1)],
        |f| f.chain_rpc = Some("https://rpc.own.example".into()),
        |f| f.chain_id = Some(1),
        |f| f.book_shop = Some(address(4)),
        |f| f.grant_issuer = Some(address(5)),
        |f| f.registry = Some(address(6)),
        |f| f.chain_confirmations = Some(1),
        |f| f.identity_server = Some("https://id.own.example".into()),
    ];
    for set in by_hand {
        let dir = profile();
        let mut manual = DaemonFlags::default();
        set(&mut manual);
        manual.save(dir.path()).unwrap();
        assert_eq!(resolve(dir.path(), Some(&server.source())).await, manual);
        let seen = status(dir.path(), Some(&server.source()));
        assert_eq!(
            (seen.source, seen.state),
            (Source::Manual, State::Manual),
            "{manual:?}"
        );
    }
    assert_eq!(server.hits(), 0);

    // Listen addresses alone do not make a profile manual.
    let dir = profile();
    let listen = DaemonFlags {
        listen: vec!["/ip4/127.0.0.1/tcp/0".into()],
        ..DaemonFlags::default()
    };
    listen.save(dir.path()).unwrap();
    let flags = resolve(dir.path(), Some(&server.source())).await;
    assert_eq!(server.hits(), 1);
    assert_eq!(flags.listen, listen.listen);
    assert_flags_of(&flags, &preset("net-a", 1));
}

#[tokio::test]
async fn without_a_source_a_profile_has_no_network() {
    let dir = profile();
    assert!(without_network(&resolve(dir.path(), None).await));
    let seen = status(dir.path(), None);
    assert_eq!((seen.source, seen.state), (Source::Off, State::Off));
}

#[tokio::test]
async fn a_damaged_saved_preset_counts_as_none() {
    let server = Server::start();
    let dir = profile();
    server.serve(200, signed(&preset("net-a", 1)));
    resolve(dir.path(), Some(&server.source())).await;
    let kept = dir.path().join("network-preset.json");
    let mut saved: serde_json::Value = serde_json::from_slice(&fs::read(&kept).unwrap()).unwrap();
    let text = saved["accepted"]["preset"]
        .as_str()
        .unwrap()
        .replace("https://id.net-a.example", "https://id.evil.example");
    saved["accepted"]["preset"] = text.into();
    fs::write(&kept, saved.to_string()).unwrap();
    assert!(without_network(&resolve(dir.path(), Some(&gone())).await));
    assert_eq!(status(dir.path(), Some(&gone())).state, State::Unavailable);

    fs::write(&kept, "garbage").unwrap();
    assert!(without_network(&resolve(dir.path(), Some(&gone())).await));
}

#[test]
fn the_source_is_https_or_loopback_and_can_be_turned_off() {
    let key = public_key(&SEED);
    assert!(PresetSource::new("https://kaikichat.com/network.json", &key).is_ok());
    assert!(PresetSource::new("http://127.0.0.1:8080/network.json", &key).is_ok());
    assert!(PresetSource::new("http://localhost:8080/network.json", &key).is_ok());
    assert!(PresetSource::new("http://[::1]:8080/network.json", &key).is_ok());
    assert!(PresetSource::new("http://kaikichat.com/network.json", &key).is_err());
    assert!(PresetSource::new("http://127.0.0.1.evil.example/n.json", &key).is_err());
    assert!(PresetSource::new("https://kaikichat.com/network.json", "abcd").is_err());

    assert_eq!(PRESET_URL, "https://kaikichat.com/network.json");
    assert_eq!(PresetSource::public().unwrap().url(), PRESET_URL);
    let chosen = |preset: Option<&str>, key: Option<&str>| {
        PresetSource::from_values(preset, key).map(|source| source.map(|s| s.url().to_owned()))
    };
    assert_eq!(chosen(None, None).unwrap().as_deref(), Some(PRESET_URL));
    assert_eq!(chosen(Some("off"), None).unwrap(), None);
    assert_eq!(chosen(Some("off"), Some("")).unwrap(), None);
    assert_eq!(
        chosen(Some("http://127.0.0.1:9/p.json"), Some(&key))
            .unwrap()
            .as_deref(),
        Some("http://127.0.0.1:9/p.json")
    );
    assert!(chosen(Some("http://example.com/p.json"), None).is_err());
    assert!(chosen(Some("https://example.com/p.json"), Some("zz")).is_err());
}

#[test]
fn routes_are_chosen_at_random_and_a_short_list_is_taken_whole() {
    let six = preset("net-a", 1);
    let mut seen = BTreeSet::new();
    for _ in 0..20 {
        let flags = six.flags(DaemonFlags::default());
        assert_flags_of(&flags, &six);
        seen.extend(flags.bootstrap);
    }
    assert_eq!(seen.len(), 6, "every route is chosen now and then");

    let mut two = preset("net-a", 1);
    two.bootstrap = (1..=2).map(route).collect();
    let flags = two.flags(DaemonFlags::default());
    assert_flags_of(&flags, &two);
    assert_eq!(flags.bootstrap.len(), 2);
}

#[tokio::test]
async fn an_offer_is_replaced_only_by_a_newer_one() {
    let server = Server::start();
    let dir = profile();
    server.serve(200, signed(&preset("net-a", 1)));
    resolve(dir.path(), Some(&server.source())).await;
    server.serve(200, signed(&preset("net-b", 5)));
    resolve(dir.path(), Some(&server.source())).await;
    // The offer served again is still the offer.
    resolve(dir.path(), Some(&server.source())).await;
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.offered.map(|o| o.serial)),
        (State::Switch, Some(5))
    );
    server.serve(200, signed(&preset("net-c", 3)));
    resolve(dir.path(), Some(&server.source())).await;
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!(
        (seen.state, seen.offered.map(|o| o.network)),
        (State::Switch, Some("net-b".into()))
    );
    server.serve(200, signed(&preset("net-c", 7)));
    resolve(dir.path(), Some(&server.source())).await;
    assert_eq!(
        status(dir.path(), Some(&server.source()))
            .offered
            .map(|o| o.network),
        Some("net-c".into())
    );
    // Seeing the network kept again, at a newer serial, withdraws the offer.
    server.serve(200, signed(&preset("net-a", 8)));
    assert_flags_of(
        &resolve(dir.path(), Some(&server.source())).await,
        &preset("net-a", 8),
    );
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!((seen.state, seen.offered), (State::Current, None));
    // Nor is an older offer made after that.
    server.serve(200, signed(&preset("net-c", 7)));
    resolve(dir.path(), Some(&server.source())).await;
    assert_eq!(status(dir.path(), Some(&server.source())).offered, None);
}

#[tokio::test]
async fn an_old_preset_asking_for_a_newer_app_is_just_old() {
    let server = Server::start();
    let dir = profile();
    server.serve(200, signed(&preset("net-a", 5)));
    resolve(dir.path(), Some(&server.source())).await;
    let mut old = preset("net-a", 4);
    old.min_version = Some("99.0.0".into());
    server.serve(200, signed(&old));
    assert_flags_of(
        &resolve(dir.path(), Some(&server.source())).await,
        &preset("net-a", 5),
    );
    let seen = status(dir.path(), Some(&server.source()));
    assert_eq!((seen.state, seen.required), (State::Cached, None));
}

#[tokio::test]
async fn a_saved_state_of_a_newer_app_is_still_read() {
    let server = Server::start();
    let dir = profile();
    server.serve(200, signed(&preset("net-a", 1)));
    resolve(dir.path(), Some(&server.source())).await;
    let kept = dir.path().join("network-preset.json");
    let mut saved: serde_json::Value = serde_json::from_slice(&fs::read(&kept).unwrap()).unwrap();
    saved["state"] = "someday".into();
    saved["futureField"] = serde_json::json!([1]);
    fs::write(&kept, saved.to_string()).unwrap();
    assert_flags_of(
        &resolve(dir.path(), Some(&gone())).await,
        &preset("net-a", 1),
    );
    // No temporary file of a check remains.
    let left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(left.contains(&"network-preset.json".to_owned()), "{left:?}");
    assert!(!left.iter().any(|name| name.ends_with(".new")), "{left:?}");
}

#[test]
fn the_directory_comes_from_the_preset_unless_set_by_hand() {
    let mut with = preset("net-a", 1);
    with.directory = Some("https://directory.net-a.example".into());
    with.directory_key = Some("AB".repeat(32));
    let flags = with.flags(DaemonFlags::default());
    assert_eq!(
        (flags.directory.as_deref(), flags.directory_key),
        (
            Some("https://directory.net-a.example"),
            Some("ab".repeat(32))
        )
    );
    // A directory set by hand wins, with its own key (or none).
    let own = DaemonFlags {
        directory: Some("https://own.example".into()),
        ..DaemonFlags::default()
    };
    let flags = with.flags(own);
    assert_eq!(
        (flags.directory.as_deref(), flags.directory_key),
        (Some("https://own.example"), None)
    );
    // Nor does a directory set by hand make the profile manual.
    let own = DaemonFlags {
        directory: Some("https://own.example".into()),
        directory_key: Some("cd".repeat(32)),
        ..DaemonFlags::default()
    };
    assert!(!own.has_network());
    assert!(sign(&serde_json::to_string(&with).unwrap(), &SEED).is_ok());
}

/// A release as a preset announces it: its version and the build of the
/// command line for macOS.
fn release(version: &str) -> Release {
    Release {
        version: version.into(),
        builds: [(
            "cli-macos-arm64".to_owned(),
            Build {
                url: "https://kaikichat.com/downloads/kaiki-macos-arm64.tar.gz".into(),
                sha256: "ab".repeat(32),
            },
        )]
        .into(),
    }
}

fn with_release(network: &str, serial: u64, version: &str) -> Preset {
    let mut preset = preset(network, serial);
    preset.release = Some(release(version));
    preset
}

/// Moves the last release check `seconds` into the past.
fn age_release_check(dir: &Path, seconds: u64) {
    let file = dir.join("release.json");
    let mut kept: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    let at = kept["checkedAt"].as_u64().expect("a check time");
    kept["checkedAt"] = (at - seconds).into();
    fs::write(&file, kept.to_string()).unwrap();
}

#[test]
fn only_valid_releases_are_signed() {
    let signs = |preset: &Preset| sign(&serde_json::to_string(preset).unwrap(), &SEED).is_ok();
    assert!(signs(&with_release("net-a", 1, "1.2.3")));
    // A release may name its version alone: nothing to install by itself.
    let mut bare = preset("net-a", 1);
    bare.release = Some(Release {
        version: "1.2.3".into(),
        builds: Default::default(),
    });
    assert!(signs(&bare));
    let broken: [fn(&mut Release); 5] = [
        |release| release.version = "1.2".into(),
        |release| release.version = "v1.2.3".into(),
        |release| {
            release
                .builds
                .values_mut()
                .for_each(|b| b.sha256 = "ab".repeat(31))
        },
        |release| {
            release
                .builds
                .values_mut()
                .for_each(|b| b.sha256 = "zz".repeat(32))
        },
        |release| {
            release
                .builds
                .values_mut()
                .for_each(|b| b.url = "http://kaikichat.com/kaiki.tar.gz".into())
        },
    ];
    for (n, change) in broken.into_iter().enumerate() {
        let mut preset = with_release("net-a", 1, "1.2.3");
        change(preset.release.as_mut().unwrap());
        assert!(!signs(&preset), "broken release {n} was signed");
    }
}

#[tokio::test]
async fn a_starting_daemon_learns_the_newest_release() {
    let server = Server::start();
    let source = server.source();
    let dir = profile();
    let seen = release_status(dir.path(), Some(&source));
    assert_eq!(
        (
            seen.current.as_str(),
            seen.latest.as_deref(),
            seen.available,
            seen.skipped,
            seen.checked_at
        ),
        (env!("CARGO_PKG_VERSION"), None, false, false, None)
    );
    server.serve(200, signed(&with_release("net-a", 1, "99.0.0")));
    resolve(dir.path(), Some(&source)).await;
    let seen = release_status(dir.path(), Some(&source));
    assert_eq!(
        (seen.latest.as_deref(), seen.available, seen.skipped),
        (Some("99.0.0"), true, false)
    );
    assert!(seen.checked_at.is_some());
    assert_eq!(seen.error, None);
    let json = serde_json::to_value(&seen).unwrap();
    for key in [
        "current",
        "latest",
        "available",
        "skipped",
        "checkedAt",
        "error",
    ] {
        assert!(json.get(key).is_some(), "{key} in {json}");
    }
    // What an update installs, as signed.
    assert_eq!(latest_release(dir.path(), &source), Some(release("99.0.0")));
    assert_eq!(
        fs::metadata(dir.path().join("release.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    // This version, or an older one, is no news.
    for version in [env!("CARGO_PKG_VERSION"), "0.0.1"] {
        let dir = profile();
        server.serve(200, signed(&with_release("net-a", 1, version)));
        resolve(dir.path(), Some(&source)).await;
        let seen = release_status(dir.path(), Some(&source));
        assert_eq!(
            (seen.latest.as_deref(), seen.available),
            (Some(version), false)
        );
    }
}

#[tokio::test]
async fn the_release_is_the_newest_presets_and_older_ones_do_not_bring_theirs_back() {
    let server = Server::start();
    let source = server.source();
    let dir = profile();
    server.serve(200, signed(&with_release("net-a", 3, "99.1.0")));
    resolve(dir.path(), Some(&source)).await;
    // An older preset of a higher version: someone replaying it.
    server.serve(200, signed(&with_release("net-a", 2, "99.2.0")));
    resolve(dir.path(), Some(&source)).await;
    assert_eq!(
        release_status(dir.path(), Some(&source)).latest.as_deref(),
        Some("99.1.0")
    );
    // A newer preset without a release withdraws it.
    server.serve(200, signed(&preset("net-a", 4)));
    resolve(dir.path(), Some(&source)).await;
    let seen = release_status(dir.path(), Some(&source));
    assert_eq!((seen.latest, seen.available), (None, false));
    assert_eq!(latest_release(dir.path(), &source), None);
    // A release is the app's, whichever network's preset brings it: here
    // one only offered.
    server.serve(200, signed(&with_release("net-b", 5, "99.3.0")));
    resolve(dir.path(), Some(&source)).await;
    let network = status(dir.path(), Some(&source));
    assert_eq!(
        (network.state, network.network.as_deref()),
        (State::Switch, Some("net-a"))
    );
    assert_eq!(
        release_status(dir.path(), Some(&source)).latest.as_deref(),
        Some("99.3.0")
    );
}

#[tokio::test]
async fn a_kept_release_is_believed_only_while_its_signature_holds() {
    let server = Server::start();
    let source = server.source();
    let dir = profile();
    server.serve(200, signed(&with_release("net-a", 1, "99.0.0")));
    resolve(dir.path(), Some(&source)).await;
    let file = dir.path().join("release.json");
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("99.0.0"), "{text}");
    fs::write(&file, text.replace("99.0.0", "99.9.9")).unwrap();
    assert_eq!(release_status(dir.path(), Some(&source)).latest, None);
    assert_eq!(latest_release(dir.path(), &source), None);

    // Nor under another key.
    let dir = profile();
    resolve(dir.path(), Some(&source)).await;
    assert_eq!(
        release_status(dir.path(), Some(&source)).latest.as_deref(),
        Some("99.0.0")
    );
    let other = PresetSource::new(&server.url, &public_key(&OTHER_SEED)).unwrap();
    assert_eq!(release_status(dir.path(), Some(&other)).latest, None);
    assert_eq!(release_status(dir.path(), None).latest, None);
}

#[tokio::test]
async fn checks_between_daemon_starts_ask_the_server_at_most_every_twelve_hours() {
    assert_eq!(RELEASE_CHECK_EVERY, Duration::from_secs(12 * 3600));
    let server = Server::start();
    let source = server.source();
    let dir = profile();
    server.serve(200, signed(&with_release("net-a", 1, "99.0.0")));
    resolve(dir.path(), Some(&source)).await;
    assert_eq!(server.hits(), 1);
    server.serve(200, signed(&with_release("net-a", 2, "99.1.0")));
    let seen = check_release(dir.path(), &source, Some(RELEASE_CHECK_EVERY)).await;
    assert_eq!((server.hits(), seen.latest.as_deref()), (1, Some("99.0.0")));

    age_release_check(dir.path(), 12 * 3600 + 1);
    let seen = check_release(dir.path(), &source, Some(RELEASE_CHECK_EVERY)).await;
    assert_eq!((server.hits(), seen.latest.as_deref()), (2, Some("99.1.0")));
    // A check learns the release only: the network is taken when the
    // daemon starts.
    assert_eq!(status(dir.path(), Some(&source)).serial, Some(1));

    // Asked for now, it asks now.
    check_release(dir.path(), &source, None).await;
    assert_eq!(server.hits(), 3);

    // A failed check counts as one: the next is twelve hours away, and the
    // release known stays.
    age_release_check(dir.path(), 12 * 3600 + 1);
    server.serve(503, signed(&with_release("net-a", 3, "99.2.0")));
    let seen = check_release(dir.path(), &source, Some(RELEASE_CHECK_EVERY)).await;
    assert_eq!((server.hits(), seen.latest.as_deref()), (4, Some("99.1.0")));
    assert!(seen.error.is_some());
    check_release(dir.path(), &source, Some(RELEASE_CHECK_EVERY)).await;
    assert_eq!(server.hits(), 4);
    // The next good check clears the error.
    server.serve(200, signed(&with_release("net-a", 3, "99.2.0")));
    let seen = check_release(dir.path(), &source, None).await;
    assert_eq!((seen.latest.as_deref(), seen.error), (Some("99.2.0"), None));
}

#[tokio::test]
async fn a_network_that_needs_a_newer_app_names_the_release_to_update_to() {
    let server = Server::start();
    let source = server.source();
    let dir = profile();
    let mut future = with_release("net-a", 1, "99.0.0");
    future.min_version = Some("99.0.0".into());
    server.serve(200, signed(&future));
    resolve(dir.path(), Some(&source)).await;
    assert_eq!(status(dir.path(), Some(&source)).state, State::Update);
    let seen = release_status(dir.path(), Some(&source));
    assert_eq!(
        (seen.latest.as_deref(), seen.available),
        (Some("99.0.0"), true)
    );
    assert_eq!(latest_release(dir.path(), &source), future.release);
}

#[tokio::test]
async fn profiles_set_by_hand_ask_for_releases_only_when_the_owner_does() {
    let server = Server::start();
    let source = server.source();
    let dir = profile();
    DaemonFlags {
        bootstrap: vec![route(1)],
        ..DaemonFlags::default()
    }
    .save(dir.path())
    .unwrap();
    server.serve(200, signed(&with_release("net-a", 1, "99.0.0")));
    let seen = check_release(dir.path(), &source, Some(RELEASE_CHECK_EVERY)).await;
    assert_eq!((server.hits(), seen.latest), (0, None));
    let seen = check_release(dir.path(), &source, None).await;
    assert_eq!((server.hits(), seen.latest.as_deref()), (1, Some("99.0.0")));
    // Still set by hand: the check took no network.
    assert_eq!(status(dir.path(), Some(&source)).source, Source::Manual);
}

#[tokio::test]
async fn a_skipped_version_is_quiet_until_a_newer_one() {
    let server = Server::start();
    let source = server.source();
    let dir = profile();
    server.serve(200, signed(&with_release("net-a", 1, "99.0.0")));
    resolve(dir.path(), Some(&source)).await;
    let seen = skip_release(dir.path(), &source, "99.0.0").unwrap();
    assert_eq!(
        (seen.latest.as_deref(), seen.available, seen.skipped),
        (Some("99.0.0"), true, true)
    );
    assert!(release_status(dir.path(), Some(&source)).skipped);
    // Checking again does not bring it back.
    check_release(dir.path(), &source, None).await;
    resolve(dir.path(), Some(&source)).await;
    assert!(release_status(dir.path(), Some(&source)).skipped);
    // A newer version is news again.
    server.serve(200, signed(&with_release("net-a", 2, "99.0.1")));
    let seen = check_release(dir.path(), &source, None).await;
    assert_eq!(
        (seen.latest.as_deref(), seen.available, seen.skipped),
        (Some("99.0.1"), true, false)
    );
    assert!(skip_release(dir.path(), &source, "not a version").is_err());
    assert!(!release_status(dir.path(), Some(&source)).skipped);
}
