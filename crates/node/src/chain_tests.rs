//! The chain reader against a local JSON-RPC stub answering with the
//! contract's own words (`contracts/test/BookShop.t.sol` pins the same
//! selectors and return data): reads happen at `head − confirmations` of the
//! head current for each read, and a wrong or unset chain reads nothing.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use agentic_mailbox_swarm::receipt::HolderKey;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SHOP: Address = [0x5b; 20];
const ISSUER: Address = [0x61; 20];
const REGISTRY: Address = [0x72; 20];
/// An operator pool address: the one of the `cast mktx` vector.
const POOL: Address = [
    0x3b, 0x6f, 0xcb, 0x55, 0x24, 0xa3, 0xa2, 0xf3, 0x0c, 0x7d, 0xaf, 0xce, 0xde, 0x23, 0x63, 0xfa,
    0x51, 0x12, 0xf5, 0x24,
];
/// The book of the contract test: key 0x22…22, salt 0x33…33, domain 0xa1…a1.
const BOOK: &str = "31165017cd3777e6f74336534ea20d41b9b3df2cdc92685eb57cca728191fa21";
/// `books(BOOK)` after the contract test's purchase: key, 100 stamps, valid
/// until 1_800_000_000 + 30 days.
const BOOK_WORDS: &str = concat!(
    "0000000000000000000000002222222222222222222222222222222222222222",
    "0000000000000000000000000000000000000000000000000000000000000064",
    "000000000000000000000000000000000000000000000000000000006b715f00",
);
const CHAIN: &str = "0x14a34";

fn book() -> [u8; 32] {
    hex::decode(BOOK).unwrap().try_into().unwrap()
}

fn bought() -> BookRecord {
    BookRecord {
        key: [0x22; 20],
        count: 100,
        valid_until: 1_802_592_000,
    }
}

fn word(value: u64) -> String {
    format!("{value:064x}")
}

/// A JSON-RPC endpoint. It answers `eth_blockNumber` itself with a head that
/// starts at `head` and grows by one per answer; `answer` gives every other
/// method's result (`Ok`) or error object (`Err`). It keeps every request
/// with the head current when it arrived.
struct Stub {
    url: String,
    requests: Arc<Mutex<Vec<(Value, u64)>>>,
}

impl Stub {
    fn count(&self, method: &str) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(r, _)| r["method"] == method)
            .count()
    }
    /// `eth_call`s with the last head answered before each.
    fn calls(&self) -> Vec<(Value, u64)> {
        let requests = self.requests.lock().unwrap();
        let mut head = None;
        let mut calls = Vec::new();
        for (request, current) in requests.iter() {
            match request["method"].as_str().unwrap() {
                "eth_blockNumber" => head = Some(*current),
                "eth_call" => calls.push((request.clone(), head.expect("a call before any head"))),
                _ => {}
            }
        }
        calls
    }
}

type Answer = dyn Fn(&Value) -> Result<Value, Value> + Send + Sync;

async fn stub(
    head: u64,
    answer: impl Fn(&Value) -> Result<Value, Value> + Send + Sync + 'static,
) -> Stub {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    let answer: Arc<Answer> = Arc::new(answer);
    let head = Arc::new(AtomicU64::new(head));
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let (seen, answer, head) = (seen.clone(), answer.clone(), head.clone());
            tokio::spawn(async move {
                let mut buffer = Vec::new();
                while let Some(body) = read_request(&mut socket, &mut buffer).await {
                    let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                    let reply = if request["method"] == "eth_blockNumber" {
                        let current = head.fetch_add(1, Ordering::SeqCst);
                        seen.lock().unwrap().push((request.clone(), current));
                        Ok(json!(format!("0x{current:x}")))
                    } else {
                        let current = head.load(Ordering::SeqCst);
                        seen.lock().unwrap().push((request.clone(), current));
                        answer(&request)
                    };
                    let reply = match reply {
                        Ok(result) => {
                            json!({"jsonrpc": "2.0", "id": request["id"], "result": result})
                        }
                        Err(error) => {
                            json!({"jsonrpc": "2.0", "id": request["id"], "error": error})
                        }
                    };
                    let body = serde_json::to_vec(&reply).unwrap();
                    let head = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
                        body.len()
                    );
                    if socket.write_all(head.as_bytes()).await.is_err()
                        || socket.write_all(&body).await.is_err()
                    {
                        return;
                    }
                }
            });
        }
    });
    Stub { url, requests }
}

/// One HTTP/1.1 request body with a content length; `None` at end of stream.
async fn read_request(socket: &mut tokio::net::TcpStream, buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    loop {
        if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buffer[..end]).to_ascii_lowercase();
            let length: usize = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0);
            if buffer.len() >= end + 4 + length {
                let body = buffer[end + 4..end + 4 + length].to_vec();
                buffer.drain(..end + 4 + length);
                return Some(body);
            }
        }
        let mut chunk = [0; 4096];
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
}

fn config(url: &str) -> ChainConfig {
    ChainConfig {
        url: url.into(),
        chain_id: 84_532,
        book_shop: SHOP,
        grant_issuer: ISSUER,
        registry: REGISTRY,
        operator_pool: Some(POOL),
        confirmations: 6,
    }
}

/// Chain 84532 answering every `eth_call` with `words`.
fn shop(words: String) -> impl Fn(&Value) -> Result<Value, Value> {
    move |request: &Value| match request["method"].as_str().unwrap_or("") {
        "eth_chainId" => Ok(json!(CHAIN)),
        "eth_call" => Ok(json!(format!("0x{words}"))),
        _ => Err(json!({"code": -32601, "message": "method not found"})),
    }
}

fn to(call: &Value) -> String {
    call["params"][0]["to"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
}

fn data(call: &Value) -> String {
    call["params"][0]["data"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
}

fn block(call: &Value) -> u64 {
    let tag = call["params"][1].as_str().unwrap();
    u64::from_str_radix(tag.trim_start_matches("0x"), 16).unwrap()
}

#[tokio::test]
async fn a_book_is_read_at_the_confirmed_block_of_the_head_of_each_read() {
    let rpc = stub(100, shop(BOOK_WORDS.into())).await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(chain.book(book()).await, Ok(Some(bought())));
    assert_eq!(chain.book(book()).await, Ok(Some(bought())));
    let calls = rpc.calls();
    assert_eq!(calls.len(), 2);
    for (call, head) in &calls {
        assert_eq!(to(call), format!("0x{}", hex::encode(SHOP)));
        assert_eq!(data(call), format!("0x0c0dee70{BOOK}"));
        assert_eq!(block(call), head - 6);
    }
    // The second read follows the chain's head, not the first read's block.
    assert!(block(&calls[1].0) > block(&calls[0].0));
    // The chain is checked once.
    assert_eq!(rpc.count("eth_chainId"), 1);
}

#[tokio::test]
async fn a_book_nobody_bought_reads_as_none() {
    let rpc = stub(100, shop("0".repeat(192))).await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(chain.book(book()).await, Ok(None));
}

/// A shop address without the contract answers an empty result: that is a
/// misconfiguration, not a chain where nobody bought anything.
#[tokio::test]
async fn an_empty_answer_is_an_error_not_an_unbought_book() {
    let rpc = stub(100, shop(String::new())).await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert!(matches!(
        chain.book(book()).await,
        Err(ChainError::Malformed(_))
    ));
}

#[tokio::test]
async fn a_node_pointed_at_another_chain_reads_nothing() {
    let rpc = stub(100, |request: &Value| {
        match request["method"].as_str().unwrap_or("") {
            "eth_chainId" => Ok(json!("0x1")),
            _ => Ok(json!(format!("0x{BOOK_WORDS}"))),
        }
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(
        chain.book(book()).await,
        Err(ChainError::WrongChain {
            expected: 84_532,
            actual: 1
        })
    );
    assert!(rpc.calls().is_empty());
}

/// An RPC that fails at a node's first read is asked again: the chain
/// check is kept only once it passed.
#[tokio::test]
async fn a_failed_chain_check_is_made_again_on_the_next_read() {
    let failed = Arc::new(AtomicU64::new(0));
    let first = failed.clone();
    let rpc = stub(100, move |request: &Value| {
        match request["method"].as_str().unwrap_or("") {
            "eth_chainId" if first.fetch_add(1, Ordering::SeqCst) == 0 => {
                Err(json!({"code": -32603, "message": "starting"}))
            }
            "eth_chainId" => Ok(json!(CHAIN)),
            _ => Ok(json!(format!("0x{BOOK_WORDS}"))),
        }
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert!(matches!(
        chain.book(book()).await,
        Err(ChainError::Transport(_))
    ));
    assert_eq!(chain.book(book()).await, Ok(Some(bought())));
    assert_eq!(chain.book(book()).await, Ok(Some(bought())));
    assert_eq!(rpc.count("eth_chainId"), 2);
}

#[tokio::test]
async fn a_chain_shorter_than_the_confirmations_confirms_nothing() {
    let rpc = stub(5, shop(BOOK_WORDS.into())).await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(chain.book(book()).await, Err(ChainError::NotConfirmed));
    assert!(rpc.calls().is_empty());
}

#[tokio::test]
async fn an_rpc_error_or_an_unreachable_endpoint_is_a_transport_failure() {
    let rpc = stub(100, |request: &Value| {
        match request["method"].as_str().unwrap_or("") {
            "eth_chainId" => Ok(json!(CHAIN)),
            _ => Err(json!({"code": -32000, "message": "header not found"})),
        }
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert!(matches!(
        chain.book(book()).await,
        Err(ChainError::Transport(_))
    ));
    // Nothing listens on a port just released.
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let chain = Rpc::new(config(&format!("http://{closed}"))).unwrap();
    assert!(matches!(
        chain.book(book()).await,
        Err(ChainError::Transport(_))
    ));
}

#[tokio::test]
async fn grant_rules_are_read_for_one_issuer_and_day_at_one_block() {
    const SERVER: Account = [0x31; 20];
    const DAY: u64 = 19_675;
    let rpc = stub(100, |request: &Value| {
        if request["method"] == "eth_chainId" {
            return Ok(json!(CHAIN));
        }
        let data = request["params"][0]["data"].as_str().unwrap_or("");
        let result = match data.get(2..10).unwrap_or("") {
            "39f65888" => word(100),
            "a7911730" => word(30),
            "b74e452b" => word(DAY + 1),
            "6619052a" => word(1),
            "ff33ddf4" => word(1_000),
            _ => return Err(json!({"code": 3, "message": "execution reverted"})),
        };
        Ok(json!(format!("0x{result}")))
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    let rules = chain.grant_day(SERVER, DAY).await;
    let calls = rpc.calls();
    let asked: std::collections::BTreeSet<String> = calls.iter().map(|(c, _)| data(c)).collect();
    let server = format!("{:0>64}", hex::encode(SERVER));
    for expected in [
        "0x39f65888".to_owned(),
        "0xa7911730".to_owned(),
        "0xb74e452b".to_owned(),
        format!("0x6619052a{server}{}", word(DAY)),
        format!("0xff33ddf4{}", word(DAY)),
    ] {
        assert!(asked.contains(&expected), "{expected} not asked: {asked:?}");
    }
    let issuer = format!("0x{}", hex::encode(ISSUER));
    assert!(calls.iter().all(|(c, _)| to(c) == issuer));
    // All at one confirmed block, although the head moves between calls.
    let blocks: std::collections::BTreeSet<u64> = calls.iter().map(|(c, _)| block(c)).collect();
    assert_eq!(blocks.len(), 1, "{blocks:?}");
    let first_head = calls.iter().map(|(_, head)| *head).min().unwrap();
    assert_eq!(*blocks.first().unwrap(), first_head - 6);
    assert_eq!(
        rules,
        Ok(GrantDay {
            active: true,
            cap_coins: 1_000,
            book_size: 100,
            max_validity_days: 30,
            today: DAY + 1,
        })
    );
}

/// The shop's terms for `coins buy`, read from the configured shop: the USD
/// price (USDC units), the USDC token, the ETH quote at the current rate,
/// the book size and validity.
/// `quote`: `Ok` answers that many wei, `Err` answers that JSON-RPC error.
async fn shop_answering(
    quote: Result<u128, Value>,
) -> (Result<ShopTerms, ChainError>, Vec<String>) {
    let rpc = stub(100, move |request: &Value| {
        if request["method"] == "eth_chainId" {
            return Ok(json!(CHAIN));
        }
        let data = request["params"][0]["data"].as_str().unwrap_or("");
        let result = match (data.get(2..10).unwrap_or(""), &quote) {
            ("34189d5e", _) => word(1_000_000),
            ("3e413bee", _) => format!("{:0>64}", hex::encode([0x0c; 20])),
            ("999b93af", Ok(quote)) => format!("{quote:064x}"),
            ("999b93af", Err(error)) => return Err(error.clone()),
            ("39f65888", _) => word(100),
            ("3e98d1fb", _) => word(30 * 86_400),
            _ => return Err(json!({"code": -32601, "message": "method not found"})),
        };
        Ok(json!(format!("0x{result}")))
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    let terms = chain.shop().await;
    let calls = rpc.calls();
    let shop = format!("0x{}", hex::encode(SHOP));
    assert!(calls.iter().all(|(c, _)| to(c) == shop));
    (terms, calls.iter().map(|(c, _)| data(c)).collect())
}

#[tokio::test]
async fn the_shop_terms_are_read_from_the_configured_shop() {
    // Beyond 64 bits of wei.
    let (terms, asked) = shop_answering(Ok(u128::from(u64::MAX) + 1)).await;
    let asked: std::collections::BTreeSet<String> = asked.into_iter().collect();
    assert_eq!(
        asked,
        [
            "0x34189d5e",
            "0x39f65888",
            "0x3e413bee",
            "0x3e98d1fb",
            "0x999b93af"
        ]
        .map(String::from)
        .into()
    );
    let terms = terms.unwrap();
    assert_eq!(
        terms,
        ShopTerms {
            address: SHOP,
            chain_id: 84_532,
            price_usdc: 1_000_000,
            usdc: [0x0c; 20],
            quote: Some(u128::from(u64::MAX) + 1),
            book_size: 100,
            validity: 30 * 86_400,
        }
    );
    // A stale rate: `quote()` reverts (as a node answers a custom error).
    // There is no ETH quote then; USDC still has a price.
    let stale = json!({
        "code": 3,
        "message": "execution reverted: custom error 0x19abf40e",
        "data": "0x19abf40e",
    });
    assert_eq!(
        shop_answering(Err(stale)).await.0,
        Ok(ShopTerms {
            quote: None,
            ..terms
        })
    );
    // A failing RPC is not a stale rate: the read fails and is tried again.
    let limited = json!({"code": -32005, "message": "request rate exceeded"});
    assert!(matches!(
        shop_answering(Err(limited)).await.0,
        Err(ChainError::Transport(_))
    ));
}

/// `serve` takes all chain flags or none.
#[test]
fn chain_flags_are_all_or_nothing() {
    let shop = "0x5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b5b";
    let issuer = "0x6161616161616161616161616161616161616161";
    let registry = "0x7272727272727272727272727272727272727272";
    let rpc = "https://sepolia.base.org";
    let flags = |rpc, chain, shop, issuer, registry, confirmations| ChainFlags {
        rpc,
        chain_id: chain,
        book_shop: shop,
        grant_issuer: issuer,
        registry,
        operator_pool: None,
        confirmations,
    };
    assert_eq!(
        chain_config(&flags(None, None, None, None, None, 6)),
        Ok(None)
    );
    assert_eq!(
        chain_config(&flags(
            Some(rpc),
            Some(84_532),
            Some(shop),
            Some(issuer),
            Some(registry),
            6
        )),
        Ok(Some(ChainConfig {
            url: rpc.into(),
            chain_id: 84_532,
            book_shop: SHOP,
            grant_issuer: ISSUER,
            registry: REGISTRY,
            operator_pool: None,
            confirmations: 6,
        }))
    );
    // The operator pool is optional, and only with the rest.
    let pool = "0x3b6fcb5524a3a2f30c7dafcede2363fa5112f524";
    assert_eq!(
        chain_config(&ChainFlags {
            operator_pool: Some(pool),
            ..flags(
                Some(rpc),
                Some(84_532),
                Some(shop),
                Some(issuer),
                Some(registry),
                6
            )
        })
        .map(|config| config.and_then(|c| c.operator_pool)),
        Ok(Some(POOL))
    );
    assert!(
        chain_config(&ChainFlags {
            operator_pool: Some(pool),
            ..flags(None, None, None, None, None, 6)
        })
        .is_err()
    );
    for (label, partial) in [
        (
            "no RPC",
            flags(
                None,
                Some(84_532),
                Some(shop),
                Some(issuer),
                Some(registry),
                6,
            ),
        ),
        (
            "no issuer",
            flags(Some(rpc), Some(84_532), Some(shop), None, Some(registry), 6),
        ),
        (
            "no registry",
            flags(Some(rpc), Some(84_532), Some(shop), Some(issuer), None, 6),
        ),
        (
            "a short address",
            flags(
                Some(rpc),
                Some(84_532),
                Some("0x5b5b"),
                Some(issuer),
                Some(registry),
                6,
            ),
        ),
        (
            "no confirmations",
            flags(
                Some(rpc),
                Some(84_532),
                Some(shop),
                Some(issuer),
                Some(registry),
                0,
            ),
        ),
    ] {
        assert!(chain_config(&partial).is_err(), "{label}");
    }
}

/// The directory's members: the active units' commitments, paged through
/// `NodeRegistry.activeUnits(start, limit)` at one confirmed block.
#[tokio::test]
async fn the_active_units_are_read_page_by_page_at_one_block() {
    let positions: Vec<[u8; 32]> = (0..300u32)
        .map(|n| {
            let mut unit = [0x55; 32];
            unit[..4].copy_from_slice(&n.to_be_bytes());
            unit
        })
        .collect();
    // Every seventh unit exited: pages come back short while `next` still
    // moves on by the limit.
    let exited = |position: usize| position % 7 == 3;
    let units: Vec<[u8; 32]> = positions
        .iter()
        .enumerate()
        .filter(|(n, _)| !exited(*n))
        .map(|(_, unit)| *unit)
        .collect();
    let served = positions.clone();
    let rpc = stub(100, move |request: &Value| {
        if request["method"] == "eth_chainId" {
            return Ok(json!(CHAIN));
        }
        let data = request["params"][0]["data"].as_str().unwrap_or("");
        if data.get(2..10) != Some("5dab6b66") {
            return Err(json!({"code": 3, "message": "execution reverted"}));
        }
        let word = |at: usize| {
            u64::from_str_radix(&data[10 + 64 * at + 48..10 + 64 * (at + 1)], 16).unwrap()
        };
        let (start, limit) = (word(0) as usize, word(1) as usize);
        let end = (start + limit).min(served.len());
        let page: Vec<&[u8; 32]> = (start.min(end)..end)
            .filter(|n| !exited(*n))
            .map(|n| &served[n])
            .collect();
        // (bytes32[] commitments, uint64 next)
        let mut words = format!("{:064x}{:064x}{:064x}", 0x40, end, page.len());
        for unit in page {
            words.push_str(&hex::encode(unit));
        }
        Ok(json!(format!("0x{words}")))
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(chain.units().await, Ok(units));
    let calls = rpc.calls();
    assert!(
        calls.len() >= 2,
        "one page cannot scan 300 positions: {calls:?}"
    );
    let registry = format!("0x{}", hex::encode(REGISTRY));
    assert!(calls.iter().all(|(c, _)| to(c) == registry));
    let blocks: std::collections::BTreeSet<u64> = calls.iter().map(|(c, _)| block(c)).collect();
    assert_eq!(blocks.len(), 1, "{blocks:?}");
}

// --- operator payouts (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md) -------------

fn receipt_key() -> HolderKey {
    HolderKey::from_bytes(&[0x11; 32]).unwrap()
}

/// `claim` of two tickets, as `cast calldata` encodes it for these fields;
/// the seed calls take the day.
#[test]
fn a_claim_is_encoded_as_the_operator_pool_decodes_it() {
    let mut holders = [[0; 32]; 10];
    for (n, holder) in holders.iter_mut().enumerate() {
        holder[31] = u8::try_from(n + 1).unwrap();
    }
    let signature = |r: u8, s: u8, v: u8| {
        let mut signature = [r; 65];
        signature[32..64].copy_from_slice(&[s; 32]);
        signature[64] = v;
        signature
    };
    let mut second_envelope = [0; 32];
    second_envelope[31] = 1;
    let tickets = [
        TicketClaim {
            book: book(),
            index: 7,
            mailbox: [0x11; 32],
            period: 20_833,
            envelope: alloy_primitives::keccak256(b"envelope").0,
            holders,
            position: 2,
            signature: signature(0xaa, 0xbb, 27),
        },
        TicketClaim {
            book: book(),
            index: 8,
            mailbox: [0x22; 32],
            period: 20_834,
            envelope: second_envelope,
            holders,
            position: 9,
            signature: signature(0xcc, 0xdd, 28),
        },
    ];
    let data = claim_calldata(3, &[0x77; 32], &tickets);
    assert_eq!(data.len(), 1_348);
    assert_eq!(
        hex::encode(alloy_primitives::keccak256(&data)),
        "ed0d96b8dbec60aef986f57dcf3b1c7a99a94f2624ed735e52bb6add96843a6a"
    );
    assert_eq!(
        hex::encode(arm_seed_calldata(20_833)),
        "c9bc7a8c0000000000000000000000000000000000000000000000000000000000005161"
    );
    assert_eq!(
        hex::encode(capture_seed_calldata(20_833)),
        "0628829b0000000000000000000000000000000000000000000000000000000000005161"
    );
    assert_eq!(
        hex::encode(pay_out_calldata(3, &[0x77; 32])),
        format!("f6a08139{}{}", word(3), "77".repeat(32))
    );
}

/// The receipt key signs an EIP-1559 transaction byte for byte as
/// `cast mktx --private-key 0x11…11 --chain 84532 --nonce 7 --gas-limit 90000
/// --gas-price 12000000 --priority-gas-price 1000000 <pool> "armSeed(uint64)" 20833`.
#[test]
fn a_transaction_is_signed_as_cast_signs_it() {
    let key = receipt_key();
    assert_eq!(
        hex::encode(key.account()),
        "19e7e376e7c213b7e7e7e46cc70a5dd086daff2a"
    );
    let raw = signed_transaction(
        &Transaction {
            chain_id: 84_532,
            nonce: 7,
            max_priority_fee: 1_000_000,
            max_fee: 12_000_000,
            gas: 90_000,
            to: POOL,
            data: arm_seed_calldata(20_833),
        },
        &key,
    );
    assert_eq!(
        hex::encode(raw),
        concat!(
            "02f89083014a3407830f424083b71b0083015f90943b6fcb5524a3a2f30c7dafcede2363fa5112f524",
            "80a4c9bc7a8c0000000000000000000000000000000000000000000000000000000000005161c080a0",
            "96a999efb7d6d0774e5abc094461deafc7fb32c310529c6abcfeb92340de103ba0376fbbba2fad32",
            "b434c0c277b4d4afa42d2caf7d2ab375edb006b2b92e04bd07"
        )
    );
}

/// A chain that takes transactions: the estimate, nonce, fee and receipt
/// methods answer as given; `sent` collects raw transactions.
fn taking(
    estimate: Result<Value, Value>,
    status: &'static str,
    balance: &'static str,
    sent: Arc<Mutex<Vec<String>>>,
) -> impl Fn(&Value) -> Result<Value, Value> {
    move |request: &Value| match request["method"].as_str().unwrap_or("") {
        "eth_chainId" => Ok(json!(CHAIN)),
        "eth_getBalance" => Ok(json!(balance)),
        "eth_estimateGas" => estimate.clone(),
        "eth_getTransactionCount" => Ok(json!("0x7")),
        "eth_maxPriorityFeePerGas" => Ok(json!("0xf4240")),
        "eth_getBlockByNumber" => Ok(json!({"baseFeePerGas": "0x5b8d80"})),
        "eth_sendRawTransaction" => {
            let raw = request["params"][0].as_str().unwrap().to_owned();
            sent.lock().unwrap().push(raw);
            Ok(json!(format!("0x{}", "ab".repeat(32))))
        }
        "eth_getTransactionReceipt" => Ok(json!({"status": status})),
        _ => Err(json!({"code": -32601, "message": "method not found"})),
    }
}

/// A node sends only what the chain estimates to go through: it signs at the
/// pending nonce with a fifth more gas than the estimate and a fee cap of
/// twice the base fee plus the tip, and waits for the receipt.
#[tokio::test]
async fn a_transaction_is_estimated_signed_sent_and_awaited() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    // An estimate of 72000 gas; the base fee is 6000000 wei.
    let rpc = stub(
        100,
        taking(
            Ok(json!("0x11940")),
            "0x1",
            "0xde0b6b3a7640000",
            sent.clone(),
        ),
    )
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    let key = receipt_key();
    let data = arm_seed_calldata(20_833);
    assert_eq!(
        chain.transact(&key, POOL, data.clone()).await,
        Ok([0xab; 32])
    );
    let raw = sent.lock().unwrap().clone();
    let expected = signed_transaction(
        &Transaction {
            chain_id: 84_532,
            nonce: 7,
            max_priority_fee: 1_000_000,
            max_fee: 13_000_000,
            gas: 86_400,
            to: POOL,
            data: data.clone(),
        },
        &key,
    );
    assert_eq!(raw, [format!("0x{}", hex::encode(expected))]);
    let requests = rpc.requests.lock().unwrap().clone();
    let estimate = requests
        .iter()
        .find(|(r, _)| r["method"] == "eth_estimateGas")
        .unwrap();
    assert_eq!(
        estimate.0["params"][0]["from"],
        format!("0x{}", hex::encode(key.account()))
    );
    assert_eq!(
        estimate.0["params"][0]["to"],
        format!("0x{}", hex::encode(POOL))
    );
    assert_eq!(
        estimate.0["params"][0]["data"],
        format!("0x{}", hex::encode(&data))
    );
    let nonce = requests
        .iter()
        .find(|(r, _)| r["method"] == "eth_getTransactionCount")
        .unwrap();
    assert_eq!(nonce.0["params"][1], "pending");
}

/// A call the contract would refuse is not sent, nor one the receipt
/// account cannot pay the gas of; a sent one that failed is reported as
/// refused.
#[tokio::test]
async fn a_refused_or_unpaid_transaction_is_never_sent_and_a_failed_one_is_reported() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let revert = json!({"code": 3, "message": "execution reverted", "data": "0x4dd5a0c7"});
    let rpc = stub(
        100,
        taking(Err(revert), "0x1", "0xde0b6b3a7640000", sent.clone()),
    )
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(
        chain
            .transact(&receipt_key(), POOL, arm_seed_calldata(1))
            .await,
        Err(ChainError::Reverted)
    );
    assert!(sent.lock().unwrap().is_empty());
    // 86400 gas at 13000000 wei is more than the account holds.
    let rpc = stub(
        100,
        taking(Ok(json!("0x11940")), "0x1", "0x10000", sent.clone()),
    )
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(
        chain
            .transact(&receipt_key(), POOL, arm_seed_calldata(1))
            .await,
        Err(ChainError::NoGas)
    );
    assert!(sent.lock().unwrap().is_empty());
    let rpc = stub(
        100,
        taking(
            Ok(json!("0x11940")),
            "0x0",
            "0xde0b6b3a7640000",
            sent.clone(),
        ),
    )
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(
        chain
            .transact(&receipt_key(), POOL, arm_seed_calldata(1))
            .await,
        Err(ChainError::Reverted)
    );
    assert_eq!(sent.lock().unwrap().len(), 1);
}

/// The pool's terms and a day's seed are read at a confirmed block, with
/// the block the seed was armed for and the chain's head.
#[tokio::test]
async fn the_pools_terms_and_seeds_are_read_from_the_configured_pool() {
    let seed = "5eed".repeat(16);
    let answered = seed.clone();
    let rpc = stub(100, move |request: &Value| {
        if request["method"] == "eth_chainId" {
            return Ok(json!(CHAIN));
        }
        let data = request["params"][0]["data"].as_str().unwrap_or("");
        let result = match data.get(2..10).unwrap_or("") {
            "ed96cc51" => format!(
                "{:0>64}",
                "3afb7e90ff972474538ef34d6a161e4f765fd8adab9f559b3d07c6a2df8000"
            ),
            "f6a18664" => word(100_000),
            "5bbb8b82" => word(360 * 86_400),
            // `seeds(20833)` was captured from block 90; 20834 is armed for
            // block 150; 20835 is not armed.
            "c2f1c307" if data.ends_with(&word(20_833)) => format!("{}{answered}", word(90)),
            "c2f1c307" if data.ends_with(&word(20_834)) => format!("{}{}", word(150), word(0)),
            "c2f1c307" => format!("{}{}", word(0), word(0)),
            _ => return Err(json!({"code": 3, "message": "execution reverted"})),
        };
        Ok(json!(format!("0x{result}")))
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    let mut threshold = [0; 32];
    threshold[1..].copy_from_slice(
        &hex::decode("3afb7e90ff972474538ef34d6a161e4f765fd8adab9f559b3d07c6a2df8000").unwrap(),
    );
    assert_eq!(
        chain.pool().await,
        Ok(PoolTerms {
            address: POOL,
            win_threshold: threshold,
            prize_usdc: 100_000,
            ticket_lifetime: 360 * 86_400,
        })
    );
    let mut captured = [0; 32];
    captured.copy_from_slice(&hex::decode(&seed).unwrap());
    let known = chain.seed_state(20_833).await.unwrap();
    assert_eq!((known.target, known.seed), (90, Some(captured)));
    let armed = chain.seed_state(20_834).await.unwrap();
    assert_eq!((armed.target, armed.seed), (150, None));
    assert!(armed.head >= 100, "the chain's head: {}", armed.head);
    let unarmed = chain.seed_state(20_835).await.unwrap();
    assert_eq!((unarmed.target, unarmed.seed), (0, None));
    let pool = format!("0x{}", hex::encode(POOL));
    for (call, head) in rpc.calls() {
        assert_eq!(to(&call), pool);
        assert_eq!(block(&call), head - 6);
    }
}

/// A node finds its own unit's registry index and owner by its commitment,
/// also once the unit is leaving: what it earned is still paid.
#[tokio::test]
async fn a_unit_is_found_in_the_registry_by_its_commitment() {
    let unit = |n: u64| [0x40 + u8::try_from(n).unwrap(); 32];
    let rpc = stub(100, move |request: &Value| {
        if request["method"] == "eth_chainId" {
            return Ok(json!(CHAIN));
        }
        let data = request["params"][0]["data"].as_str().unwrap_or("");
        let result = match data.get(2..10).unwrap_or("") {
            "fc7e9c6f" => word(3),
            "a43ec96e" => {
                let index = u64::from_str_radix(&data[data.len() - 16..], 16).unwrap();
                // owner, commitment, exitRequestedAt, withdrawAfter, state;
                // unit 2 is exiting.
                format!(
                    "{:0>64}{}{}{}{}",
                    format!("c0{:038x}", index),
                    hex::encode(unit(index)),
                    word(0),
                    word(0),
                    word(if index == 2 { 2 } else { 1 })
                )
            }
            _ => return Err(json!({"code": 3, "message": "execution reverted"})),
        };
        Ok(json!(format!("0x{result}")))
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    let mut owner = [0; 20];
    owner[0] = 0xc0;
    owner[19] = 2;
    assert_eq!(
        chain.registry_unit(unit(2)).await,
        Ok(Some(RegistryUnit { index: 2, owner }))
    );
    assert_eq!(chain.registry_unit([0x99; 32]).await, Ok(None));
    let registry = format!("0x{}", hex::encode(REGISTRY));
    assert!(rpc.calls().iter().all(|(call, _)| to(call) == registry));
}

/// What the pool owes a unit and the gas an account holds are read as
/// numbers wider than 64 bits.
#[tokio::test]
async fn what_the_pool_owes_and_the_gas_held_are_read() {
    let rpc = stub(100, move |request: &Value| {
        match request["method"].as_str().unwrap_or("") {
            "eth_chainId" => Ok(json!(CHAIN)),
            "eth_getBalance" => Ok(json!("0x1bc16d674ec80000")),
            "eth_call" => {
                let data = request["params"][0]["data"].as_str().unwrap_or("");
                if data == format!("0x239118ce{}", word(3)) {
                    Ok(json!(format!("0x{:064x}", u128::from(u64::MAX) + 5)))
                } else {
                    Err(json!({"code": 3, "message": "execution reverted"}))
                }
            }
            _ => Err(json!({"code": -32601, "message": "method not found"})),
        }
    })
    .await;
    let chain = Rpc::new(config(&rpc.url)).unwrap();
    assert_eq!(chain.owed(3).await, Ok(u128::from(u64::MAX) + 5));
    assert_eq!(
        chain.gas_balance([0x19; 20]).await,
        Ok(2_000_000_000_000_000_000)
    );
    let balance = rpc
        .requests
        .lock()
        .unwrap()
        .iter()
        .find(|(r, _)| r["method"] == "eth_getBalance")
        .unwrap()
        .0
        .clone();
    assert_eq!(balance["params"][0], format!("0x{}", "19".repeat(20)));
}

/// On a real chain with a freshly deployed pool (`scripts/deploy-contracts.py`
/// on a local anvil; one run per deployment): the node arms yesterday's seed, captures it past its target
/// block and reads it back; an account without ETH sends nothing. Run with
/// `AIN_TEST_RPC`, `AIN_TEST_POOL` and `AIN_TEST_KEY` (a funded key, hex).
#[tokio::test]
#[ignore = "needs a local anvil with a deployed OperatorPool"]
async fn a_real_chain_takes_the_nodes_seed_transactions() {
    let (Ok(url), Ok(pool), Ok(key)) = (
        std::env::var("AIN_TEST_RPC"),
        std::env::var("AIN_TEST_POOL"),
        std::env::var("AIN_TEST_KEY"),
    ) else {
        panic!("set AIN_TEST_RPC, AIN_TEST_POOL and AIN_TEST_KEY");
    };
    let pool: Address = hex::decode(pool.trim_start_matches("0x"))
        .unwrap()
        .try_into()
        .unwrap();
    let secret: [u8; 32] = hex::decode(key.trim_start_matches("0x"))
        .unwrap()
        .try_into()
        .unwrap();
    let key = HolderKey::from_bytes(&secret).unwrap();
    let chain_id =
        u64::try_from(quantity(&serde_json::from_str::<Value>(r#""0x7a69""#).unwrap()).unwrap())
            .unwrap();
    let chain = Rpc::new(ChainConfig {
        url,
        chain_id,
        book_shop: SHOP,
        grant_issuer: ISSUER,
        registry: REGISTRY,
        operator_pool: Some(pool),
        confirmations: 1,
    })
    .unwrap();
    // The pool is read at a confirmed block: past its deployment.
    chain.request("anvil_mine", json!(["0x2"])).await.unwrap();
    let latest = chain
        .request("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap();
    let day = u64::try_from(quantity(&latest["timestamp"]).unwrap()).unwrap() / 86_400 - 1;
    assert_eq!(chain.seed_state(day).await.unwrap().seed, None);
    chain
        .transact(&key, pool, arm_seed_calldata(day))
        .await
        .unwrap();
    // Capturing before the target block is refused and never sent.
    assert_eq!(
        chain.transact(&key, pool, capture_seed_calldata(day)).await,
        Err(ChainError::Reverted)
    );
    chain.request("anvil_mine", json!(["0x6"])).await.unwrap();
    chain
        .transact(&key, pool, capture_seed_calldata(day))
        .await
        .unwrap();
    chain.request("anvil_mine", json!(["0x2"])).await.unwrap();
    let state = chain.seed_state(day).await.unwrap();
    assert!(state.seed.is_some() && state.target > 0, "{state:?}");
    let empty = HolderKey::from_bytes(&[0x44; 32]).unwrap();
    assert_eq!(
        chain
            .transact(&empty, pool, arm_seed_calldata(day - 1))
            .await,
        Err(ChainError::NoGas)
    );
}

/// Against a freshly deployed pool on a local anvil
/// (`scripts/deploy-contracts.py`; one run per deployment):
/// a unit bonded with this node's receipt account, books bought in USDC, the
/// day's seed fixed by the node, a winning paid stamp naming the unit — the
/// node's own claim pays the unit's owner $0.10, and a second claim of the
/// ticket is refused unsent. Run with `AIN_TEST_RPC` and `AIN_TEST_POOL`
/// (anvil's first account pays for the setup).
#[tokio::test]
#[ignore = "needs a local anvil with a deployed OperatorPool"]
async fn a_real_pool_pays_the_nodes_claim_of_a_winning_ticket() {
    use agentic_mailbox_swarm::stamp::{BookKey, Stamp, book_id, named_operation, swarm_digest};
    let (Ok(url), Ok(pool)) = (
        std::env::var("AIN_TEST_RPC"),
        std::env::var("AIN_TEST_POOL"),
    ) else {
        panic!("set AIN_TEST_RPC and AIN_TEST_POOL");
    };
    let pool: Address = hex::decode(pool.trim_start_matches("0x"))
        .unwrap()
        .try_into()
        .unwrap();
    let chain = Rpc::new(ChainConfig {
        url,
        chain_id: 31_337,
        book_shop: SHOP,
        grant_issuer: ISSUER,
        registry: REGISTRY,
        operator_pool: Some(pool),
        confirmations: 1,
    })
    .unwrap();
    let head = || async { chain.quantity("eth_blockNumber").await.unwrap() };
    let read = |to: Address, data: Vec<u8>| {
        let chain = &chain;
        async move {
            let block = chain.quantity("eth_blockNumber").await.unwrap();
            chain.call(&to, &data, block).await.unwrap()
        }
    };
    let address_of = |word: Vec<u8>| -> Address { word[12..32].try_into().unwrap() };
    let selector =
        |signature: &str| alloy_primitives::keccak256(signature.as_bytes()).0[..4].to_vec();
    let sig = |signature: &str, words: &[[u8; 32]]| [selector(signature), words.concat()].concat();
    let payer = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    // Setup transactions from anvil's unlocked first account, each mined and
    // checked before the next.
    let send = |to: Address, value: u128, data: Vec<u8>| {
        let chain = &chain;
        async move {
            let hash = chain
                .request(
                    "eth_sendTransaction",
                    json!([{
                        "from": payer, "to": hex0x(to), "value": format!("0x{value:x}"),
                        "data": hex0x(&data), "gas": "0x2dc6c0",
                        "maxFeePerGas": "0x2540be400", "maxPriorityFeePerGas": "0x1",
                    }]),
                )
                .await
                .unwrap();
            chain.request("anvil_mine", json!(["0x1"])).await.unwrap();
            let receipt = chain
                .request("eth_getTransactionReceipt", json!([hash]))
                .await
                .unwrap();
            assert_eq!(
                receipt["status"],
                "0x1",
                "setup transaction to {}",
                hex0x(to)
            );
        }
    };
    let shop = address_of(read(pool, sig("shop()", &[])).await);
    let registry = address_of(read(pool, sig("registry()", &[])).await);
    let usdc = address_of(read(shop, sig("usdc()", &[])).await);
    let domain: [u8; 32] = read(pool, sig("domain()", &[])).await.try_into().unwrap();
    // The node: a receipt key with ETH for gas, and its unit bonded by the
    // owner (anvil's first account).
    let receipt = HolderKey::from_bytes(&[0x51; 32]).unwrap();
    chain
        .request(
            "anvil_setBalance",
            json!([hex0x(receipt.account()), "0xde0b6b3a7640000"]),
        )
        .await
        .unwrap();
    let transport = [0x7a; 32];
    let unit =
        agentic_mailbox_swarm::directory::unit_commitment(&domain, &transport, &receipt.account());
    let index = uint(&read(registry, sig("nextIndex()", &[])).await).unwrap();
    let bond = uint128(&read(registry, sig("unitBondWei()", &[])).await).unwrap();
    send(registry, bond, sig("bond(bytes32)", &[unit])).await;
    // Five books today, in USDC.
    let key = BookKey::from_bytes(&[0x3c; 32]).unwrap();
    let price = uint128(&read(shop, sig("priceUsdc()", &[])).await).unwrap();
    let mut amount = [0; 32];
    amount[16..].copy_from_slice(&(price * 5).to_be_bytes());
    send(
        usdc,
        0,
        sig(
            "mint(address,uint256)",
            &[address_argument(&address_of_str(payer)), amount],
        ),
    )
    .await;
    send(
        usdc,
        0,
        sig(
            "approve(address,uint256)",
            &[address_argument(&shop), amount],
        ),
    )
    .await;
    let books: Vec<[u8; 32]> = (0..5u8)
        .map(|n| book_id(&domain, &key.account(), &[n; 32]))
        .collect();
    for n in 0..5u8 {
        send(
            shop,
            0,
            sig(
                "buyWithUsdc(address,bytes32)",
                &[address_argument(&key.account()), [n; 32]],
            ),
        )
        .await;
    }
    chain.request("anvil_mine", json!(["0x1"])).await.unwrap();
    // The books' purchase day, as the pool reads it.
    let record = decode_book(&read(shop, sig("books(bytes32)", &[books[0]])).await)
        .unwrap()
        .unwrap();
    let validity = uint(&read(shop, sig("validity()", &[])).await).unwrap();
    let day = (record.valid_until - validity) / 86_400;
    assert_eq!(
        uint(&read(shop, sig("soldOn(uint64)", &[uint_argument(day)])).await),
        Ok(5)
    );
    // The next day the node fixes that day's seed.
    chain
        .request("evm_increaseTime", json!([86_400]))
        .await
        .unwrap();
    chain.request("anvil_mine", json!(["0x2"])).await.unwrap();
    chain
        .transact(&receipt, pool, arm_seed_calldata(day))
        .await
        .unwrap();
    chain.request("anvil_mine", json!(["0x6"])).await.unwrap();
    chain
        .transact(&receipt, pool, capture_seed_calldata(day))
        .await
        .unwrap();
    chain.request("anvil_mine", json!(["0x2"])).await.unwrap();
    assert!(chain.seed_state(day).await.unwrap().seed.is_some());
    // A winning slot among the books.
    let mut winner = None;
    'books: for book in &books {
        for slot in 0..1_000u64 {
            let won = read(
                pool,
                sig("wins(bytes32,uint32)", &[*book, uint_argument(slot)]),
            )
            .await;
            if won[63] == 1 {
                winner = Some((*book, u32::try_from(slot).unwrap()));
                break 'books;
            }
        }
    }
    let (book, slot) = winner.expect("a winning slot in five books");
    // The paid stamp names this unit alone; the node claims it.
    let mut holders = [[0; 32]; 10];
    holders[0] = unit;
    let (mailbox, period, envelope) = ([0x11; 32], day, b"a paid message".to_vec());
    let operation = named_operation(&mailbox, period, &swarm_digest(&holders[..1]), &envelope);
    let stamp = Stamp::sign(&domain, book, slot, operation, &key);
    let ticket = TicketClaim {
        book,
        index: slot,
        mailbox,
        period,
        envelope: alloy_primitives::keccak256(&envelope).0,
        holders,
        position: 0,
        signature: stamp.signature,
    };
    let owner = address_of_str(payer);
    let balance = || async {
        uint128(&read(usdc, sig("balanceOf(address)", &[address_argument(&owner)])).await).unwrap()
    };
    let before = balance().await;
    let index = u32::try_from(index).unwrap();
    chain
        .transact(
            &receipt,
            pool,
            claim_calldata(index, &transport, std::slice::from_ref(&ticket)),
        )
        .await
        .unwrap();
    assert_eq!(
        balance().await - before,
        100_000,
        "the prize, to the unit's owner"
    );
    let nonce = head().await;
    assert_eq!(
        chain
            .transact(&receipt, pool, claim_calldata(index, &transport, &[ticket]))
            .await,
        Err(ChainError::Reverted)
    );
    assert_eq!(head().await, nonce, "a refused claim is not sent");
    chain.request("anvil_mine", json!(["0x2"])).await.unwrap();
    assert_eq!(chain.owed(index).await, Ok(0));
}

fn address_of_str(text: &str) -> Address {
    hex::decode(text.trim_start_matches("0x"))
        .unwrap()
        .try_into()
        .unwrap()
}
