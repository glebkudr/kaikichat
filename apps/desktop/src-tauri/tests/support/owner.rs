//! The owner's screens over the daemon (V1-GF01): contacts by id, groups,
//! the wallet and the skill, each a window command that proxies one daemon
//! method; links open only as the daemon gave them.
use super::*;
use std::io::{BufRead, BufReader, Read, Write};

/// Every command the owner's screens add, with a request that would change
/// something if it were let through.
fn owner_commands(someone: &str) -> Vec<(&'static str, Value)> {
    vec![
        (
            "request_contact",
            json!({"request":{"networkId":someone,"name":"X","operationId":"w-1"}}),
        ),
        ("intro_requests", json!({})),
        (
            "accept_intro_request",
            json!({"request":{"requestId":"ab".repeat(32)}}),
        ),
        (
            "reject_intro_request",
            json!({"request":{"requestId":"ab".repeat(32)}}),
        ),
        ("intro_policy", json!({})),
        (
            "set_intro_policy",
            json!({"request":{"mode":"manual","dailyLimit":1,"allowed":[]}}),
        ),
        ("groups", json!({})),
        ("group", json!({"request":{"groupId":"ab".repeat(32)}})),
        (
            "create_group",
            json!({"request":{"name":"Intruders","members":[],"operationId":"w-2"}}),
        ),
        (
            "change_group",
            json!({"request":{"groupId":"ab".repeat(32),"admins":[],"operationId":"w-3"}}),
        ),
        ("coins_balance", json!({})),
        ("coins_buy", json!({})),
        ("claim_coins", json!({"request":{"provider":"google"}})),
        (
            "open_payment",
            json!({"request":{"book":"0x00","step":"eth"}}),
        ),
        (
            "install_skill",
            json!({"request":{"skill":"kaiki","host":"claude"}}),
        ),
        ("owner_cli", json!({})),
        (
            "discover_handles",
            json!({"request":{"text":"ann@example.org"}}),
        ),
        (
            "discover_lookup",
            json!({"request":{"handles":[{"kind":"google","handle":"ann@example.org"}]}}),
        ),
        ("discover_link", json!({"request":{"kind":"google"}})),
        ("discover_status", json!({"request":{"linkId":"l1"}})),
        ("discover_unlink", json!({"request":{"kind":"google"}})),
        (
            "discover_publish",
            json!({"request":{"kind":"profile","about":"x","tags":[],"langs":[]}}),
        ),
        (
            "discover_withdraw",
            json!({"request":{"cardId":"ab".repeat(32)}}),
        ),
        ("discover_search", json!({"request":{"query":"rust"}})),
        ("follows", json!({})),
        (
            "follow_group",
            json!({"request":{"group":"ab".repeat(32),"owner":someone,"name":"X"}}),
        ),
        (
            "unfollow_group",
            json!({"request":{"groupId":"ab".repeat(32)}}),
        ),
        ("profile_status", json!({})),
        ("unlock_profile", json!({"request":{"password":"guess"}})),
        ("reconnect", json!({})),
        ("network_preset", json!({})),
        ("refresh_network", json!({"request":{"switch":true}})),
    ]
}

#[tokio::test]
async fn owner_screens_commands_are_refused_to_other_windows_and_origins() {
    let f = Fixture::new().await;
    f.identity("Alice");
    let someone = format!("ain1{}", "ab".repeat(32));
    let other = tauri::WebviewWindowBuilder::new(
        &f.app,
        "untrusted",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .build()
    .unwrap();
    let policy = f.call("intro_policy", json!({})).unwrap();
    for (method, request) in owner_commands(&someone) {
        for (window, origin) in [
            (&other, "tauri://localhost"),
            (&f.window, "https://untrusted.example"),
        ] {
            let denied = invoke(window, origin, method, request.clone())
                .expect_err(&format!("{method} from {origin} on {}", window.label()));
            let denied = denied.as_str().unwrap_or_default();
            assert!(
                denied.starts_with(&format!("{method} not allowed on window "))
                    || denied == format!("Command {method} not allowed by ACL"),
                "{method} from {origin}: {denied}"
            );
        }
    }
    // Nothing the refused calls asked for happened.
    assert_eq!(f.call("intro_policy", json!({})).unwrap(), policy);
    assert_eq!(f.call("groups", json!({})).unwrap(), json!([]));
    assert!(!f.home.join(".claude").exists());
    assert_eq!(f.opened(), Vec::<String>::new());
    // The main window gets the daemon's refusals as codes, the old commands
    // included.
    let refused = f.refusal(
        "send_message",
        json!({"request":{"conversationId":"cd".repeat(32),"text":"hi","operationId":"m-1"}}),
    );
    assert_eq!(refused["retryable"], false, "{refused}");
    // A daemon without a discovery service says so to the discovery screen.
    let refused = f.refusal("discover_search", json!({"request":{"query":"rust"}}));
    assert_eq!(
        (refused["code"].as_str(), refused["retryable"].as_bool()),
        (Some("directory_not_configured"), Some(false)),
        "{refused}"
    );
}

/// A stand-in discovery service: its policy naming `key`, a login link to
/// `login`, and that link linked.
fn directory(key: &str, login: &str) -> String {
    let domain: String = agentic_desktop_host::NETWORK_DOMAIN
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let (key, login) = (key.to_owned(), login.to_owned());
    stub(move |line, _| {
        if line.starts_with("GET /v1/policy ") {
            json!({"domain": domain, "key": key, "lookupPrice": 1, "cardPrice": 10, "cardDays": 30})
        } else if line.starts_with("POST /v1/links ") {
            json!({"linkId": "l1", "loginUrl": login, "code": "4F7K-9QX2", "expiresAt": 4_000_000_000u64})
        } else {
            json!({"status": "linked", "reason": null})
        }
    })
}

/// The discovery screen through the daemon's service: addresses read from
/// pasted text, a login link opened in the browser with the code its page
/// shows, the link's state, and an open group read without joining it. A
/// service that sends the owner over plain http to another host gets
/// nothing opened.
#[tokio::test]
async fn the_discovery_screen_opens_a_login_link_reads_addresses_and_follows_a_group() {
    let key = "33".repeat(32);
    let login = "https://directory.example/v1/links/l1/login";
    let f = Fixture::with_args(vec![
        "--directory".into(),
        directory(&key, login),
        "--directory-key".into(),
        key.clone(),
    ])
    .await;
    f.identity("Alice");
    let read = f
        .call(
            "discover_handles",
            json!({"request":{"text":"Ann Lee <Ann.Lee@Gmail.com>\ngithub:Octo-Cat\nnot an address"}}),
        )
        .unwrap();
    assert_eq!(
        read,
        json!({"handles":[{"kind":"google","handle":"annlee@gmail.com"},{"kind":"github","handle":"octo-cat"}]})
    );
    let opened = f
        .call("discover_link", json!({"request":{"kind":"google"}}))
        .unwrap();
    assert_eq!(
        (opened["linkId"].as_str(), opened["code"].as_str()),
        (Some("l1"), Some("4F7K-9QX2")),
        "{opened}"
    );
    assert_eq!(f.opened(), [login]);
    let status = f
        .call("discover_status", json!({"request":{"linkId":"l1"}}))
        .unwrap();
    assert_eq!(status["status"], "linked", "{status}");

    // A card's group, read without joining: listed with the chats, then
    // left.
    let owner = format!("ain1{}", "cd".repeat(32));
    let follow = f
        .call(
            "follow_group",
            json!({"request":{"group":"ef".repeat(32),"owner":owner,"name":"Rustaceans"}}),
        )
        .unwrap();
    let id = follow["id"].as_str().unwrap().to_owned();
    let follows = f.call("follows", json!({})).unwrap();
    assert_eq!(follows[0]["name"], "Rustaceans", "{follows}");
    let listed = f
        .call("desktop_overview", json!({"request":{"after":null}}))
        .unwrap();
    assert!(
        listed["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == id.as_str() && c["title"] == "Rustaceans"),
        "{listed}"
    );
    f.call("unfollow_group", json!({"request":{"groupId":id}}))
        .unwrap();
    assert_eq!(f.call("follows", json!({})).unwrap(), json!([]));

    let g = Fixture::with_args(vec![
        "--directory".into(),
        directory(&key, "http://directory.example/v1/links/l1/login"),
        "--directory-key".into(),
        key,
    ])
    .await;
    g.identity("Bob");
    let refused = g.refusal("discover_link", json!({"request":{"kind":"github"}}));
    assert_eq!(refused["code"], "unsafe_link", "{refused}");
    assert_eq!(g.opened(), Vec::<String>::new());
}

/// Who may ask by id, and the decisions, as the contacts screen does them.
#[tokio::test]
async fn the_contacts_screen_sets_the_policy_and_decides_requests_through_the_daemon() {
    let f = Fixture::new().await;
    f.identity("Alice");
    assert_eq!(
        f.call("intro_policy", json!({})).unwrap(),
        json!({"mode":"all","dailyLimit":100,"allowed":[]})
    );
    let someone = format!("ain1{}", "ab".repeat(32));
    let policy = json!({"mode":"list","dailyLimit":5,"allowed":[someone]});
    assert_eq!(
        f.call("set_intro_policy", json!({ "request": policy }))
            .unwrap(),
        policy
    );
    assert_eq!(f.call("intro_policy", json!({})).unwrap(), policy);
    let refused = f.refusal(
        "set_intro_policy",
        json!({"request":{"mode":"sometimes","dailyLimit":5,"allowed":[]}}),
    );
    assert_eq!(refused["code"], "invalid_request", "{refused}");
    assert_eq!(f.call("intro_policy", json!({})).unwrap(), policy);

    assert_eq!(f.call("intro_requests", json!({})).unwrap(), json!([]));
    for method in ["accept_intro_request", "reject_intro_request"] {
        let refused = f.refusal(method, json!({"request":{"requestId":"ef".repeat(32)}}));
        assert_eq!(
            (refused["code"].as_str(), refused["retryable"].as_bool()),
            (Some("unknown_request"), Some(false)),
            "{method}: {refused}"
        );
    }
    // Without a swarm directory the card cannot be looked up yet, which is
    // retryable and answered at once.
    let started = Instant::now();
    let refused = f.refusal(
        "request_contact",
        json!({"request":{"networkId":someone,"name":"Someone","operationId":"ask-1"}}),
    );
    assert_eq!(
        (refused["code"].as_str(), refused["retryable"].as_bool()),
        (Some("network_unavailable"), Some(true)),
        "{refused}"
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

/// A group made in the window, as a member sees it.
#[tokio::test]
async fn the_groups_screen_makes_a_group_and_shows_the_owners_role() {
    let f = Fixture::new().await;
    let own = f.identity("Alice")["networkId"]
        .as_str()
        .unwrap()
        .to_owned();
    let made = f
        .call(
            "create_group",
            json!({"request":{"name":"Release team","members":[],"operationId":"g-1"}}),
        )
        .unwrap();
    assert_eq!(made["name"], "Release team");
    assert_eq!(made["role"], "owner");
    let id = made["id"].as_str().unwrap().to_owned();
    let listed = f.call("groups", json!({})).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1, "{listed}");
    assert_eq!(listed[0]["id"], id.as_str());
    let shown = f.call("group", json!({"request":{"groupId":id}})).unwrap();
    assert_eq!(shown["owner"], own.as_str());
    assert_eq!(shown["members"], json!([own]));
    assert_eq!(shown["admins"], json!([]));
    assert_eq!(shown["role"], "owner");
    assert!(
        f.snapshot()["conversations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == id.as_str()),
        "the group is a conversation of the chats screen"
    );
    let refused = f.refusal("group", json!({"request":{"groupId":"ef".repeat(32)}}));
    assert_eq!(refused["code"], "unknown_group", "{refused}");
    // Adding by id needs the member's card: without a directory it is
    // retryable and answered at once, and the group is unchanged.
    let started = Instant::now();
    let someone = format!("ain1{}", "ab".repeat(32));
    let refused = f.refusal(
        "change_group",
        json!({"request":{"groupId":id,"add":[someone],"operationId":"g-2"}}),
    );
    assert_eq!(refused["retryable"], true, "{refused}");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(
        f.call("group", json!({"request":{"groupId":id}})).unwrap()["members"],
        json!([own])
    );
}

#[tokio::test]
async fn the_wallet_reports_a_node_without_chain_or_identity_server_and_skills_install() {
    let f = Fixture::new().await;
    f.identity("Alice");
    let balance = f.call("coins_balance", json!({})).unwrap();
    assert_eq!(balance["remaining"], 0, "{balance}");
    assert_eq!(balance["books"], json!([]));
    assert!(balance["claim"].is_null(), "{balance}");
    let refused = f.refusal("coins_buy", json!({}));
    assert_eq!(
        (refused["code"].as_str(), refused["retryable"].as_bool()),
        (Some("chain_not_configured"), Some(false)),
        "{refused}"
    );
    let refused = f.refusal("claim_coins", json!({"request":{"provider":"google"}}));
    assert_eq!(refused["code"], "identity_not_configured", "{refused}");
    let refused = f.refusal(
        "open_payment",
        json!({"request":{"book":format!("0x{}", "11".repeat(32)),"step":"eth"}}),
    );
    assert_eq!(refused["code"], "unknown_payment", "{refused}");
    assert_eq!(f.opened(), Vec::<String>::new());

    // The agents screen installs the granted agent's skill as written; the
    // owner's skill names the CLI beside the app (profile.rs), which a
    // window on a daemon opened elsewhere does not know, so it writes none.
    let installed = f
        .call(
            "install_skill",
            json!({"request":{"skill":"agentic-messaging","host":"codex"}}),
        )
        .unwrap();
    let expected = f.home.join(".codex/skills/agentic-messaging/SKILL.md");
    assert_eq!(
        std::fs::canonicalize(installed["path"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&expected).unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(&expected).unwrap(),
        include_str!("../../../../../integrations/agent-skill/agentic-messaging/SKILL.md")
    );
    for request in [
        json!({"skill":"kaiki","host":"claude"}),
        json!({"skill":"../../evil","host":"claude"}),
        json!({"skill":"kaiki","host":"vim"}),
        json!({"skill":"kaiki","host":"claude","dir":"/tmp"}),
    ] {
        f.refusal("install_skill", json!({ "request": request }));
    }
    assert_eq!(
        files(&f.home),
        1,
        "only the granted agent's skill was written"
    );
}

/// Regular files under `dir`.
fn files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| {
                    let path = entry.path();
                    if path.is_dir() { files(&path) } else { 1 }
                })
                .sum()
        })
        .unwrap_or(0)
}

/// A stand-in HTTP service on loopback: `answer(request line, JSON body)`.
pub(crate) fn stub(answer: impl Fn(&str, &Value) -> Value + Send + 'static) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let mut length = 0usize;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
                    break;
                }
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let text = answer(&line, &body).to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
                text.len()
            );
        }
    });
    url
}

/// A stand-in identity server: a claim opens with `login`, then stays
/// pending (`POST /v1/claims`, `GET /v1/claims/{id}`).
fn identity_server(login: impl Fn(&str) -> String + Send + 'static) -> String {
    let base = std::sync::Arc::new(std::sync::OnceLock::<String>::new());
    let known = base.clone();
    let url = stub(move |line, _| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if line.starts_with("POST /v1/claims ") {
            json!({"claimId":"c1","loginUrl":login(known.get().unwrap()),"expiresAt":now + 900})
        } else {
            json!({"status":"pending"})
        }
    });
    base.set(url.clone()).unwrap();
    url
}

/// Asks again while the node waits for `code`, as the wallet screen does.
fn again(f: &Fixture, method: &str, code: &str) -> std::result::Result<Value, Value> {
    again_with(f, method, json!({}), code)
}

fn again_with(
    f: &Fixture,
    method: &str,
    body: Value,
    code: &str,
) -> std::result::Result<Value, Value> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match f.call(method, body.clone()) {
            Err(error) if error["code"] == code && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(200));
            }
            answer => return answer,
        }
    }
}

fn provider(name: &str) -> Value {
    json!({"request":{"provider":name}})
}

/// The Google and GitHub buttons open the daemon's login link at the chosen
/// provider; the window never takes a link from the webview.
#[tokio::test]
async fn logging_in_for_coins_opens_only_the_daemons_link_at_the_chosen_provider() {
    let server = identity_server(|base| format!("{base}/v1/claims/c1/login"));
    let f = Fixture::with_args(vec!["--identity-server".into(), server.clone()]).await;
    f.identity("Alice");
    let opened = again_with(&f, "claim_coins", provider("google"), "claim_pending").unwrap();
    let login = format!("{server}/v1/claims/c1/login");
    assert_eq!(opened["status"], "open", "{opened}");
    // The daemon's link as it gave it: the window adds the provider itself.
    assert_eq!(opened["loginUrl"], login.as_str());
    f.call("claim_coins", provider("github")).unwrap();
    assert_eq!(
        f.opened(),
        vec![format!("{login}/google"), format!("{login}/github")]
    );
    for request in [
        json!({"request":{"provider":"yahoo"}}),
        json!({"request":{"provider":"google","url":"https://evil.example"}}),
        json!({"request":{}}),
    ] {
        let refused = f.refusal("claim_coins", request);
        assert_eq!(refused["code"], "invalid_request", "{refused}");
    }
    assert_eq!(f.opened().len(), 2);
    // The wallet screen follows the claim through the balance.
    assert_eq!(
        f.call("coins_balance", json!({})).unwrap()["claim"]["loginUrl"],
        login.as_str()
    );

    // Plain http is opened only on loopback: a server that sends the owner
    // to another host over http gets nothing opened.
    let hostile = identity_server(|_| "http://id.example/v1/claims/c1/login".into());
    let g = Fixture::with_args(vec!["--identity-server".into(), hostile]).await;
    g.identity("Bob");
    let refused = again_with(&g, "claim_coins", provider("github"), "claim_pending")
        .expect_err("an unsafe login link");
    assert_eq!(refused["code"], "unsafe_link", "{refused}");
    assert_eq!(g.opened(), Vec::<String>::new());
}

/// A chain of one shop: every `eth_call` answers 1000 (price, book size,
/// validity, the USDC address, the ETH quote), the chain id is 31337 and the
/// head is block 16. With a stale rate the shop's `quote()` reverts.
fn chain(stale: bool) -> String {
    stub(move |_, body| {
        let data = body["params"][0]["data"].as_str().unwrap_or_default();
        if stale && data.starts_with("0x999b93af") {
            return json!({"jsonrpc":"2.0","id":body["id"],"error":{"code":3,"message":"execution reverted"}});
        }
        let result = match body["method"].as_str() {
            Some("eth_chainId") => json!("0x7a69"),
            Some("eth_blockNumber") => json!("0x10"),
            _ => json!(format!("0x{:064x}", 1000)),
        };
        json!({"jsonrpc":"2.0","id":body["id"],"result":result})
    })
}

/// A window on a node of that chain's shop.
async fn shop_window(shop: &str, stale: bool) -> Fixture {
    Fixture::with_args(
        [
            "--chain-rpc",
            &chain(stale),
            "--chain-id",
            "31337",
            "--book-shop",
            shop,
            "--grant-issuer",
            &format!("0x{}", "6e".repeat(20)),
            "--registry",
            &format!("0x{}", "7d".repeat(20)),
            "--chain-confirmations",
            "1",
        ]
        .map(String::from)
        .to_vec(),
    )
    .await
}

#[tokio::test]
async fn buying_a_book_opens_in_the_wallet_only_the_payment_the_daemon_made() {
    let shop = format!("0x{}", "5f".repeat(20));
    let f = shop_window(&shop, false).await;
    f.identity("Alice");
    // The first ask starts reading the shop's terms: retryable.
    let refused = f.refusal("coins_buy", json!({}));
    assert_eq!(
        (refused["code"].as_str(), refused["retryable"].as_bool()),
        (Some("chain_pending"), Some(true)),
        "{refused}"
    );
    let payment = again(&f, "coins_buy", "chain_pending").unwrap();
    // The price is in USD; it is paid in ETH at the quote, or in USDC in two
    // steps (allow the shop, then buy).
    let eth = payment["eth"]["uri"].as_str().unwrap().to_owned();
    assert!(
        eth.starts_with(&format!("ethereum:{shop}@31337/buy?")),
        "{eth}"
    );
    let approve = payment["usdc"]["approve"]["uri"]
        .as_str()
        .unwrap()
        .to_owned();
    let buy = payment["usdc"]["buy"]["uri"].as_str().unwrap().to_owned();
    let book = payment["book"].as_str().unwrap().to_owned();
    assert_eq!(
        f.call("coins_balance", json!({})).unwrap()["pending"][0]["book"],
        book.as_str()
    );
    for step in ["eth", "approve", "buy"] {
        f.call("open_payment", json!({"request":{"book":book,"step":step}}))
            .unwrap();
    }
    assert_eq!(f.opened(), vec![eth, approve, buy]);
    for (request, code) in [
        (
            json!({"book":format!("0x{}", "22".repeat(32)),"step":"eth"}),
            "unknown_payment",
        ),
        (json!({"book":book,"step":"sell"}), "invalid_request"),
        (json!({"book":book}), "invalid_request"),
        (
            json!({"book":book,"step":"eth","uri":"ethereum:0x00@1/buy"}),
            "invalid_request",
        ),
    ] {
        let refused = f.refusal("open_payment", json!({ "request": request }));
        assert_eq!(refused["code"], code, "{refused}");
    }
    assert_eq!(f.opened().len(), 3);
}

/// While the rate is stale the daemon offers no ETH payment: USDC is the way
/// to pay, and the ETH step is not opened.
#[tokio::test]
async fn with_a_stale_rate_the_wallet_still_opens_the_usdc_steps() {
    let f = shop_window(&format!("0x{}", "5f".repeat(20)), true).await;
    f.identity("Alice");
    let payment = again(&f, "coins_buy", "chain_pending").unwrap();
    assert!(payment["eth"].is_null(), "{payment}");
    let book = payment["book"].as_str().unwrap().to_owned();
    let refused = f.refusal(
        "open_payment",
        json!({"request":{"book":book,"step":"eth"}}),
    );
    assert_eq!(refused["code"], "unknown_payment", "{refused}");
    for step in ["approve", "buy"] {
        f.call("open_payment", json!({"request":{"book":book,"step":step}}))
            .unwrap();
    }
    assert_eq!(
        f.opened(),
        vec![
            payment["usdc"]["approve"]["uri"]
                .as_str()
                .unwrap()
                .to_owned(),
            payment["usdc"]["buy"]["uri"].as_str().unwrap().to_owned(),
        ]
    );
}
