//! The owner CLI `kaiki` (spec/owner-cli-v1.md) as real processes: it
//! starts, stops and restarts the daemon from its own secrets file, creates
//! the identity, and two profiles talk through it. Secrets live in the
//! password-sealed file, never the system keychain.
use super::bootstrap::RecordPeer;
use super::*;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

const PASSWORD: &str = "correct horse battery staple";

/// One profile driven through the CLI.
struct Owner {
    dir: TempDir,
    /// The network preset's URL and key; none turns the preset off.
    preset: Option<(String, String)>,
    /// The `kaiki` it runs: this build's, or an install of it.
    kaiki: std::path::PathBuf,
}

impl Owner {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("ain-cli-")
            .tempdir_in("/tmp")
            .unwrap();
        // A profile directory must be private, as the desktop's is.
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            dir,
            preset: None,
            kaiki: env!("CARGO_BIN_EXE_kaiki").into(),
        }
    }

    /// A profile that follows the network preset at `url`, signed by `key`.
    fn with_preset(url: &str, key: &str) -> Self {
        let mut owner = Self::new();
        owner.preset = Some((url.to_owned(), key.to_owned()));
        owner
    }

    /// `command` with this profile's network preset, or with it off.
    fn preset_env<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        match &self.preset {
            Some((url, key)) => command
                .env("AGENTIC_NETWORK_PRESET", url)
                .env("AGENTIC_NETWORK_PRESET_KEY", key),
            None => command
                .env("AGENTIC_NETWORK_PRESET", "off")
                .env_remove("AGENTIC_NETWORK_PRESET_KEY"),
        }
    }

    /// Run `kaiki` with `args` and optional stdin: its exit code and JSON.
    fn run_with(&self, password: &str, args: &[&str], input: Option<&str>) -> (i32, Value) {
        let mut command = Command::new(&self.kaiki);
        let mut child = self
            .preset_env(&mut command)
            .args(args)
            .env("HOME", self.home())
            .env("AGENTIC_DATA_DIR", self.dir.path())
            .env("AGENTIC_SECRETS", "file")
            .env("AGENTIC_PASSWORD", password)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        }
        drop(child.stdin.take());
        let output = child.wait_with_output().unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let value = serde_json::from_str(text.trim())
            .unwrap_or_else(|_| panic!("not one JSON envelope: {text:?}"));
        (output.status.code().unwrap_or(-1), value)
    }

    /// A home of its own, so nothing reaches the developer's real one.
    fn home(&self) -> std::path::PathBuf {
        let home = self.dir.path().join("home");
        fs::create_dir_all(&home).unwrap();
        home
    }

    fn run(&self, args: &[&str], input: Option<&str>) -> (i32, Value) {
        self.run_with(PASSWORD, args, input)
    }

    /// A command that must succeed: its result.
    fn ok(&self, args: &[&str], input: Option<&str>) -> Value {
        let (code, value) = self.run(args, input);
        assert_eq!(code, 0, "{args:?}: {value}");
        value["result"].clone()
    }

    /// A command that must fail with `exit` and `error`.
    fn fails(&self, args: &[&str], input: Option<&str>, exit: i32, error: &str) {
        let (code, value) = self.run(args, input);
        assert_eq!(
            (code, value["error"]["code"].as_str()),
            (exit, Some(error)),
            "{args:?}: {value}"
        );
    }

    fn start(&self) -> Value {
        self.ok(
            &["daemon", "start", "--listen", "/ip4/127.0.0.1/tcp/0"],
            None,
        )
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.run(&["daemon", "stop"], None);
    }
}

fn wait(what: &str, check: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !check() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(200));
    }
}

/// Alice and Bob, started, named and connected: the conversation id and the
/// name Bob sees Alice under.
fn connected() -> (Owner, Owner, String, String) {
    let (alice, bob) = (Owner::new(), Owner::new());
    alice.start();
    bob.start();
    alice.ok(&["init", "--name", "Alice"], None);
    bob.ok(&["init", "--name", "Bob"], None);
    let invitation = bob.ok(&["contacts", "invite"], None)["invitation"]
        .as_str()
        .unwrap()
        .to_owned();
    let added = alice.ok(
        &["contacts", "add", "--name", "Bob", "--invitation-stdin"],
        Some(&invitation),
    );
    let conversation = added["conversationId"].as_str().unwrap().to_owned();
    wait("Bob sees Alice", || {
        bob.ok(&["contacts", "list"], None)
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["conversationId"] == conversation)
    });
    let name = bob.ok(&["contacts", "list"], None)[0]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    (alice, bob, conversation, name)
}

fn send(owner: &Owner, to: &str, operation: &str, text: &str) -> Value {
    owner.ok(
        &[
            "send",
            "--to",
            to,
            "--operation-id",
            operation,
            "--text-stdin",
        ],
        Some(text),
    )
}

fn texts(page: &Value) -> Vec<&str> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["text"].as_str().unwrap())
        .collect()
}

#[test]
fn the_cli_starts_stops_and_restarts_a_profile_from_its_own_secrets() {
    let alice = Owner::new();
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
    let started = alice.start();
    assert_eq!(started["running"], true);
    let peer = started["peerId"].as_str().unwrap().to_owned();
    let identity = alice.ok(&["init", "--name", "Alice"], None);
    assert_eq!(identity["name"], "Alice");
    let network = identity["networkId"].as_str().unwrap().to_owned();
    assert!(!network.is_empty());
    // The secrets file is private and in the spec's sealed format.
    let secrets = alice.dir.path().join("secrets.json");
    assert_eq!(
        fs::metadata(&secrets).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let sealed: Value = serde_json::from_str(&fs::read_to_string(&secrets).unwrap()).unwrap();
    assert_eq!(
        (sealed["version"].as_u64(), sealed["kdf"].as_str()),
        (Some(1), Some("argon2id"))
    );
    for field in ["salt", "nonce", "ciphertext"] {
        assert!(
            sealed[field].as_str().is_some_and(|hex| !hex.is_empty()),
            "{field}"
        );
    }
    assert_eq!(alice.ok(&["daemon", "stop"], None)["running"], false);
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
    // Another command starts it again with the saved listeners: the same
    // transport identity and profile.
    assert_eq!(alice.ok(&["contacts", "list"], None), json!([]));
    let status = alice.ok(&["daemon", "status"], None);
    assert_eq!(
        (status["running"].as_bool(), status["peerId"].as_str()),
        (Some(true), Some(peer.as_str()))
    );
    assert_eq!(status["networkId"], network);
    // `init` is safe to retry; another name is refused.
    assert_eq!(
        alice.ok(&["init", "--name", "Alice"], None)["networkId"],
        network
    );
    alice.fails(&["init", "--name", "Mallory"], None, 3, "profile_exists");
    // A wrong password opens nothing and starts nothing.
    alice.ok(&["daemon", "stop"], None);
    let (code, value) = alice.run_with("wrong", &["daemon", "start"], None);
    assert_eq!(
        (code, value["error"]["code"].as_str()),
        (2, Some("secrets_locked")),
        "{value}"
    );
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
    // A changed file opens nothing either: the secret is sealed, not hashed.
    let mut changed = sealed.clone();
    let ciphertext = changed["ciphertext"].as_str().unwrap().to_owned();
    let flipped = if ciphertext.starts_with('0') {
        "1"
    } else {
        "0"
    };
    changed["ciphertext"] = json!(format!("{flipped}{}", &ciphertext[1..]));
    fs::write(&secrets, changed.to_string()).unwrap();
    alice.fails(&["daemon", "start"], None, 2, "secrets_locked");
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
}

/// Flags given to `daemon start` stay with the profile: a daemon a later
/// command starts again has them (here the identity server, so a claim is
/// pending and retryable instead of refused as not configured).
#[test]
fn a_restarted_daemon_keeps_the_flags_it_was_first_started_with() {
    let alice = Owner::new();
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let server = format!("http://{closed}");
    alice.ok(
        &[
            "daemon",
            "start",
            "--listen",
            "/ip4/127.0.0.1/tcp/0",
            "--identity-server",
            &server,
        ],
        None,
    );
    alice.ok(&["init", "--name", "Alice"], None);
    alice.ok(&["daemon", "stop"], None);
    let (code, value) = alice.run(&["coins", "claim"], None);
    assert_eq!(
        (
            code,
            value["error"]["code"].as_str(),
            value["error"]["retryable"].as_bool()
        ),
        (4, Some("claim_pending"), Some(true)),
        "{value}"
    );
}

/// The profile's one daemon, once its command line has `flag`: its pid and
/// command line. A stopped daemon may linger a moment after its socket.
fn daemon_with(owner: &Owner, flag: &str) -> (u32, String) {
    let path = fs::canonicalize(owner.dir.path()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let listing = Command::new("ps")
            .args(["-wwAo", "pid=,command="])
            .output()
            .unwrap()
            .stdout;
        let daemons: Vec<(u32, String)> = String::from_utf8(listing)
            .unwrap()
            .lines()
            .filter(|line| {
                line.contains("kaiki-agentic-node serve") && line.contains(path.to_str().unwrap())
            })
            .filter_map(|line| {
                let (pid, command) = line.trim().split_once(' ')?;
                Some((pid.parse().ok()?, command.to_owned()))
            })
            .collect();
        if let [(pid, command)] = daemons.as_slice()
            && command.contains(flag)
        {
            return (*pid, command.clone());
        }
        assert!(
            Instant::now() < deadline,
            "one daemon with {flag}: {daemons:?}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

/// A local server of one network preset at a time.
struct PresetServer {
    url: String,
    body: std::sync::Arc<std::sync::Mutex<String>>,
    hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl PresetServer {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/network.json", listener.local_addr().unwrap());
        let body = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (served, count) = (body.clone(), hits.clone());
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                    request.push(byte[0]);
                }
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let body = served.lock().unwrap().clone();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Self { url, body, hits }
    }
    fn hits(&self) -> usize {
        self.hits.load(std::sync::atomic::Ordering::SeqCst)
    }
    /// Serves the network `network` at `serial`: one route to a peer that
    /// is not there, and the identity server `identity`.
    fn serve(&self, network: &str, serial: u64, identity: &str) {
        self.serve_routes(network, serial, identity, &[&nobody()]);
    }
    /// Serves the network `network` at `serial` with the bootstrap `routes`.
    fn serve_routes(&self, network: &str, serial: u64, identity: &str, routes: &[&str]) {
        let preset = json!({
            "network": network,
            "name": format!("Network {network}"),
            "serial": serial,
            "bootstrap": routes,
            "identityServer": identity,
        });
        *self.body.lock().unwrap() =
            agentic_node::network_preset::sign(&preset.to_string(), &PRESET_SEED).unwrap();
    }
    fn owner(&self) -> Owner {
        Owner::with_preset(
            &self.url,
            &agentic_node::network_preset::public_key(&PRESET_SEED),
        )
    }
}

const PRESET_SEED: [u8; 32] = [5; 32];

/// A new profile starts its daemon with the network of the signed preset;
/// commands on a running daemon never fetch it; another network is only
/// offered, and taken when the owner asks.
#[test]
fn a_new_profile_follows_the_signed_preset_and_changes_network_only_when_asked() {
    let server = PresetServer::start();
    let (first, second) = (closed_identity_server(), closed_identity_server());
    server.serve("net-a", 1, &first);
    let alice = server.owner();
    alice.ok(&["init", "--name", "Alice"], None);
    let (started, command) = daemon_with(&alice, &format!("--identity-server {first}"));
    assert_eq!(command.matches("--bootstrap").count(), 1, "{command}");
    let network = alice.ok(&["network"], None);
    assert_eq!(
        (
            &network["source"],
            &network["state"],
            &network["network"],
            &network["serial"]
        ),
        (
            &json!("preset"),
            &json!("current"),
            &json!("net-a"),
            &json!(1)
        ),
        "{network}"
    );
    for key in ["name", "checkedAt", "offered", "required", "error"] {
        assert!(network.get(key).is_some(), "{key} in {network}");
    }
    alice.fails(&["network", "switch"], None, 3, "no_network_offer");

    // Commands on the running daemon keep its network and ask nobody.
    server.serve("net-b", 2, &second);
    let hits = server.hits();
    alice.ok(&["daemon", "status"], None);
    alice.ok(&["contacts", "list"], None);
    assert_eq!(alice.ok(&["network"], None)["state"], "current");
    assert_eq!(server.hits(), hits);

    let network = alice.ok(&["network", "refresh"], None);
    assert_eq!(
        (
            &network["state"],
            &network["network"],
            &network["offered"]["network"]
        ),
        (&json!("switch"), &json!("net-a"), &json!("net-b")),
        "{network}"
    );
    assert_eq!(server.hits(), hits + 1);
    let (refreshed, _) = daemon_with(&alice, &format!("--identity-server {first}"));
    assert_ne!(refreshed, started, "refresh starts the daemon again");

    let network = alice.ok(&["network", "switch"], None);
    assert_eq!(
        (&network["state"], &network["network"], &network["offered"]),
        (&json!("current"), &json!("net-b"), &Value::Null),
        "{network}"
    );
    daemon_with(&alice, &format!("--identity-server {second}"));
    // The profile and its identity stay.
    assert_eq!(alice.ok(&["daemon", "status"], None)["name"], "Alice");
}

/// A profile started with network flags is set by hand: the preset is not
/// asked for and changes nothing.
#[test]
fn a_profile_with_network_flags_is_manual() {
    let server = PresetServer::start();
    server.serve("net-a", 1, &closed_identity_server());
    let alice = server.owner();
    let own = closed_identity_server();
    alice.ok(
        &[
            "daemon",
            "start",
            "--listen",
            "/ip4/127.0.0.1/tcp/0",
            "--identity-server",
            &own,
        ],
        None,
    );
    let (_, command) = daemon_with(&alice, &format!("--identity-server {own}"));
    assert!(!command.contains("--bootstrap"), "{command}");
    let network = alice.ok(&["network"], None);
    assert_eq!(
        (&network["source"], &network["state"]),
        (&json!("manual"), &json!("manual"))
    );
    assert_eq!(server.hits(), 0);
    let fresh = Owner::new();
    let network = fresh.ok(&["network"], None);
    assert_eq!(
        (&network["source"], &network["state"]),
        (&json!("off"), &json!("off"))
    );
}

/// The profile's running daemon over the owner IPC, as the window's
/// settings panel calls it: the node's result for `method`.
fn window_call(owner: &Owner, method: &str, request: Value) -> Value {
    let vault = agentic_node::secrets_file::PasswordFileStore::new(
        owner.dir.path(),
        zeroize::Zeroizing::new(PASSWORD.to_owned()),
    );
    let answer = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let host = agentic_node::host::DesktopHost::attach(owner.dir.path(), &vault)
                .await
                .unwrap()
                .expect("the profile's daemon runs");
            host.call(method, request).await.unwrap()
        });
    assert!(answer.get("error").is_none(), "{method}: {answer}");
    answer["result"].clone()
}

/// A route to a peer nobody runs: a valid network preference that reaches
/// nothing.
fn nobody() -> String {
    let peer = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    format!("/ip4/127.0.0.1/tcp/9/p2p/{peer}")
}

/// `network lan` (spec/owner-cli-v1.md): local discovery is off by default;
/// `on` and `off` change only it on the running daemon and keep every other
/// network preference the window saved, and the choice, kept in the
/// profile, holds when a later command starts the daemon again.
#[test]
fn an_owner_turns_local_discovery_on_and_off_and_keeps_the_other_network_settings() {
    let alice = Owner::new();
    alice.start();
    let (daemon, _) = daemon_with(&alice, "--profile");
    // Showing it, or turning off what is off, saves nothing: a profile that
    // never changed its network keeps following its flags.
    let off = json!({"enabled": false, "active": false, "blockedByPolicy": false, "peers": []});
    assert_eq!(alice.ok(&["network", "lan"], None), off);
    assert_eq!(alice.ok(&["network", "lan", "off"], None), off);
    assert_eq!(
        window_call(&alice, "network_settings", json!({}))["revision"],
        0
    );

    // The window saved relays, a verifier, a bootstrap route and the DHT role.
    let saved = json!({
        "relays": [nobody()],
        "relayOnly": false,
        "autoNatPeers": [nobody()],
        "bootstrapPeers": [nobody()],
        "lanDiscovery": false,
        "dhtServer": true,
    });
    let revision = window_call(
        &alice,
        "configure_network",
        json!({"expectedRevision": 0, "preferences": saved}),
    )["revision"]
        .as_u64()
        .unwrap();
    let on = alice.ok(&["network", "lan", "on"], None);
    assert_eq!(
        (&on["enabled"], &on["active"], &on["blockedByPolicy"]),
        (&json!(true), &json!(true), &json!(false)),
        "{on}"
    );
    assert!(on["peers"].is_array(), "{on}");
    let mut with_lan = saved.clone();
    with_lan["lanDiscovery"] = json!(true);
    let settings = window_call(&alice, "network_settings", json!({}));
    assert_eq!(settings["preferences"], with_lan, "{settings}");
    assert_eq!(settings["revision"], revision + 1);
    // The running daemon took it: nothing was started again.
    assert_eq!(daemon_with(&alice, "--profile").0, daemon);

    // The daemon a later command starts, from the same saved flags, has it.
    alice.ok(&["daemon", "stop"], None);
    let restarted = alice.ok(&["network", "lan"], None);
    assert_eq!(
        (&restarted["enabled"], &restarted["active"]),
        (&json!(true), &json!(true)),
        "{restarted}"
    );

    let off_again = alice.ok(&["network", "lan", "off"], None);
    assert_eq!(
        (&off_again["enabled"], &off_again["active"]),
        (&json!(false), &json!(false)),
        "{off_again}"
    );
    let settings = window_call(&alice, "network_settings", json!({}));
    assert_eq!(settings["preferences"], saved, "{settings}");
    assert_eq!(settings["revision"], revision + 2);

    // An agent learns the command from the skill.
    let skill = alice.ok(&["skill", "show"], None);
    assert!(
        skill["text"]
            .as_str()
            .is_some_and(|text| text.contains("kaiki network lan on")),
        "the skill lacks `kaiki network lan on`"
    );
}

/// Whether the daemon asked `peer` for its node record: it dialed that
/// bootstrap route.
fn dialed(peer: &RecordPeer) -> bool {
    !peer.requests.lock().unwrap().is_empty()
}

/// The profile's network preferences with the bootstrap `routes` the owner
/// names (none: the key left out, as the window saves an empty field),
/// saved over the owner IPC as the window's settings panel saves them.
fn window_routes(owner: &Owner, routes: Option<Value>) -> Value {
    let settings = window_call(owner, "network_settings", json!({}));
    let mut preferences = settings["preferences"].clone();
    match routes {
        Some(routes) => preferences["bootstrapPeers"] = routes,
        None => {
            preferences
                .as_object_mut()
                .unwrap()
                .remove("bootstrapPeers");
        }
    }
    window_call(
        owner,
        "configure_network",
        json!({"expectedRevision": settings["revision"], "preferences": preferences}),
    )
}

/// Saving a network setting that is not about routes (here local
/// discovery, spec/owner-cli-v1.md) keeps a profile on its network's
/// bootstrap routes: the daemon dials the route a newer preset gives.
/// Routes the owner names in the window replace the network's until the
/// owner clears them.
#[test]
fn saved_network_settings_keep_the_presets_routes_until_the_owner_names_others() {
    let server = PresetServer::start();
    let identity = closed_identity_server();
    let first = RecordPeer::start(6);
    server.serve_routes("net-a", 1, &identity, &[&first.address]);
    let alice = server.owner();
    // Loopback only: local discovery announces nothing on the real network.
    alice.start();
    wait("the preset's route is dialed", || dialed(&first));

    assert_eq!(alice.ok(&["network", "lan", "on"], None)["enabled"], true);
    let settings = window_call(&alice, "network_settings", json!({}));
    assert_eq!(settings["revision"], 1, "{settings}");
    assert!(
        settings["preferences"].get("bootstrapPeers").is_none(),
        "the network's routes are not saved as the owner's: {settings}"
    );
    assert_eq!(
        settings["status"]["bootstrap"]["routes"],
        json!([first.address]),
        "{settings}"
    );

    // The network moves its route: a fresh look at the preset reaches the new one.
    let second = RecordPeer::start(6);
    server.serve_routes("net-a", 2, &identity, &[&second.address]);
    alice.ok(&["network", "refresh"], None);
    wait("the newer preset's route is dialed", || dialed(&second));
    let lan = alice.ok(&["network", "lan"], None);
    assert_eq!(
        (&lan["enabled"], &lan["active"]),
        (&json!(true), &json!(true)),
        "{lan}"
    );

    // A route the owner names replaces the network's, also for a daemon
    // started again with a newer preset's route.
    let own = RecordPeer::start(6);
    let named = window_routes(&alice, Some(json!([own.address])));
    assert_eq!(named["preferences"]["bootstrapPeers"], json!([own.address]));
    wait("the owner's route is dialed", || dialed(&own));
    let third = RecordPeer::start(6);
    server.serve_routes("net-a", 3, &identity, &[&third.address]);
    alice.ok(&["network", "refresh"], None);
    daemon_with(&alice, &format!("--bootstrap {}", third.address));
    let settings = window_call(&alice, "network_settings", json!({}));
    assert_eq!(
        settings["status"]["bootstrap"]["routes"],
        json!([own.address]),
        "{settings}"
    );

    // Cleared, the running daemon takes the network's route again.
    let cleared = window_routes(&alice, None);
    assert!(
        cleared["preferences"].get("bootstrapPeers").is_none(),
        "{cleared}"
    );
    assert_eq!(
        cleared["status"]["bootstrap"]["routes"],
        json!([third.address])
    );
    wait("the network's route is dialed again", || dialed(&third));
}

/// A closed port: a node told to use it as its identity server keeps asking.
fn closed_identity_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}", listener.local_addr().unwrap())
}

/// A stand-in for a discovery service that answers its policy: this
/// network, and `key` as the key it signs bindings with.
fn policy_server(key: [u8; 32]) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body = json!({
        "domain": hex::encode(agentic_node::NETWORK_DOMAIN),
        "key": hex::encode(key),
        "lookupPrice": 1,
        "cardPrice": 10,
        "cardDays": 30,
    })
    .to_string();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut request);
            let _ = std::io::Write::write_all(
                &mut stream,
                format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            );
        }
    });
    url
}

/// A profile whose daemon names its discovery service's key, as the
/// network's preset does, trusts no other key: the service's key is checked
/// before the node is asked for stamps, so a service answering with another
/// key gets nothing; with the named key the lookup goes on (here to want a
/// book to pay with).
#[test]
fn a_discovery_service_with_another_key_than_the_named_one_is_refused() {
    let alice = Owner::new();
    let service = policy_server([0x33; 32]);
    let start = |key: [u8; 32]| {
        alice.ok(
            &[
                "daemon",
                "start",
                "--listen",
                "/ip4/127.0.0.1/tcp/0",
                "--directory",
                &service,
                "--directory-key",
                &hex::encode(key),
            ],
            None,
        )
    };
    start([0x44; 32]);
    alice.ok(&["init", "--name", "Alice"], None);
    alice.fails(
        &["discover", "lookup", "--email", "ann@example.org"],
        None,
        3,
        "directory_key_mismatch",
    );
    alice.ok(&["daemon", "stop"], None);
    start([0x33; 32]);
    alice.fails(
        &["discover", "lookup", "--email", "ann@example.org"],
        None,
        3,
        "book_required",
    );
}

/// Agents run commands at once: on a stopped profile they start one daemon
/// and all use it.
#[test]
fn concurrent_commands_on_a_stopped_profile_start_one_daemon() {
    let alice = Owner::new();
    let peer = alice.start()["peerId"].as_str().unwrap().to_owned();
    alice.ok(&["init", "--name", "Alice"], None);
    alice.ok(&["daemon", "stop"], None);
    let results: Vec<(i32, Value)> = thread::scope(|scope| {
        let runs: Vec<_> = (0..3)
            .map(|_| scope.spawn(|| alice.run(&["contacts", "list"], None)))
            .collect();
        runs.into_iter().map(|run| run.join().unwrap()).collect()
    });
    for (code, value) in &results {
        assert_eq!(*code, 0, "{value}");
    }
    let status = alice.ok(&["daemon", "status"], None);
    assert_eq!(status["peerId"], peer);
    let path = alice.dir.path().to_str().unwrap().to_owned();
    let daemons = String::from_utf8(
        Command::new("ps")
            .args(["-wwAo", "command"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .lines()
    .filter(|line| line.contains("kaiki-agentic-node") && line.contains(&path))
    .count();
    assert_eq!(daemons, 1);
}

#[test]
fn two_profiles_talk_through_the_cli() {
    let (alice, bob) = (Owner::new(), Owner::new());
    alice.start();
    bob.start();
    alice.ok(&["init", "--name", "Alice"], None);
    bob.ok(&["init", "--name", "Bob"], None);
    let invitation = bob.ok(&["contacts", "invite"], None)["invitation"]
        .as_str()
        .unwrap()
        .to_owned();
    let added = alice.ok(
        &["contacts", "add", "--name", "Bob", "--invitation-stdin"],
        Some(&invitation),
    );
    assert_eq!(added["name"], "Bob");
    let conversation = added["conversationId"].as_str().unwrap().to_owned();
    wait("Bob sees Alice", || {
        bob.ok(&["contacts", "list"], None)
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["conversationId"] == conversation)
    });
    let sent = alice.ok(
        &[
            "send",
            "--to",
            "Bob",
            "--operation-id",
            "hello-1",
            "--text-stdin",
        ],
        // A here-document's final line break is not part of the message.
        Some("hi from the CLI\n"),
    );
    // The same operation id is the same message.
    let again = alice.ok(
        &[
            "send",
            "--to",
            &conversation,
            "--operation-id",
            "hello-1",
            "--text-stdin",
        ],
        Some("hi from the CLI"),
    );
    assert_eq!(again["messageId"], sent["messageId"]);
    let alice_name = bob.ok(&["contacts", "list"], None)[0]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    wait("Bob reads it", || {
        bob.ok(&["messages", "--with", &alice_name], None)
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["text"] == "hi from the CLI" && m["own"] == false)
    });
    wait("Alice sees it delivered", || {
        alice
            .ok(&["messages", "--with", "Bob"], None)
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == sent["messageId"] && m["delivery"] == "delivered")
    });
}

#[test]
fn cli_refusals_are_json_with_stable_exit_codes() {
    let alice = Owner::new();
    alice.start();
    alice.ok(&["init", "--name", "Alice"], None);
    alice.fails(
        &[
            "send",
            "--to",
            "Nobody",
            "--operation-id",
            "x",
            "--text-stdin",
        ],
        Some("text"),
        3,
        "unknown_contact",
    );
    alice.fails(
        &[
            "send",
            "--to",
            "Nobody",
            "--operation-id",
            "x",
            "--text-stdin",
        ],
        Some("  "),
        2,
        "invalid_input",
    );
    // Opening a group publishes it for good: only with --confirm.
    alice.fails(
        &[
            "groups",
            "access",
            "--group",
            "Team",
            "--to",
            "public",
            "--operation-id",
            "o-1",
        ],
        None,
        2,
        "confirmation_required",
    );
    // Discovery needs the service named at `daemon start --directory`.
    alice.fails(
        &["discover", "search", "rust"],
        None,
        3,
        "directory_not_configured",
    );
    alice.fails(&["coins", "buy"], None, 3, "chain_not_configured");
    alice.fails(&["coins", "claim"], None, 3, "identity_not_configured");
    // An operator's prizes need the pool named at `daemon start --operator-pool`.
    alice.fails(&["earnings"], None, 3, "pool_not_configured");
    alice.fails(&["earnings", "withdraw"], None, 3, "pool_not_configured");
    // Without a password the file backend opens nothing.
    let (code, value) = {
        let child = Command::new(env!("CARGO_BIN_EXE_kaiki"))
            .args(["contacts", "list"])
            .env("AGENTIC_DATA_DIR", alice.dir.path())
            .env("AGENTIC_NETWORK_PRESET", "off")
            .env("AGENTIC_SECRETS", "file")
            .env_remove("AGENTIC_PASSWORD")
            .env_remove("AGENTIC_PASSWORD_FILE")
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        (
            child.status.code().unwrap(),
            serde_json::from_slice::<Value>(&child.stdout).unwrap(),
        )
    };
    assert_eq!(
        (code, value["error"]["code"].as_str()),
        (2, Some("password_required")),
        "{value}"
    );
}

/// An owner (or the agent running the node) works through its inbox: the
/// cursor moves only on ack, a lease holds its page, `watch` wakes on a new
/// message, and the cursor survives a restart.
#[test]
fn an_owner_works_through_its_inbox_with_poll_and_ack() {
    let (alice, bob, conversation, alice_at_bob) = connected();
    for (n, text) in ["one", "two", "three"].iter().enumerate() {
        send(&alice, "Bob", &format!("in-{n}"), text);
    }
    wait("three are waiting", || {
        bob.ok(&["inbox", "watch", "--timeout-seconds", "0"], None)["conversations"]
            == json!([{"conversationId": conversation, "name": alice_at_bob, "unread": 3}])
    });
    let with = ["--with", alice_at_bob.as_str()];
    let poll = |extra: &[&str]| {
        let mut args = vec!["inbox", "poll"];
        args.extend(with);
        args.extend(extra);
        bob.ok(&args, None)
    };
    let ack = |lease: &str| {
        let mut args = vec!["inbox", "ack"];
        args.extend(with);
        args.extend(["--lease-id", lease]);
        bob.run(&args, None)
    };
    let page = poll(&["--limit", "2"]);
    assert_eq!(texts(&page), ["one", "two"]);
    assert_eq!(page["hasMore"], true);
    assert_eq!(page["conversationId"], conversation);
    // The default lease is 60 seconds.
    let expires = page["expiresAt"].as_u64().unwrap();
    assert!(
        (unix_now() + 50..=unix_now() + 61).contains(&expires),
        "{page}"
    );
    let lease = page["leaseId"].as_str().unwrap().to_owned();
    assert_eq!(poll(&[])["leaseId"], lease.as_str());
    // A leased conversation is not waiting.
    assert_eq!(
        bob.ok(&["inbox", "watch", "--timeout-seconds", "0"], None)["conversations"],
        json!([])
    );
    assert_eq!(ack(&lease).0, 0);
    assert_eq!(ack(&lease).0, 0);
    let last = poll(&[]);
    assert_eq!(texts(&last), ["three"]);
    assert_eq!(last["hasMore"], false);
    assert_eq!(ack(last["leaseId"].as_str().unwrap()).0, 0);
    let empty = poll(&[]);
    assert!(texts(&empty).is_empty());
    assert_eq!(empty["leaseId"], Value::Null);
    let (code, refused) = ack(&"ab".repeat(32));
    assert_eq!(
        (code, refused["error"]["code"].as_str()),
        (3, Some("inbox_lease_expired")),
        "{refused}"
    );
    // `watch` returns when a message arrives, not at its timeout.
    let started = Instant::now();
    let woke = thread::scope(|scope| {
        let watcher = scope.spawn(|| bob.ok(&["inbox", "watch", "--timeout-seconds", "60"], None));
        thread::sleep(Duration::from_millis(500));
        send(&alice, "Bob", "in-4", "four");
        watcher.join().unwrap()
    });
    assert_eq!(
        woke["conversations"],
        json!([{"conversationId": conversation, "name": alice_at_bob, "unread": 1}])
    );
    assert!(started.elapsed() < Duration::from_secs(30));
    // The cursor survives a restart.
    bob.ok(&["daemon", "stop"], None);
    assert_eq!(texts(&poll(&[])), ["four"]);
    // Bounds are checked before anything starts.
    bob.ok(&["daemon", "stop"], None);
    for args in [
        vec!["inbox", "poll", "--with", "x", "--limit", "0"],
        vec!["inbox", "poll", "--with", "x", "--limit", "101"],
        vec!["inbox", "poll", "--with", "x", "--lease-seconds", "601"],
        vec!["inbox", "watch", "--timeout-seconds", "3601"],
        vec!["inbox", "ack", "--with", "x", "--lease-id", "not-hex"],
    ] {
        bob.fails(&args, None, 2, "invalid_input");
    }
    assert_eq!(bob.ok(&["daemon", "status"], None)["running"], false);
    bob.fails(
        &["inbox", "poll", "--with", "Nobody"],
        None,
        3,
        "unknown_contact",
    );
}

/// The agent's credentials run `agentic-cli`: its exit code and JSON.
fn agent(credentials: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_agentic-cli"))
        .arg("--credentials")
        .arg(credentials)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

/// The owner gives another agent one contact through `grants` and takes it
/// back: the agent works with the credentials until the revoke.
#[test]
fn an_owner_grants_an_agent_one_contact_and_revokes_it() {
    let (alice, _bob, conversation, _) = connected();
    let create = [
        "grants",
        "create",
        "--name",
        "helper",
        "--contact",
        "Bob",
        "--read",
        "--send",
        "--operation-id",
        "g-1",
    ];
    let grant = alice.ok(&create, None);
    let id = grant["grantId"].as_str().unwrap().to_owned();
    assert!(id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()));
    let contacts = json!([{"conversationId": conversation, "name": "Bob"}]);
    assert_eq!(grant["contacts"], contacts);
    assert_eq!(grant["actions"], json!(["read_inbox", "send_message"]));
    assert_eq!(grant["maxDataBytes"], 4096);
    // Thirty days by default.
    let expires = grant["expiresAt"].as_u64().unwrap();
    assert!(expires.abs_diff(unix_now() + 30 * 86_400) < 120, "{grant}");
    let credentials = PathBuf::from(grant["credentialsPath"].as_str().unwrap());
    assert_eq!(
        fs::metadata(&credentials).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(
        grant["cliConfig"]["command"]
            .as_str()
            .unwrap()
            .ends_with("/agentic-cli")
    );
    assert_eq!(
        grant["mcpConfig"]["mcpServers"].as_object().unwrap().len(),
        1
    );
    // A retry in a later second is the same grant; another intent under the
    // same operation id is refused.
    thread::sleep(Duration::from_millis(1100));
    assert_eq!(alice.ok(&create, None)["grantId"], id.as_str());
    alice.fails(
        &[
            "grants",
            "create",
            "--name",
            "other",
            "--contact",
            "Bob",
            "--read",
            "--operation-id",
            "g-1",
        ],
        None,
        3,
        "operation_conflict",
    );
    let listed = alice.ok(&["grants", "list"], None);
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(
        (
            listed[0]["grantId"].as_str(),
            listed[0]["name"].as_str(),
            listed[0]["status"].as_str()
        ),
        (Some(id.as_str()), Some("helper"), Some("active"))
    );
    assert_eq!(listed[0]["contacts"], contacts);
    assert_eq!(listed[0]["expiresAt"], grant["expiresAt"]);
    assert_eq!(agent(&credentials, &["context"]).0, 0);
    alice.ok(&["grants", "revoke", "--grant-id", &id], None);
    alice.ok(&["grants", "revoke", "--grant-id", &id], None);
    let (code, denied) = agent(&credentials, &["context"]);
    assert_eq!(
        (code, denied["error"]["code"].as_str()),
        (3, Some("unauthorized")),
        "{denied}"
    );
    assert_eq!(alice.ok(&["grants", "list"], None)[0]["status"], "revoked");
    alice.fails(
        &["grants", "revoke", "--grant-id", &"cd".repeat(32)],
        None,
        3,
        "unknown_grant",
    );
    alice.fails(
        &["grants", "revoke", "--grant-id", "xyz"],
        None,
        2,
        "invalid_input",
    );
    let refused: [(&[&str], i32, &str); 4] = [
        (&["--contact", "Bob"], 2, "invalid_input"),
        (
            &["--contact", "Bob", "--read", "--days", "0"],
            2,
            "invalid_input",
        ),
        (
            &["--contact", "Bob", "--send", "--max-bytes", "48001"],
            2,
            "invalid_input",
        ),
        (&["--contact", "Nobody", "--read"], 3, "unknown_contact"),
    ];
    for (n, (extra, exit, error)) in refused.into_iter().enumerate() {
        let operation = format!("bad-{n}");
        let mut args = vec![
            "grants",
            "create",
            "--name",
            "n",
            "--operation-id",
            &operation,
        ];
        args.extend(extra);
        alice.fails(&args, None, exit, error);
    }
    assert_eq!(
        alice
            .ok(&["grants", "list"], None)
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // A grant lives at most thirty days. The refusal binds nothing (not even
    // the kept expiry): the same operation id then gives the longest grant.
    let longest = |days| {
        [
            "grants",
            "create",
            "--name",
            "n",
            "--contact",
            "Bob",
            "--read",
            "--operation-id",
            "longest",
            "--days",
            days,
        ]
    };
    alice.fails(&longest("31"), None, 2, "invalid_input");
    let grant = alice.ok(&longest("30"), None);
    let expires = grant["expiresAt"].as_u64().unwrap();
    assert!(expires.abs_diff(unix_now() + 30 * 86_400) < 120, "{grant}");
}

/// On Linux without a Secret Service the default is the password-sealed
/// file: without a password nothing opens, with one the profile runs.
#[cfg(target_os = "linux")]
#[test]
fn without_a_secret_service_linux_defaults_to_the_password_file() {
    let alice = Owner::new();
    let run = |password: Option<&str>| {
        let mut command = Command::new(&alice.kaiki);
        command
            .args(["contacts", "list"])
            .env("AGENTIC_DATA_DIR", alice.dir.path())
            .env("AGENTIC_NETWORK_PRESET", "off")
            .env_remove("AGENTIC_SECRETS")
            .env_remove("AGENTIC_PASSWORD_FILE")
            // No session bus answers here.
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={}/no-bus", alice.dir.path().display()),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        match password {
            Some(password) => command.env("AGENTIC_PASSWORD", password),
            None => command.env_remove("AGENTIC_PASSWORD"),
        };
        let output = command.output().unwrap();
        (
            output.status.code().unwrap_or(-1),
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        )
    };
    let (code, value) = run(None);
    assert_eq!(
        (code, value["error"]["code"].as_str()),
        (2, Some("password_required")),
        "{value}"
    );
    let (code, value) = run(Some(PASSWORD));
    assert_eq!(code, 0, "{value}");
    assert!(alice.dir.path().join("secrets.json").is_file());
}

/// Contact by ID from the CLI (spec/contact-by-id-v1.md): the recipient's
/// policy, waiting requests and the request itself. The swarm flow runs in
/// the managed-time rig; here, without a swarm, the CLI's own contract.
#[test]
fn an_owner_sets_who_may_ask_by_id_and_decides_on_requests() {
    let alice = Owner::new();
    alice.start();
    let id = alice.ok(&["init", "--name", "Alice"], None)["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        alice.ok(&["contacts", "policy"], None),
        json!({"mode": "all", "dailyLimit": 100, "allowed": []})
    );
    let someone = format!("ain1{}", "ab".repeat(32));
    let other = format!("ain1{}", "cd".repeat(32));
    assert_eq!(
        alice.ok(
            &[
                "contacts", "policy", "--mode", "manual", "--allow", &someone, "--allow", &other,
            ],
            None
        ),
        json!({"mode": "manual", "dailyLimit": 100, "allowed": [someone, other]})
    );
    // Fields not given stay; the list is replaced only when given.
    assert_eq!(
        alice.ok(&["contacts", "policy", "--daily-limit", "5"], None),
        json!({"mode": "manual", "dailyLimit": 5, "allowed": [someone, other]})
    );
    // `--allow` replaces the list.
    let third = format!("ain1{}", "ef".repeat(32));
    assert_eq!(
        alice.ok(&["contacts", "policy", "--allow", &third], None)["allowed"],
        json!([third])
    );
    assert_eq!(
        alice.ok(&["contacts", "policy", "--clear-allowed"], None)["allowed"],
        json!([])
    );
    assert_eq!(
        alice.ok(&["contacts", "policy", "--daily-limit", "0"], None)["dailyLimit"],
        0
    );
    // It survives a restart.
    alice.ok(&["daemon", "stop"], None);
    assert_eq!(alice.ok(&["contacts", "policy"], None)["mode"], "manual");
    assert_eq!(alice.ok(&["contacts", "requests"], None), json!([]));
    let unknown = "ef".repeat(32);
    alice.fails(
        &["contacts", "accept", "--request", &unknown],
        None,
        3,
        "unknown_request",
    );
    alice.fails(
        &["contacts", "reject", "--request", &unknown],
        None,
        3,
        "unknown_request",
    );
    // Without a swarm directory a request cannot look the card up yet.
    alice.fails(
        &[
            "contacts",
            "request",
            "--id",
            &someone,
            "--name",
            "Someone",
            "--operation-id",
            "ask-1",
        ],
        None,
        4,
        "network_unavailable",
    );
    // One's own id is refused by the node.
    alice.fails(
        &[
            "contacts",
            "request",
            "--id",
            &id,
            "--name",
            "Me",
            "--operation-id",
            "ask-2",
        ],
        None,
        3,
        "invalid_request",
    );
    // Malformed input is refused before anything starts.
    alice.ok(&["daemon", "stop"], None);
    let bad: [&[&str]; 6] = [
        &[
            "contacts",
            "request",
            "--id",
            "ain1xyz",
            "--name",
            "X",
            "--operation-id",
            "a",
        ],
        &["contacts", "policy", "--mode", "sometimes"],
        &["contacts", "policy", "--daily-limit", "1001"],
        &["contacts", "policy", "--allow", "not-an-id"],
        &["contacts", "policy", "--allow", &someone, "--clear-allowed"],
        &["contacts", "accept", "--request", "zz"],
    ];
    for args in bad {
        alice.fails(args, None, 2, "invalid_input");
    }
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
}

/// Groups from the CLI (spec/groups-v1.md). Without a swarm a group has only
/// its owner; making members of ids needs the swarm, which the rig covers.
#[test]
fn an_owner_makes_and_uses_groups_from_the_cli() {
    let alice = Owner::new();
    alice.start();
    alice.ok(&["init", "--name", "Alice"], None);
    assert_eq!(alice.ok(&["groups", "list"], None), json!([]));
    let made = alice.ok(
        &[
            "groups",
            "create",
            "--name",
            "Solo",
            "--operation-id",
            "g-1",
        ],
        None,
    );
    assert_eq!(
        (made["name"].as_str(), made["role"].as_str()),
        (Some("Solo"), Some("owner"))
    );
    let id = made["id"].as_str().unwrap().to_owned();
    // Safe to retry.
    assert_eq!(
        alice.ok(
            &[
                "groups",
                "create",
                "--name",
                "Solo",
                "--operation-id",
                "g-1"
            ],
            None
        )["id"],
        id.as_str()
    );
    let listed = alice.ok(&["groups", "list"], None);
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["id"], id.as_str());
    // A group by its name or id, as a member sees it.
    let own = alice.ok(&["daemon", "status"], None)["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    for group in ["Solo", id.as_str()] {
        let shown = alice.ok(&["groups", "show", "--group", group], None);
        assert_eq!(shown["id"], id.as_str());
        assert_eq!(shown["owner"], own.as_str());
        assert_eq!(shown["members"], json!([own]));
        assert_eq!(shown["admins"], json!([]));
        assert!(shown["epoch"].is_u64(), "{shown}");
    }
    // Two groups of one name: only their ids tell them apart.
    let twins: Vec<String> = ["g-3", "g-4"]
        .iter()
        .map(|op| {
            alice.ok(
                &["groups", "create", "--name", "Twin", "--operation-id", op],
                None,
            )["id"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_ne!(twins[0], twins[1]);
    alice.fails(
        &["groups", "show", "--group", "Twin"],
        None,
        3,
        "ambiguous_group",
    );
    alice.fails(
        &[
            "groups",
            "send",
            "--group",
            "Twin",
            "--operation-id",
            "m-9",
            "--text-stdin",
        ],
        Some("to whom?"),
        3,
        "ambiguous_group",
    );
    assert_eq!(
        alice.ok(&["groups", "show", "--group", &twins[1]], None)["id"],
        twins[1].as_str()
    );
    let sent = alice.ok(
        &[
            "groups",
            "send",
            "--group",
            "Solo",
            "--operation-id",
            "m-1",
            "--text-stdin",
        ],
        Some("only me"),
    );
    assert!(sent["messageId"].is_string(), "{sent}");
    assert_eq!(
        alice.ok(&["messages", "--with", "Solo"], None)[0]["text"],
        "only me"
    );
    let someone = format!("ain1{}", "ab".repeat(32));
    // Finding a member's card needs the swarm directory.
    alice.fails(
        &[
            "groups",
            "add",
            "--group",
            "Solo",
            "--member",
            &someone,
            "--operation-id",
            "c-1",
        ],
        None,
        4,
        "network_unavailable",
    );
    // Not a member: nothing to remove.
    alice.fails(
        &[
            "groups",
            "remove",
            "--group",
            "Solo",
            "--member",
            &someone,
            "--operation-id",
            "c-2",
        ],
        None,
        3,
        "invalid_request",
    );
    // A ban needs no card: its commit is made and waits for the notary,
    // and the next change waits its turn.
    let banned = alice.ok(
        &[
            "groups",
            "ban",
            "--group",
            "Solo",
            "--member",
            &someone,
            "--operation-id",
            "c-5",
        ],
        None,
    );
    assert!(banned["commit"].is_string(), "{banned}");
    alice.fails(
        &[
            "groups",
            "unban",
            "--group",
            "Solo",
            "--member",
            &someone,
            "--operation-id",
            "c-6",
        ],
        None,
        4,
        "group_busy",
    );
    alice.fails(
        &["groups", "show", "--group", "Nope"],
        None,
        3,
        "unknown_group",
    );
    // Malformed input is refused before anything starts.
    alice.ok(&["daemon", "stop"], None);
    let bad: [&[&str]; 4] = [
        &[
            "groups",
            "create",
            "--name",
            "X",
            "--member",
            "ain1xyz",
            "--operation-id",
            "g-2",
        ],
        &[
            "groups",
            "add",
            "--group",
            "Solo",
            "--member",
            "nope",
            "--operation-id",
            "c-3",
        ],
        &[
            "groups",
            "admins",
            "--group",
            "Solo",
            "--admin",
            "nope",
            "--operation-id",
            "c-4",
        ],
        &[
            "groups",
            "ban",
            "--group",
            "Solo",
            "--member",
            "nope",
            "--operation-id",
            "c-7",
        ],
    ];
    for args in bad {
        alice.fails(args, None, 2, "invalid_input");
    }
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
}

/// Channels and doors from the command line
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md): a public channel only
/// with --confirm, its history kept longer and what that costs; a closed
/// one whose keys go by card; a group by request whose applications wait
/// for a decision.
#[test]
fn an_owner_makes_channels_and_decides_at_doors_from_the_cli() {
    let alice = Owner::new();
    alice.start();
    alice.ok(&["init", "--name", "Alice"], None);
    let public = |confirm: bool| {
        let mut args = vec![
            "channels",
            "create",
            "--name",
            "News",
            "--access",
            "public",
            "--operation-id",
            "c-1",
        ];
        if confirm {
            args.push("--confirm");
        }
        args
    };
    // What is written in a public channel is public for good.
    alice.fails(&public(false), None, 2, "confirmation_required");
    let news = alice.ok(&public(true), None);
    assert_eq!(
        (
            &news["kind"],
            &news["access"],
            &news["role"],
            &news["retention"]
        ),
        (
            &json!("channel"),
            &json!("public"),
            &json!("owner"),
            &json!(30)
        ),
        "{news}"
    );
    // Its history kept longer: a commit for the notaries.
    let kept = alice.ok(
        &[
            "channels",
            "retention",
            "--channel",
            "News",
            "--days",
            "90",
            "--operation-id",
            "r-1",
        ],
        None,
    );
    assert!(kept["commit"].is_string(), "{kept}");
    // Until the notaries order that commit the channel keeps 30 days.
    let storage = alice.ok(&["channels", "storage", "--channel", "News"], None);
    assert_eq!(
        storage,
        json!({"retention": 30, "parts": 0, "bytes": 0, "stampsPerMonth": 0, "addedLastMonth": 0})
    );
    // A closed channel: keys go by card, found through the swarm; it keeps
    // no history.
    let club = alice.ok(
        &[
            "channels",
            "create",
            "--name",
            "Club",
            "--access",
            "private",
            "--operation-id",
            "c-2",
        ],
        None,
    );
    assert_eq!(
        (&club["kind"], &club["access"]),
        (&json!("channel"), &json!("private"))
    );
    let someone = format!("ain1{}", "ab".repeat(32));
    alice.fails(
        &[
            "channels",
            "subscribe",
            "--channel",
            "Club",
            "--member",
            &someone,
            "--operation-id",
            "s-1",
        ],
        None,
        4,
        "network_unavailable",
    );
    alice.fails(
        &[
            "channels",
            "unsubscribe",
            "--channel",
            "Club",
            "--member",
            &someone,
            "--operation-id",
            "u-1",
        ],
        None,
        3,
        "invalid_request",
    );
    alice.fails(
        &[
            "channels",
            "retention",
            "--channel",
            "Club",
            "--days",
            "90",
            "--operation-id",
            "r-2",
        ],
        None,
        3,
        "invalid_request",
    );
    // It stays closed: by request or private, never public.
    alice.fails(
        &[
            "groups",
            "access",
            "--group",
            "Club",
            "--to",
            "public",
            "--confirm",
            "--operation-id",
            "a-0",
        ],
        None,
        3,
        "invalid_request",
    );
    let reseeded = alice.ok(
        &[
            "channels",
            "reseed",
            "--channel",
            "Club",
            "--operation-id",
            "x-1",
        ],
        None,
    );
    assert!(reseeded["commit"].is_string(), "{reseeded}");
    // A group by request: applications wait for a decision.
    alice.ok(
        &[
            "groups",
            "create",
            "--name",
            "Solo",
            "--operation-id",
            "g-1",
        ],
        None,
    );
    let door = alice.ok(
        &[
            "groups",
            "access",
            "--group",
            "Solo",
            "--to",
            "request",
            "--operation-id",
            "a-1",
        ],
        None,
    );
    assert!(door["commit"].is_string(), "{door}");
    assert_eq!(
        alice.ok(&["groups", "requests", "--group", "Solo"], None),
        json!([])
    );
    let nobody = "ab".repeat(32);
    // An application another admin already answered: nothing to decide.
    alice.fails(
        &[
            "groups",
            "decide",
            "--group",
            "Solo",
            "--request",
            &nobody,
            "--accept",
        ],
        None,
        3,
        "unknown_request",
    );
    // Asking at a door means finding its card through the swarm.
    alice.fails(
        &[
            "groups",
            "join",
            "--group-ref",
            &nobody,
            "--note",
            &"ä".repeat(280),
            "--operation-id",
            "j-0",
        ],
        None,
        4,
        "network_unavailable",
    );
    alice.fails(
        &["discover", "search", "rust", "--kind", "channel"],
        None,
        3,
        "directory_not_configured",
    );
    // Malformed input is refused before anything starts.
    alice.ok(&["daemon", "stop"], None);
    let long = "ä".repeat(281);
    let bad: [&[&str]; 6] = [
        &[
            "channels",
            "retention",
            "--channel",
            "News",
            "--days",
            "45",
            "--operation-id",
            "r-3",
        ],
        &[
            "channels",
            "create",
            "--name",
            "X",
            "--access",
            "open",
            "--operation-id",
            "c-3",
        ],
        &["groups", "decide", "--group", "Solo", "--request", &nobody],
        &[
            "groups",
            "join",
            "--group-ref",
            "nope",
            "--operation-id",
            "j-1",
        ],
        &[
            "groups",
            "decide",
            "--group",
            "Solo",
            "--request",
            &nobody,
            "--accept",
            "--reject",
        ],
        &[
            "groups",
            "join",
            "--group-ref",
            &nobody,
            "--note",
            &long,
            "--operation-id",
            "j-2",
        ],
    ];
    for args in bad {
        alice.fails(args, None, 2, "invalid_input");
    }
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
}

/// The skill (spec/owner-cli-v1.md): it names every command of the spec,
/// states the rules an agent needs, tells where this CLI is, and installs
/// into a host's skills without the daemon.
#[test]
fn the_skill_describes_every_command_and_installs_into_a_hosts_skills() {
    let alice = Owner::new();
    let shown = alice.ok(&["skill", "show"], None);
    assert_eq!(shown["name"], "kaiki");
    let text = shown["text"].as_str().unwrap().to_owned();
    // The app's bundle is not on PATH: before any command the skill names
    // this very CLI, quoted for a shell, with this profile (not the default
    // one), so a later session still finds both.
    let cli = Path::new(env!("CARGO_BIN_EXE_kaiki"));
    let named = [cli.to_path_buf(), fs::canonicalize(cli).unwrap()]
        .iter()
        .filter_map(|path| {
            text.find(&format!(
                "`'{}' --data-dir '{}'`",
                path.display(),
                alice.dir.path().display()
            ))
        })
        .min()
        .unwrap_or_else(|| panic!("the skill does not name {}:\n{text}", cli.display()));
    let first_command = text.find("```").unwrap();
    assert!(
        named < first_command,
        "the CLI's path comes after a command"
    );
    // The front matter hosts load a skill by.
    let front = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split("\n---\n").next())
        .expect("front matter");
    assert!(front.lines().any(|l| l == "name: kaiki"), "{front}");
    assert!(
        front.lines().any(|l| l
            .strip_prefix("description: ")
            .is_some_and(|d| !d.trim().is_empty())),
        "{front}"
    );
    let spec = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spec/owner-cli-v1.md"
    ))
    .unwrap();
    let block = spec
        .split("```\nkaiki [")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    let mut commands = 0;
    for line in block.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let words: Vec<&str> = line
            .split_whitespace()
            .take_while(|w| !w.starts_with('-') && !w.starts_with('['))
            .take(2)
            .collect();
        let command = format!("kaiki {}", words.join(" "));
        let named = [" ", "\n", "`"]
            .iter()
            .any(|end| text.contains(&format!("{command}{end}")));
        assert!(named, "the skill lacks `{command}`");
        commands += 1;
    }
    assert!(commands >= 30, "{commands} commands in the spec block");
    let lower = text.to_lowercase();
    for rule in ["untrusted", "operation id", "exit code", "retryable"] {
        assert!(lower.contains(rule), "the skill lacks the rule on {rule}");
    }
    let dir = alice.dir.path().join("skills");
    let target = dir.join("kaiki/SKILL.md");
    for _ in 0..2 {
        let installed = alice.ok(&["skill", "install", "--dir", dir.to_str().unwrap()], None);
        assert_eq!(installed["path"], target.to_str().unwrap());
        assert_eq!(fs::read_to_string(&target).unwrap(), text);
    }
    // By default into the host's skills in the home directory.
    let default = alice.home().join(".claude/skills/kaiki/SKILL.md");
    let installed = alice.ok(&["skill", "install"], None);
    assert_eq!(installed["path"], default.to_str().unwrap());
    assert_eq!(fs::read_to_string(&default).unwrap(), text);
    assert_eq!(alice.ok(&["daemon", "status"], None)["running"], false);
}

/// A line-based MCP client over `kaiki mcp`'s stdio.
struct McpHost {
    child: std::process::Child,
    input: Option<std::process::ChildStdin>,
    output: std::io::BufReader<std::process::ChildStdout>,
    next: u64,
}

impl McpHost {
    fn start(owner: &Owner) -> Self {
        let mut command = Command::new(&owner.kaiki);
        let mut child = owner
            .preset_env(&mut command)
            .arg("mcp")
            .env("HOME", owner.home())
            .env("AGENTIC_DATA_DIR", owner.dir.path())
            .env("AGENTIC_SECRETS", "file")
            .env("AGENTIC_PASSWORD", PASSWORD)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut host = Self {
            child,
            input,
            output,
            next: 1,
        };
        let init = host.request(
            "initialize",
            json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "owner-cli-test", "version": "1"}}),
        );
        assert_eq!(init["result"]["protocolVersion"], "2025-11-25", "{init}");
        assert!(init["result"]["serverInfo"]["name"].is_string(), "{init}");
        host.write(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        host
    }
    fn write(&mut self, value: Value) {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{value}").unwrap();
        input.flush().unwrap();
    }
    fn request(&mut self, method: &str, params: Value) -> Value {
        use std::io::BufRead;
        let id = self.next;
        self.next += 1;
        self.write(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let mut line = String::new();
            assert!(self.output.read_line(&mut line).unwrap() > 0, "MCP closed");
            let value: Value = serde_json::from_str(line.trim()).unwrap();
            if value["id"] == id {
                return value;
            }
        }
    }
    fn tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }
    fn ok(&mut self, name: &str, arguments: Value) -> Value {
        let answer = self.tool(name, arguments);
        assert_eq!(answer["result"]["isError"], false, "{answer}");
        let structured = answer["result"]["structuredContent"].clone();
        // An object, and the same JSON as the text the model reads.
        assert!(structured.is_object(), "{answer}");
        let text: Value =
            serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, structured);
        structured
    }
    fn refused(&mut self, name: &str, arguments: Value) -> Value {
        let answer = self.tool(name, arguments);
        assert_eq!(answer["result"]["isError"], true, "{answer}");
        answer["result"]["structuredContent"]["error"].clone()
    }
    fn close(mut self) {
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(Instant::now() < deadline, "MCP did not exit after EOF");
            thread::sleep(Duration::from_millis(50));
        }
    }
}

/// An agent host drives the profile over `kaiki mcp` with the tools the
/// spec lists, and gets the CLI's results and refusals.
#[test]
fn an_agent_host_drives_the_profile_over_mcp() {
    let alice = Owner::new();
    alice.start();
    alice.ok(&["init", "--name", "Alice"], None);
    // The MCP server starts the daemon like any command.
    alice.ok(&["daemon", "stop"], None);
    let mut host = McpHost::start(&alice);
    let listed = host.request("tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().unwrap().clone();
    let mut names: Vec<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect();
    names.sort();
    let mut expected = vec![
        "coins_balance",
        "contacts_accept",
        "contacts_list",
        "contacts_reject",
        "contacts_request",
        "contacts_requests",
        "discover_link",
        "discover_lookup",
        "discover_publish",
        "discover_search",
        "discover_status",
        "discover_unlink",
        "discover_withdraw",
        "channels_create",
        "channels_reseed",
        "channels_retention",
        "channels_storage",
        "channels_subscribe",
        "channels_unsubscribe",
        "groups_access",
        "groups_add",
        "groups_ban",
        "groups_create",
        "groups_decide",
        "groups_follow",
        "groups_follows",
        "groups_join",
        "groups_list",
        "groups_remove",
        "groups_requests",
        "groups_send",
        "groups_show",
        "groups_unban",
        "groups_unfollow",
        "inbox_ack",
        "inbox_poll",
        "inbox_watch",
        "messages",
        "send",
    ];
    expected.sort();
    assert_eq!(names, expected);
    let making = [
        "contacts_request",
        "send",
        "groups_create",
        "groups_add",
        "groups_remove",
        "groups_ban",
        "groups_unban",
        "groups_send",
        "groups_access",
        "groups_join",
        "channels_create",
        "channels_retention",
        "channels_subscribe",
        "channels_unsubscribe",
        "channels_reseed",
    ];
    for tool in &tools {
        let name = tool["name"].as_str().unwrap();
        assert!(
            tool["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{name}"
        );
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object", "{name}");
        assert_eq!(schema["additionalProperties"], false, "{name}");
        if making.contains(&name) {
            assert!(
                schema["required"]
                    .as_array()
                    .is_some_and(|r| r.contains(&json!("operationId"))),
                "{name}"
            );
        }
    }
    let made = host.ok(
        "groups_create",
        json!({"name": "Solo", "members": [], "operationId": "g-1"}),
    );
    assert_eq!(made["role"], "owner");
    // Arguments that do not fit are refused, and nothing is sent.
    for bad in [
        json!({"group": "Solo", "text": "without an operation"}),
        json!({"group": "Solo", "operationId": "m-0", "text": "extra", "extra": 1}),
    ] {
        let refused = host.tool("groups_send", bad);
        assert!(
            refused.get("error").is_some() || refused["result"]["isError"] == true,
            "{refused}"
        );
    }
    let sent = host.ok(
        "groups_send",
        json!({"group": "Solo", "operationId": "m-1", "text": "from MCP"}),
    );
    assert!(sent["messageId"].is_string(), "{sent}");
    let history = host.ok("messages", json!({"with": "Solo"}));
    assert_eq!(
        history["messages"].as_array().unwrap().len(),
        1,
        "{history}"
    );
    assert_eq!(history["messages"][0]["text"], "from MCP");
    assert_eq!(host.ok("contacts_list", json!({})), json!({"contacts": []}));
    // Open groups and discovery: opening a group needs the owner's
    // confirmation, and discovery the daemon's service.
    assert_eq!(host.ok("groups_follows", json!({})), json!({"follows": []}));
    let unconfirmed = host.refused(
        "groups_access",
        json!({"group": "Solo", "to": "public", "operationId": "o-1"}),
    );
    assert_eq!(unconfirmed["code"], "confirmation_required");
    // The host asks the owner before this tool; confirmed, it commits.
    let opened = host.ok(
        "groups_access",
        json!({"group": "Solo", "to": "public", "confirm": true, "operationId": "o-1"}),
    );
    assert!(opened["commit"].is_string(), "{opened}");
    let unconfigured = host.refused("discover_search", json!({"query": "rust"}));
    assert_eq!(
        (
            unconfigured["code"].as_str(),
            unconfigured["retryable"].as_bool()
        ),
        (Some("directory_not_configured"), Some(false)),
        "{unconfigured}"
    );
    assert_eq!(
        host.ok("groups_list", json!({}))["groups"][0]["name"],
        "Solo"
    );
    // Channels: a public one only when confirmed; its history kept for
    // ever, as days or "forever".
    let unconfirmed = host.refused(
        "channels_create",
        json!({"name": "News", "access": "public", "operationId": "c-1"}),
    );
    assert_eq!(unconfirmed["code"], "confirmation_required");
    let news = host.ok(
        "channels_create",
        json!({"name": "News", "access": "public", "confirm": true, "operationId": "c-1"}),
    );
    assert_eq!(
        (&news["kind"], &news["access"]),
        (&json!("channel"), &json!("public")),
        "{news}"
    );
    let kept = host.ok(
        "channels_retention",
        json!({"channel": "News", "days": "forever", "operationId": "r-1"}),
    );
    assert!(kept["commit"].is_string(), "{kept}");
    let someone = format!("ain1{}", "ab".repeat(32));
    let pending = host.refused(
        "contacts_request",
        json!({"networkId": someone, "name": "S", "operationId": "c-1"}),
    );
    assert_eq!(
        (pending["code"].as_str(), pending["retryable"].as_bool()),
        (Some("network_unavailable"), Some(true)),
        "{pending}"
    );
    let unknown = host.refused(
        "send",
        json!({"to": "Nobody", "operationId": "m-2", "text": "?"}),
    );
    assert_eq!(unknown["code"], "unknown_contact");
    assert_eq!(unknown["retryable"], false);
    // Tools the spec leaves to the owner's CLI are not there.
    let absent = host.tool("grants_create", json!({}));
    assert!(absent.get("error").is_some(), "{absent}");
    host.close();
    // What the tools did is the profile's.
    assert_eq!(
        alice.ok(&["messages", "--with", "Solo"], None)[0]["text"],
        "from MCP"
    );
}

/// `daemon status` tells how far the node reaches the network, the first
/// thing to read when an agent's messages do not go out (the fields are
/// mapped by `host::network_status`, tested on a testnet snapshot).
#[test]
fn daemon_status_tells_how_far_the_node_reaches_the_network() {
    let (alice, _bob, _, _) = connected();
    wait("Alice's status counts Bob's connection", || {
        alice.ok(&["daemon", "status"], None)["network"]["connectedPeers"].as_u64() >= Some(1)
    });
    let network = alice.ok(&["daemon", "status"], None)["network"].clone();
    assert!(network["bootstrap"].is_string(), "{network}");
    assert!(network["holders"].is_u64(), "{network}");
    assert!(network["failures"].is_object(), "{network}");
}

/// Files by path over HTTP, any other path 404: a network preset and the
/// builds a release in it names.
struct ReleaseServer {
    base: String,
    files: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
    hits: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, usize>>>,
}

impl ReleaseServer {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let files = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::<
            String,
            Vec<u8>,
        >::new()));
        let hits = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::<
            String,
            usize,
        >::new()));
        let (served, count) = (files.clone(), hits.clone());
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut byte = [0u8; 1];
                while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                    request.push(byte[0]);
                }
                let path = String::from_utf8_lossy(&request)
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                *count.lock().unwrap().entry(path.clone()).or_default() += 1;
                let (status, body) = match served.lock().unwrap().get(&path).cloned() {
                    Some(body) => ("200 OK", body),
                    None => ("404 Not Found", Vec::new()),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(&body);
            }
        });
        Self { base, files, hits }
    }
    fn put(&self, path: &str, bytes: &[u8]) -> String {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), bytes.to_vec());
        format!("{}{path}", self.base)
    }
    fn hits(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }
    /// Serves the network `net-a` at `serial`, with `release` if any.
    fn preset(&self, serial: u64, release: Option<Value>) {
        let peer = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let mut preset = json!({
            "network": "net-a",
            "name": "Network net-a",
            "serial": serial,
            "bootstrap": [format!("/ip4/127.0.0.1/tcp/9/p2p/{peer}")],
            "identityServer": closed_identity_server(),
        });
        if let Some(release) = release {
            preset["release"] = release;
        }
        let signed = agentic_node::network_preset::sign(&preset.to_string(), &PRESET_SEED).unwrap();
        self.put("/network.json", signed.as_bytes());
    }
    /// A profile following this server's preset, running `kaiki`.
    fn owner(&self, kaiki: &Path) -> Owner {
        let mut owner = Owner::with_preset(
            &format!("{}/network.json", self.base),
            &agentic_node::network_preset::public_key(&PRESET_SEED),
        );
        owner.kaiki = kaiki.to_owned();
        owner
    }
}

const RELEASE_ARCHIVE: &str = "/kaiki-cli-test.tar.gz";

/// `version` with the build `cli-test` at the server's archive, announced
/// with the hash of `bytes`.
fn test_release(server: &ReleaseServer, version: &str, bytes: &[u8]) -> Value {
    json!({
        "version": version,
        "builds": {"cli-test": {
            "url": format!("{}{RELEASE_ARCHIVE}", server.base),
            "sha256": hex::encode(<sha2::Sha256 as sha2::Digest>::digest(bytes)),
        }},
    })
}

/// The command line as install.sh leaves it, made of this build's binaries
/// and the marker naming the build `cli-test`: its `bin` directory.
fn cli_install() -> (TempDir, std::path::PathBuf) {
    let root = tempfile::Builder::new()
        .prefix("kaiki-install-")
        .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap();
    let bin = root.path().join("share/kaiki/bin");
    fs::create_dir_all(&bin).unwrap();
    for (name, from) in [
        ("kaiki", env!("CARGO_BIN_EXE_kaiki")),
        (
            "kaiki-agentic-node",
            env!("CARGO_BIN_EXE_kaiki-agentic-node"),
        ),
    ] {
        if fs::hard_link(from, bin.join(name)).is_err() {
            fs::copy(from, bin.join(name)).unwrap();
        }
    }
    fs::write(bin.join("install.json"), r#"{"build":"cli-test"}"#).unwrap();
    (root, bin)
}

/// The published build: scripts that run this build's binaries, the marker,
/// and a file only the new build has.
fn cli_archive() -> Vec<u8> {
    let staging = TempDir::new().unwrap();
    let dir = staging.path().join("kaiki");
    fs::create_dir_all(&dir).unwrap();
    for (name, target) in [
        ("kaiki", env!("CARGO_BIN_EXE_kaiki")),
        (
            "kaiki-agentic-node",
            env!("CARGO_BIN_EXE_kaiki-agentic-node"),
        ),
    ] {
        fs::write(
            dir.join(name),
            format!("#!/bin/sh\nexec '{target}' \"$@\"\n"),
        )
        .unwrap();
        fs::set_permissions(dir.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    // The daemon's former name, for kaiki 0.2.4 and older (scripts/pack-cli.sh).
    std::os::unix::fs::symlink("kaiki-agentic-node", dir.join("agentic-node")).unwrap();
    fs::write(dir.join("install.json"), r#"{"build":"cli-test"}"#).unwrap();
    fs::write(dir.join("NEW"), "the new build").unwrap();
    // Packed as scripts/publish-cli.sh packs it: no macOS metadata entries.
    let out = staging.path().join("kaiki.tar.gz");
    let bsd = Command::new("tar")
        .arg("--version")
        .output()
        .unwrap()
        .stdout
        .starts_with(b"bsdtar");
    let mut tar = Command::new("tar");
    tar.env("COPYFILE_DISABLE", "1");
    if bsd {
        tar.args(["--no-xattrs", "--no-mac-metadata"]);
    }
    assert!(
        tar.arg("-czf")
            .arg(&out)
            .arg("-C")
            .arg(staging.path())
            .arg("kaiki")
            .status()
            .unwrap()
            .success()
    );
    fs::read(out).unwrap()
}

/// File names and sizes under `dir`, and what else is beside it.
fn install_state(bin: &Path) -> (Vec<(String, u64)>, Vec<String>) {
    let mut files: Vec<_> = fs::read_dir(bin)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into_string().unwrap(),
                entry.metadata().unwrap().len(),
            )
        })
        .collect();
    files.sort();
    let mut beside: Vec<_> = fs::read_dir(bin.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    beside.sort();
    (files, beside)
}

/// An install of `kaiki` learns of a newer release from the preset (when
/// its daemon starts, and between starts every twelve hours), says so in
/// every answer until the owner skips it, and replaces itself with the
/// announced build when asked: the daemon starts again from the new build,
/// and the skill installed follows it. A build other than the announced one
/// changes nothing, and the daemon keeps running.
#[test]
fn kaiki_tells_of_a_newer_release_and_updates_itself_when_asked() {
    let version = env!("CARGO_PKG_VERSION");
    let server = ReleaseServer::start();
    let archive = cli_archive();
    server.put(RELEASE_ARCHIVE, &archive);
    server.preset(1, Some(test_release(&server, "99.0.0", &archive)));
    let (_root, bin) = cli_install();
    let alice = server.owner(&bin.join("kaiki"));
    alice.ok(&["init", "--name", "Alice"], None);
    let (before, _) = daemon_with(&alice, "bin/kaiki-agentic-node serve");

    let (code, envelope) = alice.run(&["contacts", "list"], None);
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(
        envelope["update"],
        json!({"current": version, "latest": "99.0.0"}),
        "{envelope}"
    );
    let presets = server.hits("/network.json");
    let check = alice.ok(&["update", "--check"], None);
    assert_eq!(server.hits("/network.json"), presets + 1, "a check asks");
    assert_eq!(
        (
            &check["current"],
            &check["latest"],
            &check["available"],
            &check["skipped"],
            &check["error"]
        ),
        (
            &json!(version),
            &json!("99.0.0"),
            &json!(true),
            &json!(false),
            &Value::Null
        ),
        "{check}"
    );
    assert!(check["checkedAt"].is_u64(), "{check}");

    // Between daemon starts, a command asks again once twelve hours passed.
    server.preset(2, Some(test_release(&server, "99.1.0", &archive)));
    let presets = server.hits("/network.json");
    alice.ok(&["contacts", "list"], None);
    assert_eq!(server.hits("/network.json"), presets);
    let kept = alice.dir.path().join("release.json");
    let mut release: Value = serde_json::from_slice(&fs::read(&kept).unwrap()).unwrap();
    release["checkedAt"] = json!(release["checkedAt"].as_u64().unwrap() - 12 * 3600 - 1);
    fs::write(&kept, release.to_string()).unwrap();
    let (_, envelope) = alice.run(&["contacts", "list"], None);
    assert_eq!(server.hits("/network.json"), presets + 1);
    assert_eq!(envelope["update"]["latest"], "99.1.0", "{envelope}");

    // Skipped, it is quiet; still there when asked.
    assert_eq!(alice.ok(&["update", "--skip"], None)["skipped"], true);
    let (_, envelope) = alice.run(&["contacts", "list"], None);
    assert!(envelope.get("update").is_none(), "{envelope}");
    assert_eq!(alice.ok(&["update", "--check"], None)["skipped"], true);

    // A skill installed before the update, now out of date.
    let skill = alice.ok(&["skill", "install"], None)["path"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::write(&skill, "an older skill").unwrap();

    // Bytes other than the announced ones change nothing.
    let installed = install_state(&bin);
    server.preset(3, Some(test_release(&server, "99.2.0", b"another build")));
    alice.fails(&["update"], None, 3, "hash_mismatch");
    assert_eq!(install_state(&bin), installed);
    assert_eq!(
        daemon_with(&alice, "bin/kaiki-agentic-node serve").0,
        before
    );

    server.preset(4, Some(test_release(&server, "99.2.0", &archive)));
    let updated = alice.ok(&["update"], None);
    assert_eq!(
        updated,
        json!({"updated": true, "from": version, "to": "99.2.0", "restarted": true})
    );
    assert_eq!(
        fs::read_to_string(bin.join("NEW")).unwrap(),
        "the new build"
    );
    // The new build's node: the one its script runs.
    let node = format!("{} serve", env!("CARGO_BIN_EXE_kaiki-agentic-node"));
    let (after, command) = daemon_with(&alice, &node);
    assert_ne!(after, before, "{command}");
    assert_eq!(alice.ok(&["daemon", "status"], None)["name"], "Alice");
    assert_eq!(
        fs::read_to_string(&skill).unwrap(),
        alice.ok(&["skill", "show"], None)["text"].as_str().unwrap()
    );
    assert_eq!(install_state(&bin).1, ["bin"]);
}

/// Only an install from the archive replaces itself, only with a build
/// for it, and nothing is fetched when there is nothing newer.
#[test]
fn kaiki_replaces_itself_only_with_the_announced_build() {
    let version = env!("CARGO_PKG_VERSION");
    let server = ReleaseServer::start();
    let archive = cli_archive();
    server.put(RELEASE_ARCHIVE, &archive);
    let (_root, bin) = cli_install();
    let before = install_state(&bin);
    let owner = server.owner(&bin.join("kaiki"));

    // Nothing announced, or this very version: nothing to fetch.
    server.preset(1, None);
    assert_eq!(
        owner.ok(&["update"], None),
        json!({"updated": false, "current": version, "latest": null})
    );
    server.preset(2, Some(test_release(&server, version, &archive)));
    assert_eq!(
        owner.ok(&["update"], None),
        json!({"updated": false, "current": version, "latest": version})
    );
    owner.fails(&["update", "--skip"], None, 3, "no_update");
    assert_eq!(server.hits(RELEASE_ARCHIVE), 0);

    // A preset that cannot be fetched is no answer: try again later.
    let mut away = Owner::with_preset(
        &format!("{}/gone.json", server.base),
        &agentic_node::network_preset::public_key(&PRESET_SEED),
    );
    away.kaiki = bin.join("kaiki");
    away.fails(&["update"], None, 4, "unavailable");

    // A release without a build for this install.
    let mut other = test_release(&server, "99.0.0", &archive);
    other["builds"] = json!({"cli-other": other["builds"]["cli-test"].clone()});
    server.preset(4, Some(other));
    owner.fails(&["update"], None, 3, "no_build");
    assert_eq!(server.hits(RELEASE_ARCHIVE), 0);
    assert_eq!(install_state(&bin), before);

    // This build, as cargo left it, does not replace itself, nor asks.
    server.preset(5, Some(test_release(&server, "99.0.0", &archive)));
    let presets = server.hits("/network.json");
    let cargo = server.owner(Path::new(env!("CARGO_BIN_EXE_kaiki")));
    cargo.fails(&["update"], None, 3, "not_updatable");
    assert_eq!(
        (server.hits("/network.json"), server.hits(RELEASE_ARCHIVE)),
        (presets, 0)
    );
    // Without a daemon, an update starts none.
    assert_eq!(
        owner.ok(&["update"], None),
        json!({"updated": true, "from": version, "to": "99.0.0", "restarted": false})
    );
    assert_eq!(owner.ok(&["daemon", "status"], None)["running"], false);
}

/// `kaiki` on the profile it opens by itself (no `--data-dir`, no
/// `AGENTIC_DATA_DIR`) in a home of its own, with the file secrets and the
/// password in a file.
struct DefaultProfile {
    home: TempDir,
}

impl DefaultProfile {
    fn new() -> Self {
        // Short: the profile's socket is under Library/Application Support.
        let home = tempfile::Builder::new()
            .prefix("ain-h")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(home.path().join(".kaiki-password"), PASSWORD).unwrap();
        fs::set_permissions(
            home.path().join(".kaiki-password"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        Self { home }
    }

    fn password_file(&self) -> std::path::PathBuf {
        self.home.path().join(".kaiki-password")
    }

    /// `kaiki` with `args`, as `change` leaves its environment: its exit
    /// code and JSON.
    fn run(&self, kaiki: &Path, args: &[&str], change: impl FnOnce(&mut Command)) -> (i32, Value) {
        let mut command = Command::new(kaiki);
        command
            .args(args)
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join(".config"))
            .env("XDG_DATA_HOME", self.home.path().join(".local/share"))
            .env_remove("AGENTIC_DATA_DIR")
            .env("AGENTIC_SECRETS", "file")
            .env("AGENTIC_PASSWORD_FILE", self.password_file())
            .env_remove("AGENTIC_PASSWORD")
            .env("AGENTIC_NETWORK_PRESET", "off")
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        change(&mut command);
        let output = command.output().unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let value = serde_json::from_str(text.trim())
            .unwrap_or_else(|_| panic!("not one JSON envelope: {text:?}"));
        (output.status.code().unwrap_or(-1), value)
    }

    fn ok_with(&self, kaiki: &Path, args: &[&str], change: impl FnOnce(&mut Command)) -> Value {
        let (code, value) = self.run(kaiki, args, change);
        assert_eq!(code, 0, "{args:?}: {value}");
        value["result"].clone()
    }

    fn ok(&self, kaiki: &Path, args: &[&str]) -> Value {
        self.ok_with(kaiki, args, |_| {})
    }

    const START: [&str; 4] = ["daemon", "start", "--listen", "/ip4/127.0.0.1/tcp/0"];

    /// Everything under the home's places of login items.
    fn login_items(&self) -> Vec<std::path::PathBuf> {
        let mut found = Vec::new();
        let mut dirs = vec![
            self.home.path().join("Library/LaunchAgents"),
            self.home.path().join(".config"),
        ];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir).into_iter().flatten() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    found.push(path);
                }
            }
        }
        found
    }
}

impl Drop for DefaultProfile {
    fn drop(&mut self) {
        let kaiki = Path::new(env!("CARGO_BIN_EXE_kaiki"));
        let _ = self.run(kaiki, &["daemon", "stop"], |_| {});
        let other = self.home.path().join("other");
        if other.is_dir() {
            let _ = self.run(kaiki, &["daemon", "stop"], |command| {
                command.env("AGENTIC_DATA_DIR", &other);
            });
        }
    }
}

/// An install of `kaiki` (install.sh's) puts the daemon of the profile it
/// opens by itself into the owner's login items whenever it starts it, by
/// any command: the job runs this install's `kaiki daemon start` with the
/// secrets backend and the password's file, never the password.
/// `kaiki autostart off` takes it out, and later starts leave it out;
/// `kaiki autostart on` puts it back. A build of kaiki (tests,
/// development), a profile named by `AGENTIC_DATA_DIR`, or a password given
/// only in `AGENTIC_PASSWORD` starts nothing at login.
#[test]
fn an_installed_kaiki_starts_the_daemon_at_login_until_the_owner_turns_it_off() {
    let owner = DefaultProfile::new();
    let build = Path::new(env!("CARGO_BIN_EXE_kaiki"));
    let (_install, bin) = cli_install();
    let installed = bin.join("kaiki");
    let none = Vec::<std::path::PathBuf>::new();

    owner.ok(build, &DefaultProfile::START);
    assert_eq!(owner.ok(build, &["autostart"])["state"], "off");
    assert_eq!(owner.login_items(), none);
    owner.ok(build, &["daemon", "stop"]);

    let other = owner.home.path().join("other");
    fs::create_dir(&other).unwrap();
    fs::set_permissions(&other, fs::Permissions::from_mode(0o700)).unwrap();
    let named = |command: &mut Command| {
        command.env("AGENTIC_DATA_DIR", &other);
    };
    owner.ok_with(&installed, &DefaultProfile::START, named);
    assert_eq!(owner.login_items(), none, "a named profile only when asked");
    owner.ok_with(&installed, &["daemon", "stop"], named);

    let password_only = |command: &mut Command| {
        command
            .env_remove("AGENTIC_PASSWORD_FILE")
            .env("AGENTIC_PASSWORD", PASSWORD);
    };
    let (code, refused) = owner.run(&installed, &["autostart", "on"], password_only);
    assert_eq!(
        (code, refused["error"]["code"].as_str()),
        (2, Some("password_file_required")),
        "{refused}"
    );
    owner.ok_with(&installed, &DefaultProfile::START, password_only);
    assert_eq!(owner.login_items(), none);
    owner.ok(&installed, &["daemon", "stop"]);

    // The first command an agent runs starts the daemon: that is enough.
    let flag_and_both = |command: &mut Command| {
        command
            .env_remove("AGENTIC_SECRETS")
            .env("AGENTIC_PASSWORD", PASSWORD);
    };
    owner.ok_with(
        &installed,
        &["--secrets", "file", "init", "--name", "Owner"],
        flag_and_both,
    );
    let on = owner.ok_with(&installed, &["autostart"], flag_and_both);
    assert_eq!(on["state"], "on", "{on}");
    let job = std::path::PathBuf::from(on["path"].as_str().unwrap());
    let home = fs::canonicalize(owner.home.path()).unwrap();
    assert!(
        job.starts_with(owner.home.path()) || job.starts_with(&home),
        "{}",
        job.display()
    );
    assert!(job.is_file(), "{:?}", owner.login_items());
    let text = fs::read_to_string(&job).unwrap();
    // Linux names a binary by its resolved path.
    let canonical = fs::canonicalize(&installed).unwrap();
    assert!(
        [installed.as_path(), canonical.as_path()]
            .iter()
            .any(|kaiki| text.contains(kaiki.to_str().unwrap())),
        "the install's kaiki is not in the job:\n{text}"
    );
    for part in [
        "daemon",
        "start",
        "AGENTIC_SECRETS",
        owner.password_file().to_str().unwrap(),
    ] {
        assert!(text.contains(part), "{part} is not in the job:\n{text}");
    }
    assert!(
        !text.contains(PASSWORD),
        "the password is in the job:\n{text}"
    );
    assert_eq!(
        text.matches("AGENTIC_PASSWORD").count(),
        text.matches("AGENTIC_PASSWORD_FILE").count(),
        "AGENTIC_PASSWORD is in the job:\n{text}"
    );

    assert_eq!(owner.ok(&installed, &["autostart", "off"])["state"], "off");
    assert!(!job.exists());
    owner.ok(&installed, &["daemon", "stop"]);
    owner.ok(&installed, &DefaultProfile::START);
    assert_eq!(owner.ok(&installed, &["autostart"])["state"], "off");
    assert!(!job.exists(), "a start does not put it back");

    assert_eq!(owner.ok(&installed, &["autostart", "on"])["state"], "on");
    assert!(job.is_file());
}
