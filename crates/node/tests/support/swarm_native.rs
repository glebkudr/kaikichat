//! Native spike of the mailbox swarm (phase 7 of
//! Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md, second half): real daemons and a
//! local anvil chain with the contracts deployed. Ten holders bond their
//! units, Alice buys a book, sends to Bob while he is offline and leaves;
//! three holders lose their disks and are repaired by the swarm; Bob comes
//! online and reads everything in order.
//!
//! Ignored by default: it needs anvil, forge and cast on PATH, which the
//! build wrapper provides.
//! `python3 scripts/build-storage.py run cargo test -p agentic-node --test processes -- --ignored native_swarm_spike --nocapture`
use super::*;

/// Anvil's first default account: deploys, bonds and pays.
const DEPLOYER: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const DEPLOYER_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
const TREASURY: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";
const POOL: &str = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC";
const ISSUER: &str = "0x90F79bf6EB2c4f870365E785982E1f101E93b906";
/// `agentic_node::NETWORK_DOMAIN`.
const DOMAIN: &str = "0xae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18";
/// $1.00 a book, in USDC units.
const PRICE_USDC: &str = "1000000";
/// The mock ETH/USD feed's $2500.00 (eight decimals): a book is 0.0004 ETH.
const RATE: &str = "250000000000";
const QUOTE: &str = "400000000000000";
/// A day: the mock feed is set once and must not go stale during a run.
const MAX_PRICE_AGE: &str = "86400";
const UNIT_BOND: &str = "101";
/// Stamps per book: enough for the longest run.
const BOOK_SIZE: &str = "1000";
const HOLDERS: usize = 10;
/// Messages sent; `AIN_SPIKE_MESSAGES` overrides it.
const MESSAGES: usize = 20;
const LOST: [usize; 3] = [1, 4, 7];

struct Anvil {
    child: Child,
    url: String,
}

impl Drop for Anvil {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn anvil() -> Anvil {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let child = Command::new("anvil")
        .args(["--port", &port.to_string(), "--block-time", "1", "--silent"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("anvil on PATH");
    let url = format!("http://127.0.0.1:{port}");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Command::new("cast")
        .args(["block-number", "--rpc-url", &url])
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        assert!(Instant::now() < deadline, "anvil did not start");
        thread::sleep(Duration::from_millis(200));
    }
    Anvil { child, url }
}

fn run(program: &str, args: &[&str]) -> String {
    let output = Command::new(program).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn deploy(url: &str, contract: &str, arguments: &[&str]) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../contracts");
    let mut args = vec![
        "create",
        "--root",
        root,
        "--rpc-url",
        url,
        "--private-key",
        DEPLOYER,
        "--broadcast",
    ];
    let solc = std::env::var("AIN_SOLC").ok();
    if let Some(solc) = &solc {
        args.extend(["--use", solc]);
    }
    args.push(contract);
    if !arguments.is_empty() {
        args.push("--constructor-args");
        args.extend(arguments);
    }
    let output = run("forge", &args);
    output
        .lines()
        .find_map(|line| line.strip_prefix("Deployed to: "))
        .unwrap_or_else(|| panic!("no address: {output}"))
        .trim()
        .to_owned()
}

fn info(node: &Node) -> Value {
    node.call("node_info", json!({}))
}

/// Poll `check` every half second until it holds or `limit` passes.
fn until(limit: Duration, what: &str, check: impl Fn() -> bool) -> Duration {
    let started = Instant::now();
    while !check() {
        assert!(started.elapsed() < limit, "{what} within {limit:?}");
        thread::sleep(Duration::from_millis(500));
    }
    started.elapsed()
}

fn route(node: &Node) -> String {
    format!("{}/p2p/{}", node.listeners[0], node.peer)
}

fn texts(node: &Node, conversation: &str, own: bool) -> Vec<String> {
    node.snapshot()["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == conversation)
        .map(|c| {
            c["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|m| m["own"] == own)
                .map(|m| m["text"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A local chain with the contracts and ten bonded holders whose
/// directories are complete, and the flags a profile's daemon joins with.
struct Network {
    chain: Anvil,
    splitter: String,
    shop: String,
    usdc: String,
    holders: Vec<Node>,
    flags: Vec<String>,
    started: Instant,
    directory: Duration,
}

fn network(name: &str) -> Network {
    let chain = anvil();
    let usdc = deploy(&chain.url, "test/PaymentMocks.sol:MockUsdc", &[]);
    let now = run(
        "cast",
        &[
            "block",
            "latest",
            "-f",
            "timestamp",
            "--rpc-url",
            &chain.url,
        ],
    );
    let feed = deploy(
        &chain.url,
        "test/PaymentMocks.sol:MockEthUsdFeed",
        &[RATE, &now],
    );
    let splitter = deploy(
        &chain.url,
        "src/RoyaltySplitter.sol:RoyaltySplitter",
        &[&format!("[{TREASURY},{POOL}]"), "[1000,9000]"],
    );
    let shop = deploy(
        &chain.url,
        "src/BookShop.sol:BookShop",
        &[
            DOMAIN,
            PRICE_USDC,
            BOOK_SIZE,
            "2592000",
            &splitter,
            &usdc,
            &feed,
            MAX_PRICE_AGE,
        ],
    );
    let issuer = deploy(
        &chain.url,
        "src/GrantIssuer.sol:GrantIssuer",
        &[DEPLOYER_ADDRESS, DOMAIN, "100", "30", "1000", ISSUER],
    );
    let genesis = run("cast", &["keccak", name]);
    let registry = deploy(
        &chain.url,
        "src/NodeRegistry.sol:NodeRegistry",
        &[&genesis, UNIT_BOND, "60", "120", "600", "3"],
    );
    let flags = |bootstrap: Option<String>| {
        let mut flags: Vec<String> = [
            "--chain-rpc",
            &chain.url,
            "--chain-id",
            "31337",
            "--book-shop",
            &shop,
            "--grant-issuer",
            &issuer,
            "--registry",
            &registry,
            "--chain-confirmations",
            "1",
        ]
        .map(String::from)
        .to_vec();
        if let Some(bootstrap) = bootstrap {
            flags.extend(["--bootstrap".to_owned(), bootstrap]);
        }
        flags
    };
    let tcp = ["/ip4/127.0.0.1/tcp/0"];
    // Holders learn their commitments, bond them, and start again so the
    // first registry read lists them.
    let mut holders = vec![Node::start_with_args(&tcp, flags(None))];
    let hub = route(&holders[0]);
    for _ in 1..HOLDERS {
        holders.push(Node::start_with_args(&tcp, flags(Some(hub.clone()))));
    }
    for holder in &mut holders {
        let commitment = info(holder)["directory"]["ownCommitment"]
            .as_str()
            .unwrap()
            .to_owned();
        holder.kill();
        run(
            "cast",
            &[
                "send",
                &registry,
                "bond(bytes32)",
                &format!("0x{commitment}"),
                "--value",
                UNIT_BOND,
                "--private-key",
                DEPLOYER,
                "--rpc-url",
                &chain.url,
            ],
        );
    }
    // The last bond confirmed (one confirmation) before the first read.
    let bonded: u64 = run("cast", &["block-number", "--rpc-url", &chain.url])
        .parse()
        .unwrap();
    until(Duration::from_secs(20), "two more blocks", || {
        run("cast", &["block-number", "--rpc-url", &chain.url])
            .parse::<u64>()
            .unwrap()
            >= bonded + 2
    });
    let started = Instant::now();
    for holder in &mut holders {
        holder.launch();
    }
    let flags = flags(Some(hub));
    let begun = Instant::now();
    until(
        Duration::from_secs(180),
        "holder directories complete",
        || {
            holders
                .iter()
                .all(|node| info(node)["mailboxSwarm"]["holders"] == HOLDERS)
        },
    );
    let directory = begun.elapsed();
    for holder in &holders {
        assert_eq!(
            info(holder)["mailboxHolder"]["unit"],
            info(holder)["directory"]["ownCommitment"]
        );
    }
    Network {
        chain,
        splitter,
        shop,
        usdc,
        holders,
        flags,
        started,
        directory,
    }
}

impl Network {
    /// A profile's daemon on this network. Holders serve it only once it
    /// has a book to show (access by book), so it gets the directory after
    /// buying one (`directory`).
    fn join(&self) -> Node {
        Node::start_with_args(&["/ip4/127.0.0.1/tcp/0"], self.flags.clone())
    }

    /// Wait for `node`'s directory to be complete.
    fn directory(&self, node: &Node) {
        until(Duration::from_secs(180), "a profile's directory", || {
            info(node)["mailboxSwarm"]["holders"] == HOLDERS
        });
    }

    /// `node`'s payment request for a book.
    fn payment(&self, node: &Node) -> Value {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let answer = rpc(&node.socket(), TOKEN, "coins_buy", json!({})).unwrap();
            if let Some(result) = answer.get("result") {
                assert_eq!(result["priceUsdc"], PRICE_USDC, "{result}");
                return result.clone();
            }
            assert_eq!(answer["error"]["code"], "chain_pending", "{answer}");
            assert!(Instant::now() < deadline, "no shop terms");
            thread::sleep(Duration::from_millis(500));
        }
    }

    /// Send one of the node's payment steps as the deployer.
    fn pay(&self, step: &Value, value: Option<&str>) {
        let mut args = vec![
            "send",
            step["to"].as_str().unwrap(),
            step["calldata"].as_str().unwrap(),
            "--private-key",
            DEPLOYER,
            "--rpc-url",
            &self.chain.url,
        ];
        if let Some(value) = value {
            args.extend(["--value", value]);
        }
        run("cast", &args);
    }

    /// `node` buys a book in ETH with the calldata it handed out; its daemon
    /// notices the purchase.
    fn fund(&self, node: &Node) -> Duration {
        let payment = self.payment(node);
        assert_eq!(payment["eth"]["quote"], QUOTE, "{payment}");
        self.pay(&payment["eth"], payment["eth"]["value"].as_str());
        self.noticed(node)
    }

    /// `node` buys a book in USDC: the deployer is minted the price, allows
    /// the shop and buys, with the steps the node handed out.
    fn fund_in_usdc(&self, node: &Node) -> Duration {
        let payment = self.payment(node);
        // forge prints the checksummed address, the node lowercase hex.
        assert!(
            payment["usdc"]["token"]
                .as_str()
                .is_some_and(|token| token.eq_ignore_ascii_case(&self.usdc)),
            "{payment}"
        );
        run(
            "cast",
            &[
                "send",
                &self.usdc,
                "mint(address,uint256)",
                DEPLOYER_ADDRESS,
                PRICE_USDC,
                "--private-key",
                DEPLOYER,
                "--rpc-url",
                &self.chain.url,
            ],
        );
        self.pay(&payment["usdc"]["approve"], None);
        self.pay(&payment["usdc"]["buy"], None);
        self.noticed(node)
    }

    fn noticed(&self, node: &Node) -> Duration {
        until(Duration::from_secs(120), "the purchase noticed", || {
            node.call("coins_balance", json!({}))["remaining"] == 1_000
        })
    }
}

#[test]
#[ignore = "needs anvil, forge and cast; run through the build wrapper"]
fn native_swarm_spike() {
    let messages: usize = std::env::var("AIN_SPIKE_MESSAGES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(MESSAGES);
    let mut net = network("native-swarm-spike");
    let alice = net.join();
    let mut bob = net.join();
    let conversation = connect(&alice, &bob, None);
    // Alice buys a book and her node notices the purchase.
    let bought = net.fund(&alice);
    let treasury = run(
        "cast",
        &[
            "call",
            &net.splitter,
            "credits(address)(uint256)",
            TREASURY,
            "--rpc-url",
            &net.chain.url,
        ],
    );
    // A tenth of the ETH she paid at the quote goes to the treasury.
    let share = (QUOTE.parse::<u128>().unwrap() / 10).to_string();
    assert_eq!(treasury.split_whitespace().next(), Some(share.as_str()));
    // Bob only reads, but a node reads nothing without a book to show.
    net.fund(&bob);
    for node in [&alice, &bob] {
        net.directory(node);
    }
    // Bob is offline; Alice sends and leaves once everything is stored.
    bob.kill();
    let sent_at = Instant::now();
    for n in 0..messages {
        alice.send(
            &conversation,
            &format!("offline {n}"),
            &format!("offline-{n}"),
        );
    }
    let stored = until(Duration::from_secs(300), "every message stored", || {
        let snapshot = alice.snapshot();
        let own: Vec<&Value> = snapshot["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == conversation)
            .map(|c| {
                c["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|m| m["own"] == true)
                    .collect()
            })
            .unwrap_or_default();
        own.len() == messages && own.iter().all(|m| m["delivery"]["phase"] == "delivered")
    });
    let alice_info = info(&alice);
    drop(alice);
    // Three holders lose their disks and come back empty.
    for index in LOST {
        let holder = &mut net.holders[index];
        holder.kill();
        let disk = holder.root.path().join("profile.db.mailbox.db");
        assert!(disk.exists(), "no mailbox store at {}", disk.display());
        fs::remove_file(&disk).unwrap();
        holder.launch();
    }
    let repair_started = Instant::now();
    let holders = &net.holders;
    let repaired = until(
        Duration::from_secs(600),
        "the lost holders repaired",
        || {
            LOST.iter().all(|index| {
                info(&holders[*index])["mailboxHolder"]["tickets"]
                    .as_u64()
                    .is_some_and(|tickets| tickets >= messages as u64)
            })
        },
    );
    // Bob comes online and reads everything, in order.
    bob.launch();
    let expected: Vec<String> = (0..messages).map(|n| format!("offline {n}")).collect();
    let read = until(Duration::from_secs(300), "Bob read everything", || {
        texts(&bob, &conversation, false) == expected
    });
    let report = json!({
        "holders": HOLDERS,
        "messages": messages,
        "lostDisks": LOST.len(),
        "seconds": {
            "directoryComplete": net.directory.as_secs_f64(),
            "purchaseNoticed": bought.as_secs_f64(),
            "allStored": stored.as_secs_f64(),
            "repaired": repaired.as_secs_f64(),
            "readByRecipient": read.as_secs_f64(),
            "sinceHoldersStarted": net.started.elapsed().as_secs_f64(),
            "sinceFirstSend": sent_at.elapsed().as_secs_f64(),
            "sinceRepairStarted": repair_started.elapsed().as_secs_f64(),
        },
        "sender": {
            "sent": alice_info["mailboxSwarm"]["sent"],
            "failureKinds": alice_info["mailboxSwarm"]["failureKinds"],
            "chain": alice_info["chain"],
        },
        "lostHolders": LOST.iter().map(|index| info(&holders[*index])["mailboxSwarm"]["replication"].clone()).collect::<Vec<_>>(),
    });
    println!("native-swarm-spike {report}");
}

fn network_id(node: &Node) -> String {
    node.snapshot()["identity"]["networkId"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// An owner IPC call asked again while the node looks up cards.
fn once_cards_are_read(node: &Node, method: &str, request: Value) -> Value {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let answer = rpc(&node.socket(), TOKEN, method, request.clone()).unwrap();
        if answer["error"]["code"] != "card_pending" {
            return answer;
        }
        assert!(Instant::now() < deadline, "cards never read: {answer}");
        thread::sleep(Duration::from_millis(500));
    }
}

fn group(node: &Node, id: &str) -> Value {
    rpc(&node.socket(), TOKEN, "group", json!({ "groupId": id })).unwrap()
}

/// Contact by id and groups with real daemons on a local chain: three
/// profiles that never met reach each other by network id, talk in a group,
/// and a removal ordered by the notaries leaves the removed member reading
/// nothing new.
#[test]
#[ignore = "needs anvil, forge and cast; run through the build wrapper"]
fn native_contacts_and_groups() {
    let net = network("native-contacts-and-groups");
    let (alice, bob, carol) = (net.join(), net.join(), net.join());
    for (name, node) in [("Alice", &alice), ("Bob", &bob), ("Carol", &carol)] {
        node.profile(name);
    }
    // Alice and Carol pay in ETH at the rate, Bob in USDC.
    net.fund(&alice);
    net.fund_in_usdc(&bob);
    net.fund(&carol);
    for node in [&alice, &bob, &carol] {
        net.directory(node);
    }
    // Every card published before anyone asks.
    let published = until(Duration::from_secs(180), "cards published", || {
        [&alice, &bob, &carol]
            .iter()
            .all(|node| info(node)["mailboxSwarm"]["intro"]["published"].is_string())
    });
    // Alice asks Bob by his id; his default policy lets her in.
    let bob_id = network_id(&bob);
    let asked_at = Instant::now();
    let asked = once_cards_are_read(
        &alice,
        "request_contact",
        json!({"networkId": bob_id, "name": "Bob", "operationId": "ask-1"}),
    );
    let conversation = asked["result"]["conversationId"]
        .as_str()
        .unwrap_or_else(|| panic!("{asked}"))
        .to_owned();
    let contact = until(Duration::from_secs(180), "Bob has Alice", || {
        bob.snapshot()["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == conversation.as_str())
    });
    alice.send(&conversation, "hi by ID", "m-1");
    let talked = until(Duration::from_secs(120), "Bob reads Alice", || {
        texts(&bob, &conversation, false) == ["hi by ID"]
    });
    // A group of the three, made of ids.
    let made = once_cards_are_read(
        &alice,
        "create_group",
        json!({"name": "Team", "members": [bob_id, network_id(&carol)], "operationId": "g-1"}),
    );
    let g = made["result"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{made}"))
        .to_owned();
    let joined = until(Duration::from_secs(180), "both joined", || {
        [&bob, &carol]
            .iter()
            .all(|node| group(node, &g).get("result").is_some())
    });
    alice.send(&g, "hello everyone", "m-2");
    let heard = until(Duration::from_secs(120), "the group heard", || {
        [&bob, &carol]
            .iter()
            .all(|node| texts(node, &g, false) == ["hello everyone"])
    });
    // Carol is removed through the notaries.
    let epoch = group(&alice, &g)["result"]["epoch"].as_u64().unwrap();
    let changed = rpc(
        &alice.socket(),
        TOKEN,
        "change_group",
        json!({"groupId": g, "remove": [network_id(&carol)], "operationId": "c-1"}),
    )
    .unwrap();
    assert!(changed.get("result").is_some(), "{changed}");
    let decided = until(Duration::from_secs(180), "the removal decided", || {
        [&alice, &bob]
            .iter()
            .all(|node| group(node, &g)["result"]["epoch"] == epoch + 1)
            && group(&carol, &g)["result"]["members"] == json!([])
    });
    alice.send(&g, "without Carol", "m-3");
    let after = until(Duration::from_secs(120), "Bob reads on", || {
        texts(&bob, &g, false).len() == 2
    });
    thread::sleep(Duration::from_secs(10));
    assert_eq!(texts(&carol, &g, false), ["hello everyone"]);
    let report = json!({
        "holders": HOLDERS,
        "seconds": {
            "directoryComplete": net.directory.as_secs_f64(),
            "cardsPublished": published.as_secs_f64(),
            "contactJoined": contact.as_secs_f64(),
            "sinceAsked": asked_at.elapsed().as_secs_f64(),
            "firstMessageRead": talked.as_secs_f64(),
            "groupJoined": joined.as_secs_f64(),
            "groupMessageRead": heard.as_secs_f64(),
            "removalDecided": decided.as_secs_f64(),
            "messageAfterRemoval": after.as_secs_f64(),
        },
        "alice": {
            "sent": info(&alice)["mailboxSwarm"]["sent"],
            "intro": info(&alice)["mailboxSwarm"]["intro"],
            "groups": info(&alice)["mailboxSwarm"]["groups"],
            "failureKinds": info(&alice)["mailboxSwarm"]["failureKinds"],
        },
    });
    println!("native-contacts-and-groups {report}");
}

/// Holds a local network open for a real agent host (V1-AF08): writes the
/// daemon flags, the chain and the shop to `$AIN_AF08_DIR/network.json` and
/// runs until `$AIN_AF08_DIR/stop` appears. The profiles are the host's.
#[test]
#[ignore = "manual: the network of an agent-host acceptance run"]
fn af08_host_network() {
    let dir = PathBuf::from(std::env::var("AIN_AF08_DIR").expect("AIN_AF08_DIR"));
    let net = network("af08-host-network");
    let description = json!({
        "flags": net.flags,
        "chain": net.chain.url,
        "shop": net.shop,
        "priceUsdc": PRICE_USDC,
        "usdc": net.usdc,
        "payer": DEPLOYER,
    });
    fs::write(dir.join("network.json"), description.to_string()).unwrap();
    until(Duration::from_secs(6 * 3600), "the stop file", || {
        dir.join("stop").exists()
    });
}

/// The discovery service beside a holder of the network, as its binary runs.
struct DirectoryService {
    child: Child,
    url: String,
    _dir: TempDir,
}

impl Drop for DirectoryService {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl DirectoryService {
    /// Build and start `agentic-directory` checking stamps at `node`.
    fn beside(node: &Node) -> Self {
        run(
            env!("CARGO"),
            &["build", "--locked", "-p", "agentic-directory"],
        );
        let binary = std::env::current_exe()
            .unwrap()
            .parent()
            .and_then(|deps| deps.parent())
            .unwrap()
            .join("agentic-directory");
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| {
            let path = dir.path().join(name);
            fs::write(&path, text).unwrap();
            path
        };
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let url = format!("http://127.0.0.1:{port}");
        let child = Command::new(binary)
            .env("AIN_DIR_LISTEN", format!("127.0.0.1:{port}"))
            .env("AIN_DIR_PUBLIC_URL", &url)
            .env("AIN_DIR_DOMAIN", DOMAIN)
            .env("AIN_DIR_SIGNING_KEY_FILE", write("key", &"21".repeat(32)))
            .env("AIN_DIR_PEPPER_FILE", write("pepper", &"22".repeat(32)))
            .env("AIN_DIR_DATABASE", dir.path().join("directory.db"))
            .env(
                "AIN_DIR_GOOGLE_CLIENT_ID",
                "native.apps.googleusercontent.com",
            )
            .env(
                "AIN_DIR_GOOGLE_CLIENT_SECRET_FILE",
                write("google", "secret"),
            )
            .env("AIN_DIR_NODE_SOCKET", node.socket())
            .env("AIN_DIR_NODE_TOKEN_FILE", write("token", TOKEN))
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let service = Self {
            child,
            url,
            _dir: dir,
        };
        until(Duration::from_secs(20), "the directory listening", || {
            service.http("GET", "/v1/policy", None).0 == 200
        });
        service
    }

    /// A request over HTTP: its status and JSON body.
    fn http(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, Value) {
        let mut command = Command::new("curl");
        command.args(["-s", "-w", "\n%{http_code}", "-X", method]);
        if body.is_some() {
            command.args([
                "-H",
                "content-type: application/json",
                "--data-binary",
                "@-",
            ]);
        }
        let mut child = command
            .arg(format!("{}{path}", self.url))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(body) = body {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(body.to_string().as_bytes())
                .unwrap();
        } else {
            drop(child.stdin.take());
        }
        let output = child.wait_with_output().unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let (body, status) = text.rsplit_once('\n').unwrap_or(("", "0"));
        (
            status.trim().parse().unwrap_or(0),
            serde_json::from_str(body).unwrap_or(Value::Null),
        )
    }

    /// POST `body` until the node beside the service has read the payer's
    /// book from the chain.
    fn post_paid(&self, path: &str, body: &Value) -> (u16, Value) {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let answer = self.http("POST", path, Some(body));
            let retry = answer.1["error"] == "unknown_book" || answer.1["error"] == "book_required";
            if !retry || Instant::now() > deadline {
                return answer;
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}

fn result(answer: Value) -> Value {
    answer
        .get("result")
        .cloned()
        .unwrap_or_else(|| panic!("{answer}"))
}

/// Discovery with real daemons, a local chain and the service's binary
/// beside holder 0 (spec/discovery-v1.md): Alice opens a group and publishes
/// its card, paid at the holder; Bob finds it by interest with his book's
/// pass, follows it and reads Alice's post; a lookup of an address nobody
/// bound is paid and finds nobody.
#[test]
#[ignore = "needs anvil, forge and cast; run through the build wrapper"]
fn native_discovery() {
    let net = network("native-discovery");
    let (alice, bob) = (net.join(), net.join());
    alice.profile("Alice");
    bob.profile("Bob");
    net.fund(&alice);
    net.fund(&bob);
    for node in [&alice, &bob] {
        net.directory(node);
    }
    let service = DirectoryService::beside(&net.holders[0]);
    // Alice's open group.
    let made = result(once_cards_are_read(
        &alice,
        "create_group",
        json!({"name": "Rustaceans", "members": [], "operationId": "g-1"}),
    ));
    let g = made["id"].as_str().unwrap().to_owned();
    let epoch = made["epoch"].as_u64().unwrap();
    alice.call(
        "change_group",
        json!({"groupId": g, "access": "public", "operationId": "open"}),
    );
    until(Duration::from_secs(120), "the group opened", || {
        group(&alice, &g)["result"]["epoch"].as_u64() > Some(epoch)
    });
    assert_eq!(group(&alice, &g)["result"]["access"], "public");
    // Its card, paid with ten of Alice's stamps at holder 0.
    let card = alice.call(
        "discover_card",
        json!({"kind": "group", "groupId": g, "about": "Rust and agents", "tags": ["rust"], "langs": ["en"]}),
    );
    let (status, published) = service.post_paid(
        "/v1/cards",
        &json!({"card": card["card"], "stamps": card["stamps"], "grants": card["grants"]}),
    );
    assert_eq!(status, 201, "{published}");
    // Bob finds it with a pass of his book, and follows it.
    let pass = bob.call("discover_pass", json!({}));
    let (status, found) = service.post_paid(
        "/v1/search",
        &json!({"query": "rust", "pass": pass["pass"]}),
    );
    assert_eq!(status, 200, "{found}");
    let cards = found["cards"].as_array().unwrap();
    assert_eq!(cards.len(), 1, "{found}");
    let wire = hex::decode(cards[0]["card"].as_str().unwrap()).unwrap();
    let (author, request, _) = agentic_protocol::directory::verify_request(
        &wire,
        agentic_node::NETWORK_DOMAIN,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap();
    let agentic_protocol::directory::Request::Card(agentic_protocol::directory::Card::Group {
        group_id,
        name,
        ..
    }) = request
    else {
        panic!("not a group's card");
    };
    let reference =
        agentic_protocol::group::group_ref(&agentic_node::NETWORK_DOMAIN, &author, &group_id);
    assert_eq!(
        hex::encode(reference),
        group(&alice, &g)["result"]["groupRef"].as_str().unwrap()
    );
    let follow = bob.call(
        "follow_group",
        json!({"group": hex::encode(reference), "owner": agentic_protocol::network_id(&author), "name": name}),
    );
    let follow = follow["id"].as_str().unwrap().to_owned();
    alice.send(&g, "welcome", "p-1");
    until(Duration::from_secs(240), "Bob reads the open group", || {
        texts(&bob, &follow, false) == ["welcome"]
    });
    // A lookup is paid, found or not.
    let digest =
        agentic_protocol::directory::handle_digest("google", "nobody@example.org").unwrap();
    let stamps = bob.call(
        "discover_stamps",
        json!({"handles": [{"kind": "google", "digest": hex::encode(digest)}]}),
    );
    let (status, looked) = service.post_paid(
        "/v1/lookup",
        &json!({
            "handles": [{"kind": "google", "digest": hex::encode(digest)}],
            "day": stamps["day"],
            "stamps": stamps["stamps"],
            "grants": stamps["grants"],
        }),
    );
    assert_eq!(status, 200, "{looked}");
    assert!(looked["results"][0]["binding"].is_null(), "{looked}");
}

// --- big groups and channels (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md) ----

impl Network {
    /// Profiles named `names` on this network, each with a bought book, its
    /// directory complete and its card published; how long the cards took.
    fn people<const N: usize>(&self, names: [&str; N]) -> ([Node; N], Duration) {
        let nodes = names.map(|name| {
            let node = self.join();
            node.profile(name);
            node
        });
        for node in &nodes {
            self.fund(node);
        }
        for node in &nodes {
            self.directory(node);
        }
        let published = until(Duration::from_secs(180), "cards published", || {
            nodes
                .iter()
                .all(|node| info(node)["mailboxSwarm"]["intro"]["published"].is_string())
        });
        (nodes, published)
    }
}

fn epoch(node: &Node, g: &str) -> Option<u64> {
    group(node, g)["result"]["epoch"].as_u64()
}

fn members(node: &Node, g: &str) -> Vec<String> {
    group(node, g)["result"]["members"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|m| m.as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A counter of `node`'s public mailboxes: posts, archives, channel keys.
fn public(node: &Node, key: &str) -> u64 {
    info(node)["mailboxSwarm"]["public"][key]
        .as_u64()
        .unwrap_or(0)
}

/// Closed channels' keys `node` took, from its intro mailbox or directly.
fn subscribed(node: &Node) -> u64 {
    let intro = &info(node)["mailboxSwarm"]["intro"];
    ["swarm", "direct"]
        .iter()
        .map(|way| intro[way]["subscribed"].as_u64().unwrap_or(0))
        .sum()
}

/// `by` changes group `g` with `change`; how long until every one of
/// `nodes` is at the next epoch.
fn changed(by: &Node, g: &str, change: Value, op: &str, nodes: &[&Node], what: &str) -> Duration {
    let e = epoch(by, g).unwrap();
    let mut request = change;
    request["groupId"] = json!(g);
    request["operationId"] = json!(op);
    by.call("change_group", request);
    until(Duration::from_secs(180), what, || {
        nodes.iter().all(|node| epoch(node, g) == Some(e + 1))
    })
}

/// `node` knocks at the door of group `reference`, asking again while the
/// door's card is looked up or not published yet; how long it took.
fn knock(node: &Node, reference: &str, note: &str, op: &str) -> (Value, Duration) {
    let started = Instant::now();
    loop {
        let answer = rpc(
            &node.socket(),
            TOKEN,
            "join_group",
            json!({"groupRef": reference, "note": note, "operationId": op}),
        )
        .unwrap();
        let code = answer["error"]["code"].as_str();
        if code != Some("card_pending") && code != Some("card_not_found") {
            return (result(answer), started.elapsed());
        }
        assert!(
            started.elapsed() < Duration::from_secs(180),
            "no door card for {op}: {answer}"
        );
        thread::sleep(Duration::from_secs(1));
    }
}

/// Bans, a group's door and an open group, with real daemons on a local
/// chain (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts 5, 6 and 10a;
/// spec/groups-v1.md, Bans). An admin bans a member, whom nobody adds back
/// until the owner lifts the ban. The group opens: a stranger follows it and
/// reads a member's post by its certificate; the unbanned member knocks at
/// the open door, is let in at once and writes, and the follower takes her
/// post by the certificate the batch gave her. A second group lets in by
/// request: a stranger who lets no stranger in knocks, an admin decides, the
/// batch takes him.
#[test]
#[ignore = "needs anvil, forge and cast; run through the build wrapper"]
fn native_bans_doors_and_open_groups() {
    let net = network("native-bans-doors-and-open-groups");
    let ([alice, bob, carol, dave], published) = net.people(["Alice", "Bob", "Carol", "Dave"]);
    let (alice_id, bob_id, carol_id, dave_id) = (
        network_id(&alice),
        network_id(&bob),
        network_id(&carol),
        network_id(&dave),
    );
    let made = result(once_cards_are_read(
        &alice,
        "create_group",
        json!({"name": "Team", "members": [bob_id, carol_id], "operationId": "g-1"}),
    ));
    let g = made["id"].as_str().unwrap().to_owned();
    let joined = until(Duration::from_secs(180), "Bob and Carol joined", || {
        [&bob, &carol].iter().all(|node| epoch(node, &g).is_some())
    });
    let promoted = changed(
        &alice,
        &g,
        json!({"admins": [bob_id]}),
        "admin",
        &[&alice, &bob, &carol],
        "Bob an admin",
    );
    assert_eq!(group(&bob, &g)["result"]["role"], "admin");

    // Bob, an admin, bans Carol: she is out in the same commit.
    let banned = changed(
        &bob,
        &g,
        json!({"ban": [carol_id]}),
        "ban",
        &[&alice, &bob],
        "the ban decided",
    );
    let out = until(Duration::from_secs(120), "Carol out", || {
        members(&carol, &g).is_empty()
    });
    for node in [&alice, &bob] {
        assert_eq!(
            group(node, &g)["result"]["banned"],
            json!([{"id": carol_id, "byOwner": false}])
        );
        assert!(!members(node, &g).contains(&carol_id));
    }
    let back = once_cards_are_read(
        &alice,
        "change_group",
        json!({"groupId": g, "add": [carol_id], "operationId": "add-back"}),
    );
    assert_eq!(back["error"]["code"], "banned", "{back}");
    alice.send(&g, "without Carol", "m-1");
    let read_after_ban = until(Duration::from_secs(120), "Bob reads on", || {
        texts(&bob, &g, false) == ["without Carol"]
    });
    // The owner lifts it.
    let unbanned = changed(
        &alice,
        &g,
        json!({"unban": [carol_id]}),
        "unban",
        &[&alice, &bob],
        "the ban lifted",
    );
    for node in [&alice, &bob] {
        assert_eq!(group(node, &g)["result"]["banned"], json!([]));
    }
    assert!(texts(&carol, &g, false).is_empty());

    // Open to everyone: Dave, a stranger to it, follows and reads Bob.
    let opened = changed(
        &alice,
        &g,
        json!({"access": "public"}),
        "open",
        &[&alice, &bob],
        "the group opened",
    );
    let reference = group(&alice, &g)["result"]["groupRef"]
        .as_str()
        .unwrap()
        .to_owned();
    let follow = dave.call(
        "follow_group",
        json!({"group": reference, "owner": alice_id, "name": "Team"}),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    bob.send(&g, "for everyone", "p-1");
    let followed = until(Duration::from_secs(420), "Dave reads Bob's post", || {
        texts(&dave, &follow, false) == ["for everyone"]
    });
    // Carol, unbanned, knocks at the open door and is let in at once.
    let (knocked, door_found) = knock(&carol, &reference, "may I come back?", "knock-1");
    assert_eq!(knocked["groupId"], g.as_str());
    let let_in = until(Duration::from_secs(300), "Carol let in", || {
        [&alice, &bob, &carol]
            .iter()
            .all(|node| members(node, &g).contains(&carol_id))
    });
    carol.send(&g, "I am here again", "p-2");
    let newcomer_read = until(Duration::from_secs(420), "Dave reads Carol's post", || {
        texts(&dave, &follow, false) == ["for everyone", "I am here again"]
    });

    // A group by request: Dave lets no stranger in by himself and knocks;
    // Bob, an admin, decides.
    let made = result(once_cards_are_read(
        &alice,
        "create_group",
        json!({"name": "Club", "members": [bob_id], "operationId": "h-1"}),
    ));
    let h = made["id"].as_str().unwrap().to_owned();
    until(Duration::from_secs(180), "Bob in the club", || {
        epoch(&bob, &h).is_some()
    });
    changed(
        &alice,
        &h,
        json!({"admins": [bob_id]}),
        "h-admin",
        &[&alice, &bob],
        "Bob the club's admin",
    );
    changed(
        &alice,
        &h,
        json!({"access": "request"}),
        "h-door",
        &[&alice, &bob],
        "the club by request",
    );
    assert_eq!(group(&bob, &h)["result"]["access"], "request");
    dave.call(
        "set_intro_policy",
        json!({"mode": "manual", "dailyLimit": 20, "allowed": []}),
    );
    let h_ref = group(&alice, &h)["result"]["groupRef"]
        .as_str()
        .unwrap()
        .to_owned();
    let (knocked, club_door_found) = knock(&dave, &h_ref, "and to the club", "knock-2");
    assert_eq!(knocked["groupId"], h.as_str());
    let listed = until(Duration::from_secs(180), "the request at Bob's", || {
        bob.call("door_requests", json!({"groupId": h}))
            .as_array()
            .is_some_and(|list| !list.is_empty())
    });
    let requests = bob.call("door_requests", json!({"groupId": h}));
    let request = &requests[0];
    assert_eq!(
        (request["networkId"].as_str(), request["note"].as_str()),
        (Some(dave_id.as_str()), Some("and to the club")),
        "{requests}"
    );
    assert_eq!(epoch(&dave, &h), None);
    bob.call(
        "door_decide",
        json!({"groupId": h, "requestId": request["requestId"], "accept": true}),
    );
    let admitted = until(Duration::from_secs(240), "Dave in the club", || {
        [&alice, &bob, &dave]
            .iter()
            .all(|node| members(node, &h).contains(&dave_id))
    });
    dave.send(&h, "thanks for letting me in", "h-m1");
    let club_read = until(Duration::from_secs(120), "the club reads Dave", || {
        [&alice, &bob]
            .iter()
            .all(|node| texts(node, &h, false) == ["thanks for letting me in"])
    });
    // Carol never read what was written while she was out.
    assert!(!texts(&carol, &g, false).contains(&"without Carol".to_owned()));
    let report = json!({
        "holders": HOLDERS,
        "seconds": {
            "directoryComplete": net.directory.as_secs_f64(),
            "cardsPublished": published.as_secs_f64(),
            "groupJoined": joined.as_secs_f64(),
            "adminMade": promoted.as_secs_f64(),
            "banDecided": banned.as_secs_f64(),
            "bannedOut": out.as_secs_f64(),
            "readAfterBan": read_after_ban.as_secs_f64(),
            "banLifted": unbanned.as_secs_f64(),
            "groupOpened": opened.as_secs_f64(),
            "followerReadMember": followed.as_secs_f64(),
            "openDoorCardFound": door_found.as_secs_f64(),
            "letInAtOpenDoor": let_in.as_secs_f64(),
            "followerReadNewcomer": newcomer_read.as_secs_f64(),
            "requestDoorCardFound": club_door_found.as_secs_f64(),
            "requestListedAtAdmin": listed.as_secs_f64(),
            "admittedAfterDecision": admitted.as_secs_f64(),
            "newcomerReadByGroup": club_read.as_secs_f64(),
            "sinceHoldersStarted": net.started.elapsed().as_secs_f64(),
        },
        "door": {
            "alice": info(&alice)["mailboxSwarm"]["door"],
            "bob": info(&bob)["mailboxSwarm"]["door"],
        },
        "follower": info(&dave)["mailboxSwarm"]["public"],
        "failureKinds": ([&alice, &bob, &carol, &dave].map(|node| info(node)["mailboxSwarm"]["failureKinds"].clone())),
    });
    println!("native-bans-doors-and-open-groups {report}");
}

/// A post's text of about 23 KB: three of them fill an archive part.
fn long_post(word: &str) -> String {
    word.repeat(11_600 / word.chars().count())
}

/// A public channel with real daemons on a local chain
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, parts 8–10b): its owner and
/// an admin post, a follower reads new posts on the channel's pace, the
/// team lays the full archive part, and a later follower reads the history
/// and the part.
#[test]
#[ignore = "needs anvil, forge and cast; run through the build wrapper"]
fn native_public_channel() {
    let net = network("native-public-channel");
    let ([alice, bob, dave, erin], published) = net.people(["Alice", "Bob", "Dave", "Erin"]);
    let (alice_id, bob_id) = (network_id(&alice), network_id(&bob));
    let made = result(once_cards_are_read(
        &alice,
        "create_group",
        json!({"name": "News", "members": [bob_id], "kind": "channel", "access": "public", "operationId": "c-1"}),
    ));
    assert_eq!(
        (&made["kind"], &made["access"]),
        (&json!("channel"), &json!("public"))
    );
    let c = made["id"].as_str().unwrap().to_owned();
    let reference = made["groupRef"].as_str().unwrap().to_owned();
    let team = until(Duration::from_secs(180), "Bob in the team", || {
        epoch(&bob, &c).is_some()
    });
    assert_eq!(group(&bob, &c)["result"]["role"], "admin");
    let kept = changed(
        &alice,
        &c,
        json!({"retention": 90}),
        "keep",
        &[&alice, &bob],
        "kept for 90 days",
    );
    assert_eq!(group(&alice, &c)["result"]["retention"], 90);
    // Dave follows first; the posts come after.
    let dave_follow = dave.call(
        "follow_group",
        json!({"group": reference, "owner": alice_id, "name": "News"}),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let posts = [
        long_post("post"),
        long_post("news item"),
        long_post("thought"),
    ];
    alice.send(&c, &posts[0], "p-1");
    bob.send(&c, &posts[1], "p-2");
    alice.send(&c, &posts[2], "p-3");
    let posted = Instant::now();
    let followed = until(Duration::from_secs(720), "Dave reads the posts", || {
        texts(&dave, &dave_follow, false).len() == 3
    });
    let mut read = texts(&dave, &dave_follow, false);
    read.sort();
    let mut expected = posts.to_vec();
    expected.sort();
    assert_eq!(read, expected);
    // The team lays the part the posts filled.
    let laid = until(Duration::from_secs(720), "an archive part laid", || {
        public(&alice, "archivesLaid") + public(&bob, "archivesLaid") >= 1
    });
    let since_posted = posted.elapsed();
    let storage = alice.call("channel_storage", json!({"groupId": c}));
    let parts = storage["parts"].as_u64().unwrap();
    assert_eq!(storage["retention"], 90, "{storage}");
    assert!(parts >= 1, "{storage}");
    assert_eq!(
        storage["stampsPerMonth"].as_u64(),
        Some((parts * 30).div_ceil(25)),
        "{storage}"
    );
    // Erin, following later, reads the history and the part.
    let erin_follow = erin.call(
        "follow_group",
        json!({"group": reference, "owner": alice_id, "name": "News"}),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let history = until(
        Duration::from_secs(720),
        "Erin reads the history and the archive",
        || texts(&erin, &erin_follow, false).len() == 3 && public(&erin, "archives") >= 1,
    );
    let follows = erin.call("follows", json!({}));
    assert_eq!(
        (&follows[0]["kind"], &follows[0]["retention"]),
        (&json!("channel"), &json!(90)),
        "{follows}"
    );
    let report = json!({
        "holders": HOLDERS,
        "postBytes": posts.iter().map(String::len).collect::<Vec<_>>(),
        "seconds": {
            "directoryComplete": net.directory.as_secs_f64(),
            "cardsPublished": published.as_secs_f64(),
            "teamJoined": team.as_secs_f64(),
            "retentionSet": kept.as_secs_f64(),
            "followerReadPosts": followed.as_secs_f64(),
            "archivePartLaid": since_posted.as_secs_f64(),
            "archiveWait": laid.as_secs_f64(),
            "laterFollowerReadHistoryAndArchive": history.as_secs_f64(),
            "sinceHoldersStarted": net.started.elapsed().as_secs_f64(),
        },
        "storage": storage,
        "team": ([&alice, &bob].map(|node| info(node)["mailboxSwarm"]["public"].clone())),
        "followers": ([&dave, &erin].map(|node| info(node)["mailboxSwarm"]["public"].clone())),
        "failureKinds": ([&alice, &bob, &dave, &erin].map(|node| info(node)["mailboxSwarm"]["failureKinds"].clone())),
    });
    println!("native-public-channel {report}");
}

/// A closed channel with real daemons on a local chain
/// (Docs/V1_LARGE_GROUPS_CHANNELS_2026_09_28.md, part 10c): the owner gives
/// keys; the subscribers who took them publish keys of their own; one who
/// leaves keys waiting for her decision publishes none. An admin takes a
/// subscriber off and the channel moves to new keys. Then the owner takes
/// the admin off the team: the owner's node reseeds onto the subscribers'
/// own keys by itself and gives the waiting subscriber keys again. The
/// subscribers read on; the former admin and the removed subscriber do not.
#[test]
#[ignore = "needs anvil, forge and cast; run through the build wrapper"]
fn native_closed_channel() {
    let net = network("native-closed-channel");
    let ([alice, bob, carol, dave, erin], published) =
        net.people(["Alice", "Bob", "Carol", "Dave", "Erin"]);
    let (bob_id, carol_id, dave_id, erin_id) = (
        network_id(&bob),
        network_id(&carol),
        network_id(&dave),
        network_id(&erin),
    );
    carol.call(
        "set_intro_policy",
        json!({"mode": "manual", "dailyLimit": 20, "allowed": []}),
    );
    let made = result(once_cards_are_read(
        &alice,
        "create_group",
        json!({"name": "Club", "members": [bob_id], "kind": "channel", "access": "private", "operationId": "c-1"}),
    ));
    assert_eq!(made["access"], "private");
    let c = made["id"].as_str().unwrap().to_owned();
    let reference = made["groupRef"].as_str().unwrap().to_owned();
    let team = until(Duration::from_secs(180), "Bob in the team", || {
        epoch(&bob, &c).is_some()
    });
    assert_eq!(group(&bob, &c)["result"]["role"], "admin");
    // Keys for three: Dave and Erin take them and publish keys of their own;
    // Carol's wait for her.
    let given = once_cards_are_read(
        &alice,
        "channel_subscribe",
        json!({"groupId": c, "members": [dave_id, carol_id, erin_id], "operationId": "s-1"}),
    );
    assert!(given.get("result").is_some(), "{given}");
    let keys_given = until(Duration::from_secs(240), "keys taken and published", || {
        subscribed(&dave) == 1
            && subscribed(&erin) == 1
            && public(&alice, "subscriberKeys") >= 2
            && carol
                .call("intro_requests", json!({}))
                .as_array()
                .is_some_and(|list| !list.is_empty())
    });
    assert_eq!(public(&alice, "subscriberKeys"), 2);
    assert_eq!(subscribed(&carol), 0);
    alice.send(&c, "members only", "m-1");
    let first_read = until(Duration::from_secs(720), "the first post read", || {
        [&dave, &erin]
            .iter()
            .all(|node| texts(node, &reference, false) == ["members only"])
            && texts(&bob, &c, false) == ["members only"]
    });

    // Bob, an admin, takes Erin off: his node publishes the key update.
    let taken_off = changed(
        &bob,
        &c,
        json!({"unsubscribe": [erin_id]}),
        "u-1",
        &[&alice, &bob],
        "Erin taken off",
    );
    let rekeyed = until(Duration::from_secs(720), "Dave on the new key", || {
        public(&bob, "keyUpdates") >= 1 && public(&dave, "rekeyed") >= 1
    });
    alice.send(&c, "already without Erin", "m-2");
    let second_read = until(Duration::from_secs(720), "Dave reads on", || {
        texts(&dave, &reference, false).len() == 2
    });
    assert_eq!(public(&erin, "rekeyed"), 0);

    // Alice takes Bob off the team: her node reseeds onto the subscribers'
    // own keys by itself, and gives Carol, whose key it never read, keys
    // again.
    let dave_rekeyed = public(&dave, "rekeyed");
    let removed = changed(
        &alice,
        &c,
        json!({"remove": [bob_id]}),
        "r-1",
        &[&alice],
        "Bob off the team",
    );
    let reseeded = until(Duration::from_secs(720), "the hard reseed", || {
        public(&alice, "hardReseeds") >= 1
            && public(&dave, "rekeyed") > dave_rekeyed
            && carol
                .call("intro_requests", json!({}))
                .as_array()
                .is_some_and(|list| list.len() >= 2)
    });
    assert_eq!(public(&alice, "hardReseeds"), 1);
    assert_eq!(public(&bob, "hardReseeds"), 0);
    for request in carol
        .call("intro_requests", json!({}))
        .as_array()
        .unwrap()
        .clone()
    {
        carol.call(
            "accept_intro_request",
            json!({"requestId": request["requestId"]}),
        );
    }
    alice.send(&c, "already without Bob", "m-3");
    let third_read = until(Duration::from_secs(720), "the subscribers read on", || {
        texts(&dave, &reference, false).len() == 3
            && texts(&carol, &reference, false).contains(&"already without Bob".to_owned())
    });
    assert_eq!(
        texts(&dave, &reference, false),
        [
            "members only",
            "already without Erin",
            "already without Bob"
        ]
    );
    // Neither the former admin nor the removed subscriber reads on.
    thread::sleep(Duration::from_secs(30));
    assert!(!texts(&bob, &c, false).contains(&"already without Bob".to_owned()));
    assert_eq!(texts(&erin, &reference, false), ["members only"]);
    let report = json!({
        "holders": HOLDERS,
        "seconds": {
            "directoryComplete": net.directory.as_secs_f64(),
            "cardsPublished": published.as_secs_f64(),
            "teamJoined": team.as_secs_f64(),
            "keysTakenAndPublished": keys_given.as_secs_f64(),
            "firstPostRead": first_read.as_secs_f64(),
            "subscriberTakenOff": taken_off.as_secs_f64(),
            "rekeyedAfterTakenOff": rekeyed.as_secs_f64(),
            "readAfterTakenOff": second_read.as_secs_f64(),
            "adminRemoved": removed.as_secs_f64(),
            "hardReseedAndKeysAgain": reseeded.as_secs_f64(),
            "readAfterAdminRemoved": third_read.as_secs_f64(),
            "sinceHoldersStarted": net.started.elapsed().as_secs_f64(),
        },
        "owner": info(&alice)["mailboxSwarm"]["public"],
        "removedAdmin": info(&bob)["mailboxSwarm"]["public"],
        "subscribers": ([&carol, &dave, &erin].map(|node| info(node)["mailboxSwarm"]["public"].clone())),
        "failureKinds": ([&alice, &bob, &carol, &dave, &erin].map(|node| info(node)["mailboxSwarm"]["failureKinds"].clone())),
    });
    println!("native-closed-channel {report}");
}

/// `to`'s invitation, added by `from`: a contact made without cards.
fn invite(from: &Node, to: &Node, name: &str) -> String {
    let invitation = to.call("create_invitation", json!({"addresses": null}));
    let contact = from.call(
        "add_contact",
        json!({"name": name, "invitation": invitation}),
    );
    let id = contact["id"].as_str().unwrap().to_owned();
    until(Duration::from_secs(60), "the contact joined", || {
        to.snapshot()["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == id.as_str())
    });
    id
}

/// Whether `node` shows the message `text` it received with low trust.
fn low_trust(node: &Node, conversation: &str, text: &str) -> bool {
    node.snapshot()["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == conversation)
        .and_then(|c| {
            c["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["own"] == false && m["text"] == text)
                .map(|m| m["lowTrust"] == true)
        })
        .unwrap_or(false)
}

/// The delivery phase of `node`'s own message `text`.
fn phase(node: &Node, conversation: &str, text: &str) -> String {
    node.snapshot()["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == conversation)
        .and_then(|c| {
            c["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["own"] == true && m["text"] == text)
                .map(|m| {
                    m["delivery"]["phase"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned()
                })
        })
        .unwrap_or_default()
}

/// The peers `node` verified by their signed records and is connected to.
fn verified(node: &Node) -> Vec<String> {
    info(node)["bootstrap"]["verifiedPeers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["peerId"].as_str().unwrap().to_owned())
        .collect()
}

/// `flags` with another chain RPC and these bootstrap routes.
fn with_routes(flags: &[String], rpc: &str, bootstrap: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut given = flags.iter();
    while let Some(flag) = given.next() {
        match flag.as_str() {
            "--chain-rpc" => {
                given.next();
                out.extend(["--chain-rpc".to_owned(), rpc.to_owned()]);
            }
            "--bootstrap" => {
                given.next();
            }
            _ => out.push(flag.clone()),
        }
    }
    for route in bootstrap {
        out.extend(["--bootstrap".to_owned(), route.clone()]);
    }
    out
}

/// Kaiki without the Internet (README, "How it compares"): contacts on one
/// LAN, all online, while the chain RPC hangs, the holders are gone and
/// every bootstrap route swallows its packets. The daemons start at once;
/// the owners turn on local discovery as the settings panel does; the nodes
/// find each other over mDNS and messages go both ways directly, paid with
/// books their recipients read while online. A book the recipient never
/// read cannot be checked offline: that message stays queued until the
/// recipient reads the chain again.
#[test]
#[ignore = "needs anvil, forge and cast; run through the build wrapper"]
fn native_lan_without_internet() {
    let mut net = network("native-lan-without-internet");
    let (mut alice, mut bob, mut carol) = (net.join(), net.join(), net.join());
    for (name, node) in [("Alice", &alice), ("Bob", &bob), ("Carol", &carol)] {
        node.profile(name);
        net.fund(node);
    }
    for node in [&alice, &bob, &carol] {
        net.directory(node);
    }
    let ab = invite(&alice, &bob, "Bob");
    let cb = invite(&carol, &bob, "Bob");
    // Online, Alice and Bob write each other and Bob writes Carol; Carol
    // never writes Bob, so Bob never reads her book.
    alice.send(&ab, "online from Alice", "a-1");
    bob.send(&ab, "online from Bob", "b-1");
    bob.send(&cb, "online from Bob to Carol", "b-2");
    until(Duration::from_secs(120), "the online messages read", || {
        texts(&bob, &ab, false) == ["online from Alice"]
            && texts(&alice, &ab, false) == ["online from Bob"]
            && texts(&carol, &cb, false) == ["online from Bob to Carol"]
    });
    until(
        Duration::from_secs(120),
        "the books read, outboxes empty",
        || {
            [&alice, &bob, &carol].iter().all(|node| {
                let info = info(node);
                info["pendingOutbox"] == 0 && info["chain"]["learned"].as_u64() >= Some(1)
            })
        },
    );
    let online_chain = [&alice, &bob, &carol].map(|node| info(node)["chain"].clone());

    // The Internet goes: the holders stop, the chain RPC takes connections
    // and never answers, and the bootstrap routes are UDP ports that drop
    // every packet, as a router without an uplink does.
    let hanging_rpc = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let rpc_url = format!("http://{}", hanging_rpc.local_addr().unwrap());
    let silent: Vec<std::net::UdpSocket> = (0..4)
        .map(|_| std::net::UdpSocket::bind("127.0.0.1:0").unwrap())
        .collect();
    let bootstrap: Vec<String> = silent
        .iter()
        .zip(&net.holders)
        .map(|(socket, holder)| {
            format!(
                "/ip4/127.0.0.1/udp/{}/quic-v1/p2p/{}",
                socket.local_addr().unwrap().port(),
                holder.peer
            )
        })
        .collect();
    for holder in &mut net.holders {
        holder.kill();
    }
    let offline = with_routes(&net.flags, &rpc_url, &bootstrap);
    let mut starts = vec![];
    for node in [&mut alice, &mut bob, &mut carol] {
        node.kill();
        node.arguments = offline.clone();
        // The owner's daemon listens on every interface.
        node.listeners = vec![
            "/ip4/0.0.0.0/udp/0/quic-v1".to_owned(),
            "/ip4/0.0.0.0/tcp/0".to_owned(),
        ];
        let begun = Instant::now();
        node.launch();
        starts.push(begun.elapsed().as_secs_f64());
    }
    // Local discovery is off by default; each owner turns it on in the
    // settings, which send every preference back with the change.
    let turned_on = Instant::now();
    for node in [&alice, &bob, &carol] {
        let settings = node.call("network_settings", json!({}));
        let mut preferences = settings["preferences"].clone();
        assert_eq!(preferences["lanDiscovery"], false, "{settings}");
        preferences["lanDiscovery"] = json!(true);
        let saved = node.call(
            "configure_network",
            json!({"expectedRevision": settings["revision"], "preferences": preferences}),
        );
        assert_eq!(saved["status"]["lanDiscovery"]["active"], true, "{saved}");
    }
    let found = || {
        let (a, b, c) = (verified(&alice), verified(&bob), verified(&carol));
        a.contains(&bob.peer)
            && b.contains(&alice.peer)
            && b.contains(&carol.peer)
            && c.contains(&bob.peer)
    };
    while !found() && turned_on.elapsed() < Duration::from_secs(120) {
        thread::sleep(Duration::from_millis(250));
    }
    let discovered = turned_on.elapsed();
    assert!(
        found(),
        "LAN peers verified within 120 s: {:?}",
        [&alice, &bob, &carol].map(|node| {
            let info = info(node);
            json!({"lan": info["lanDiscovery"], "bootstrap": info["bootstrap"],
                "connections": info["peerConnections"]})
        })
    );
    let lan_info = [&alice, &bob, &carol].map(|node| {
        let info = info(node);
        json!({"lanDiscovery": info["lanDiscovery"], "bootstrap": info["bootstrap"],
            "advertisedAddresses": info["advertisedAddresses"], "transportsUsed": info["transportsUsed"]})
    });

    // Both online on the LAN: both ways, directly.
    alice.send(&ab, "offline from Alice", "a-2");
    bob.send(&ab, "offline from Bob", "b-3");
    let exchanged = until(Duration::from_secs(120), "read both ways offline", || {
        texts(&bob, &ab, false).last().map(String::as_str) == Some("offline from Alice")
            && texts(&alice, &ab, false).last().map(String::as_str) == Some("offline from Bob")
    });
    let receipted = until(Duration::from_secs(60), "both delivered", || {
        phase(&alice, &ab, "offline from Alice") == "delivered"
            && phase(&bob, &ab, "offline from Bob") == "delivered"
    });
    // Past the idle timeout of a connection nothing uses.
    thread::sleep(Duration::from_secs(75));
    alice.send(&ab, "after a pause from Alice", "a-3");
    bob.send(&ab, "after a pause from Bob", "b-4");
    let after_pause = until(
        Duration::from_secs(120),
        "read both ways after a pause",
        || {
            texts(&bob, &ab, false).last().map(String::as_str) == Some("after a pause from Alice")
                && texts(&alice, &ab, false).last().map(String::as_str)
                    == Some("after a pause from Bob")
        },
    );

    // Bob never read Carol's book and cannot read it now: once his read of
    // it fails, her message is taken with low trust; his goes.
    let rejected_before = info(&bob)["rejectedFrames"].as_u64().unwrap();
    let sent_at = Instant::now();
    carol.send(&cb, "offline from Carol", "c-1");
    bob.send(&cb, "offline from Bob to Carol", "b-5");
    let carol_read = until(Duration::from_secs(120), "Carol reads Bob offline", || {
        texts(&carol, &cb, false).last().map(String::as_str) == Some("offline from Bob to Carol")
    });
    until(
        Duration::from_secs(120),
        "Bob takes Carol's message with low trust",
        || texts(&bob, &cb, false) == ["offline from Carol"],
    );
    let low_trust_taken = sent_at.elapsed();
    assert!(low_trust(&bob, &cb, "offline from Carol"));
    let delivered_low = until(Duration::from_secs(60), "Carol's message delivered", || {
        phase(&carol, &cb, "offline from Carol") == "delivered"
    });
    let waiting = json!({
        "carolPhase": phase(&carol, &cb, "offline from Carol"),
        "carolPendingOutbox": info(&carol)["pendingOutbox"],
        "carolFailureKinds": info(&carol)["mailboxSwarm"]["failureKinds"],
        "bobRejectedFrames": info(&bob)["rejectedFrames"].as_u64().unwrap() - rejected_before,
        "bobChain": info(&bob)["chain"],
    });

    // The Internet comes back for Bob alone (the holders stay gone): he
    // reads her book, and her next message is checked, without the mark.
    bob.kill();
    bob.arguments = with_routes(&net.flags, &net.chain.url, &bootstrap);
    bob.launch();
    carol.send(&cb, "back from Carol", "c-2");
    let recovered = until(
        Duration::from_secs(180),
        "Bob reads Carol with the chain",
        || texts(&bob, &cb, false) == ["offline from Carol", "back from Carol"],
    );
    assert!(!low_trust(&bob, &cb, "back from Carol"));
    assert!(low_trust(&bob, &cb, "offline from Carol"));
    let report = json!({
        "holders": HOLDERS,
        "seconds": {
            "directoryComplete": net.directory.as_secs_f64(),
            "offlineStarts": starts,
            "lanPeersVerifiedAfterTurnedOn": discovered.as_secs_f64(),
            "sinceTurnedOn": turned_on.elapsed().as_secs_f64(),
            "offlineReadBothWays": exchanged.as_secs_f64(),
            "offlineDeliveredBothWays": receipted.as_secs_f64(),
            "readBothWaysAfterPause": after_pause.as_secs_f64(),
            "bobToCarolOffline": carol_read.as_secs_f64(),
            "carolTakenWithLowTrust": low_trust_taken.as_secs_f64(),
            "carolLowTrustDeliveredAfter": delivered_low.as_secs_f64(),
            "carolDeliveredOnceBobReadsChain": recovered.as_secs_f64(),
        },
        "onlineChain": online_chain,
        "lan": lan_info,
        "unknownBookWhileOffline": waiting,
        "failureKinds": ([&alice, &bob, &carol].map(|node| info(node)["mailboxSwarm"]["failureKinds"].clone())),
    });
    drop((hanging_rpc, silent));
    println!("native-lan-without-internet {report}");
}

/// How long local discovery takes to verify every pair of daemons on this
/// host's LAN, round after round, with `AIN_LAN_SILENT` (default 4)
/// bootstrap routes that drop every packet. No chain: discovery only.
#[test]
#[ignore = "manual: multicast DNS on this host's LAN"]
fn lan_discovery_rounds() {
    let rounds: usize = std::env::var("AIN_LAN_ROUNDS").map_or(5, |v| v.parse().unwrap());
    let count: usize = std::env::var("AIN_LAN_SILENT").map_or(4, |v| v.parse().unwrap());
    let silent: Vec<std::net::UdpSocket> = (0..count)
        .map(|_| std::net::UdpSocket::bind("127.0.0.1:0").unwrap())
        .collect();
    let mut flags = vec![];
    for socket in &silent {
        let peer = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        flags.extend([
            "--bootstrap".to_owned(),
            format!(
                "/ip4/127.0.0.1/udp/{}/quic-v1/p2p/{peer}",
                socket.local_addr().unwrap().port()
            ),
        ]);
    }
    let wildcard = ["/ip4/0.0.0.0/udp/0/quic-v1", "/ip4/0.0.0.0/tcp/0"];
    // With `AIN_LAN_DEAD` peers the profiles met online and that are gone
    // now, as the holders are when the Internet goes.
    let dead: usize = std::env::var("AIN_LAN_DEAD").map_or(0, |v| v.parse().unwrap());
    let mut kept: Vec<Node> = vec![];
    if dead > 0 {
        let tcp = ["/ip4/127.0.0.1/tcp/0"];
        let mut peers = vec![Node::start(&tcp)];
        let hub = route(&peers[0]);
        for _ in 1..dead {
            peers.push(Node::start_with_args(
                &tcp,
                vec!["--bootstrap".to_owned(), hub.clone()],
            ));
        }
        kept = (0..3)
            .map(|_| Node::start_with_args(&tcp, vec!["--bootstrap".to_owned(), hub.clone()]))
            .collect();
        let met = until(Duration::from_secs(60), "records cached", || {
            kept.iter()
                .all(|node| info(node)["bootstrap"]["cachedHints"].as_u64() >= Some(1))
        });
        thread::sleep(Duration::from_secs(20));
        println!(
            "met in {met:?}: cached {:?}",
            kept.iter()
                .map(|node| info(node)["bootstrap"]["cachedHints"].clone())
                .collect::<Vec<_>>()
        );
        drop(peers);
    }
    let mut times = vec![];
    for round in 0..rounds {
        let nodes: Vec<Node> = if dead > 0 {
            for node in &mut kept {
                node.kill();
                node.arguments = flags.clone();
                node.listeners = wildcard.map(String::from).to_vec();
                node.launch();
            }
            std::mem::take(&mut kept)
        } else {
            (0..3)
                .map(|_| Node::start_with_args(&wildcard, flags.clone()))
                .collect()
        };
        for node in &nodes {
            let settings = node.call("network_settings", json!({}));
            let mut preferences = settings["preferences"].clone();
            preferences["lanDiscovery"] = json!(true);
            node.call(
                "configure_network",
                json!({"expectedRevision": settings["revision"], "preferences": preferences}),
            );
        }
        let begun = Instant::now();
        let all = || {
            nodes.iter().all(|node| {
                let own = verified(node);
                nodes
                    .iter()
                    .all(|other| other.peer == node.peer || own.contains(&other.peer))
            })
        };
        while !all() && begun.elapsed() < Duration::from_secs(120) {
            thread::sleep(Duration::from_millis(250));
        }
        let done = all();
        times.push(if done {
            begun.elapsed().as_secs_f64()
        } else {
            -1.0
        });
        println!("round {round}: verified={done} after {:?}", begun.elapsed());
        if !done {
            for node in &nodes {
                let info = info(node);
                println!(
                    "  {} lan={} bootstrap={} conns={}",
                    node.peer, info["lanDiscovery"], info["bootstrap"], info["peerConnections"]
                );
                println!(
                    "  stderr: {}",
                    fs::read_to_string(node.root.path().join("stderr.log"))
                        .unwrap_or_default()
                        .lines()
                        .rev()
                        .take(20)
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            }
        }
        if dead > 0 {
            kept = nodes;
        }
    }
    println!(
        "lan-discovery-rounds silent={count} dead={dead} {}",
        json!(times)
    );
    assert!(
        times.iter().all(|t| (0.0..=30.0).contains(t)),
        "every round verified within 30 s"
    );
}
