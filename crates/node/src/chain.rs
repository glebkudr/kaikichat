//! Books and grant rules read from the chain (phase 1b of
//! Docs/V1_MAILBOX_SWARM_IMPLEMENTATION.md): Ethereum JSON-RPC over HTTP to
//! the operator's own RPC choice, read `confirmations` blocks below the head.
//! Selectors and return words are pinned by `contracts/test/BookShop.t.sol`.
use agentic_mailbox_swarm::Account;
use agentic_mailbox_swarm::receipt::HolderKey;
use serde_json::{Value, json};
use std::time::Duration;

pub(crate) type Address = [u8; 20];

/// Where a node reads books and grant rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChainConfig {
    pub(crate) url: String,
    pub(crate) chain_id: u64,
    pub(crate) book_shop: Address,
    pub(crate) grant_issuer: Address,
    /// `NodeRegistry`: the units the directory lists.
    pub(crate) registry: Address,
    /// `OperatorPool`, if the node draws and claims prizes.
    pub(crate) operator_pool: Option<Address>,
    pub(crate) confirmations: u64,
}

/// The chain flags of `serve`, as given.
pub(crate) struct ChainFlags<'a> {
    pub(crate) rpc: Option<&'a str>,
    pub(crate) chain_id: Option<u64>,
    pub(crate) book_shop: Option<&'a str>,
    pub(crate) grant_issuer: Option<&'a str>,
    pub(crate) registry: Option<&'a str>,
    pub(crate) operator_pool: Option<&'a str>,
    pub(crate) confirmations: u64,
}

/// A bought book as `BookShop.books` records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BookRecord {
    pub(crate) key: Account,
    pub(crate) count: u32,
    pub(crate) valid_until: u64,
}

/// What `GrantIssuer` says about one issuer on one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GrantDay {
    pub(crate) active: bool,
    pub(crate) cap_coins: u64,
    pub(crate) book_size: u32,
    pub(crate) max_validity_days: u64,
    /// The chain's UTC day at the block read.
    pub(crate) today: u64,
}

/// What `coins buy` quotes: the shop, its chain, its immutable terms and
/// the ETH price at the rate of the read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShopTerms {
    pub(crate) address: Address,
    pub(crate) chain_id: u64,
    /// USDC units (six decimals) per book.
    pub(crate) price_usdc: u128,
    pub(crate) usdc: Address,
    /// Wei per book at the feed's rate; `None` while the rate is stale.
    pub(crate) quote: Option<u128>,
    pub(crate) book_size: u32,
    /// Seconds a book lasts from its purchase.
    pub(crate) validity: u64,
}

/// `OperatorPool`'s terms (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PoolTerms {
    pub(crate) address: Address,
    /// A slot wins when its draw is below this, big-endian.
    pub(crate) win_threshold: [u8; 32],
    /// A holder's prize in USDC units.
    pub(crate) prize_usdc: u128,
    /// Seconds a ticket lasts from its book's purchase.
    pub(crate) ticket_lifetime: u64,
}

/// A day's seed as the pool keeps it: the block it was armed for (zero if
/// never), the seed once captured, and the chain's head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SeedState {
    pub(crate) target: u64,
    pub(crate) seed: Option<[u8; 32]>,
    pub(crate) head: u64,
}

/// A registry unit's index and owner, whom the pool pays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegistryUnit {
    pub(crate) index: u32,
    pub(crate) owner: Address,
}

/// One ticket as `OperatorPool.claim` takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TicketClaim {
    pub(crate) book: [u8; 32],
    pub(crate) index: u32,
    pub(crate) mailbox: [u8; 32],
    pub(crate) period: u64,
    /// keccak256 of the envelope bytes.
    pub(crate) envelope: [u8; 32],
    pub(crate) holders: [[u8; 32]; 10],
    /// The claimant's place in `holders`.
    pub(crate) position: u8,
    /// The stamp's `r || s || v`.
    pub(crate) signature: [u8; 65],
}

/// An EIP-1559 transaction of no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Transaction {
    pub(crate) chain_id: u64,
    pub(crate) nonce: u64,
    pub(crate) max_priority_fee: u128,
    pub(crate) max_fee: u128,
    pub(crate) gas: u64,
    pub(crate) to: Address,
    pub(crate) data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ChainError {
    Transport(String),
    WrongChain {
        expected: u64,
        actual: u64,
    },
    /// The chain has no block `confirmations` below its head yet.
    NotConfirmed,
    /// The call reverted: the contract refused, the node answered.
    Reverted,
    /// The sending account cannot pay the transaction's gas.
    NoGas,
    Malformed(&'static str),
}

#[async_trait::async_trait]
pub(crate) trait Chain: Send + Sync {
    /// The book's purchase at a confirmed block; `None` if not bought by then.
    async fn book(&self, book: [u8; 32]) -> Result<Option<BookRecord>, ChainError>;
    /// The grant rules for `server` on `day` at a confirmed block.
    async fn grant_day(&self, server: Account, day: u64) -> Result<GrantDay, ChainError>;
    /// The shop's terms at a confirmed block.
    async fn shop(&self) -> Result<ShopTerms, ChainError>;
    /// The active registry units' commitments at a confirmed block, in
    /// registry order.
    async fn units(&self) -> Result<Vec<[u8; 32]>, ChainError>;
    /// Whether an operator pool is configured at all.
    fn has_pool(&self) -> bool {
        false
    }
    /// The pool's terms at a confirmed block.
    async fn pool(&self) -> Result<PoolTerms, ChainError> {
        Err(ChainError::Transport("no operator pool".into()))
    }
    /// `day`'s seed at a confirmed block, with the head.
    async fn seed_state(&self, _day: u64) -> Result<SeedState, ChainError> {
        Err(ChainError::Transport("no operator pool".into()))
    }
    /// The registry unit of `commitment`, active or leaving.
    async fn registry_unit(
        &self,
        _commitment: [u8; 32],
    ) -> Result<Option<RegistryUnit>, ChainError> {
        Err(ChainError::Transport("no operator pool".into()))
    }
    /// What the pool credited unit `unit` and has not paid yet, USDC units.
    async fn owed(&self, _unit: u32) -> Result<u128, ChainError> {
        Err(ChainError::Transport("no operator pool".into()))
    }
    /// The ETH `account` holds for gas, in wei.
    async fn gas_balance(&self, _account: Account) -> Result<u128, ChainError> {
        Err(ChainError::Transport("no operator pool".into()))
    }
    /// Whether `data` sent by `from` to `to` would go through now.
    async fn check(&self, _from: Account, _to: Address, _data: Vec<u8>) -> Result<(), ChainError> {
        Err(ChainError::Transport("no operator pool".into()))
    }
    /// Sends `data` to `to` from `key`'s account if it would go through, and
    /// waits for its receipt: the transaction's hash.
    async fn transact(
        &self,
        _key: &HolderKey,
        _to: Address,
        _data: Vec<u8>,
    ) -> Result<[u8; 32], ChainError> {
        Err(ChainError::Transport("no operator pool".into()))
    }
}

/// `books(bytes32)`.
const BOOKS: [u8; 4] = [0x0c, 0x0d, 0xee, 0x70];
/// `bookSize()`.
const BOOK_SIZE: [u8; 4] = [0x39, 0xf6, 0x58, 0x88];
/// `maxValidityDays()`.
const MAX_VALIDITY_DAYS: [u8; 4] = [0xa7, 0x91, 0x17, 0x30];
/// `issuerActiveOn(address,uint64)`.
const ISSUER_ACTIVE_ON: [u8; 4] = [0x66, 0x19, 0x05, 0x2a];
/// `capForDay(uint64)`.
const CAP_FOR_DAY: [u8; 4] = [0xff, 0x33, 0xdd, 0xf4];
/// `today()`.
const TODAY: [u8; 4] = [0xb7, 0x4e, 0x45, 0x2b];
/// `priceUsdc()`.
const PRICE_USDC: [u8; 4] = [0x34, 0x18, 0x9d, 0x5e];
/// `usdc()`.
const USDC: [u8; 4] = [0x3e, 0x41, 0x3b, 0xee];
/// `quote()`.
const QUOTE: [u8; 4] = [0x99, 0x9b, 0x93, 0xaf];
/// `validity()`.
const VALIDITY: [u8; 4] = [0x3e, 0x98, 0xd1, 0xfb];
/// `buy(address,bytes32)`.
const BUY: [u8; 4] = [0x90, 0x58, 0xe2, 0x28];
/// `buyWithUsdc(address,bytes32)`.
const BUY_WITH_USDC: [u8; 4] = [0xa1, 0x05, 0x72, 0xfd];
/// ERC-20 `approve(address,uint256)`.
const APPROVE: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
/// `activeUnits(uint32,uint32)`.
const ACTIVE_UNITS: [u8; 4] = [0x5d, 0xab, 0x6b, 0x66];
/// `OperatorPool.seeds(uint64)`: (target block, seed).
const SEEDS: [u8; 4] = [0xc2, 0xf1, 0xc3, 0x07];
/// `winThreshold()`.
const WIN_THRESHOLD: [u8; 4] = [0xed, 0x96, 0xcc, 0x51];
/// `prizeUsdc()`.
const PRIZE_USDC: [u8; 4] = [0xf6, 0xa1, 0x86, 0x64];
/// `ticketLifetime()`.
const TICKET_LIFETIME: [u8; 4] = [0x5b, 0xbb, 0x8b, 0x82];
/// `owed(uint32)`.
const OWED: [u8; 4] = [0x23, 0x91, 0x18, 0xce];
/// `claim(uint32,bytes32,(bytes32,uint32,bytes32,uint64,bytes32,bytes32[10],uint8,uint8,bytes32,bytes32)[])`.
const CLAIM: [u8; 4] = [0xe4, 0x05, 0x93, 0xb5];
/// `payOut(uint32,bytes32)`.
const PAY_OUT: [u8; 4] = [0xf6, 0xa0, 0x81, 0x39];
/// `armSeed(uint64)`.
const ARM_SEED: [u8; 4] = [0xc9, 0xbc, 0x7a, 0x8c];
/// `captureSeed(uint64)`.
const CAPTURE_SEED: [u8; 4] = [0x06, 0x28, 0x82, 0x9b];
/// `NodeRegistry.unit(uint32)`: owner, commitment, exit, withdraw, state.
const UNIT: [u8; 4] = [0xa4, 0x3e, 0xc9, 0x6e];
/// `NodeRegistry.nextIndex()`.
const NEXT_INDEX: [u8; 4] = [0xfc, 0x7e, 0x9c, 0x6f];
/// How long a sent transaction's receipt is awaited.
const RECEIPT_POLLS: u32 = 60;
const RECEIPT_EVERY: Duration = Duration::from_secs(2);
/// Registry positions scanned per `activeUnits` call.
const UNITS_PAGE: u64 = 256;
/// Positions scanned at most (a registry this large is not a V1 testnet).
const MAX_POSITIONS: u64 = 1 << 16;

/// Reads over JSON-RPC.
pub(crate) struct Rpc {
    config: ChainConfig,
    http: reqwest::Client,
    /// Set once `eth_chainId` matched; a failed check is made again.
    checked: tokio::sync::OnceCell<()>,
}

impl Rpc {
    pub(crate) fn new(config: ChainConfig) -> Result<Self, ChainError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| ChainError::Transport(error.to_string()))?;
        Ok(Self {
            config,
            http,
            checked: tokio::sync::OnceCell::new(),
        })
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value, ChainError> {
        let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
        let reply: Value = self
            .http
            .post(&self.config.url)
            .json(&body)
            .send()
            .await
            .map_err(|error| ChainError::Transport(error.to_string()))?
            .json()
            .await
            .map_err(|error| ChainError::Transport(error.to_string()))?;
        if let Some(error) = reply.get("error") {
            // Nodes answer a revert with code 3 (geth, anvil) or with its
            // message (older ones).
            let reverted = error["code"] == 3
                || error["message"]
                    .as_str()
                    .is_some_and(|m| m.starts_with("execution reverted"));
            return Err(if reverted {
                ChainError::Reverted
            } else {
                ChainError::Transport(error.to_string())
            });
        }
        reply
            .get("result")
            .cloned()
            .ok_or(ChainError::Malformed("no result"))
    }

    async fn quantity(&self, method: &str) -> Result<u64, ChainError> {
        let value = self.request(method, json!([])).await?;
        let text = value.as_str().ok_or(ChainError::Malformed("quantity"))?;
        u64::from_str_radix(text.trim_start_matches("0x"), 16)
            .map_err(|_| ChainError::Malformed("quantity"))
    }

    /// The confirmed block of the current head, after the chain matched.
    async fn confirmed(&self) -> Result<u64, ChainError> {
        self.checked
            .get_or_try_init(|| async {
                let actual = self.quantity("eth_chainId").await?;
                if actual == self.config.chain_id {
                    Ok(())
                } else {
                    Err(ChainError::WrongChain {
                        expected: self.config.chain_id,
                        actual,
                    })
                }
            })
            .await?;
        self.quantity("eth_blockNumber")
            .await?
            .checked_sub(self.config.confirmations)
            .ok_or(ChainError::NotConfirmed)
    }

    async fn call(&self, to: &Address, data: &[u8], block: u64) -> Result<Vec<u8>, ChainError> {
        let value = self
            .request(
                "eth_call",
                json!([
                    {"to": format!("0x{}", hex::encode(to)), "data": format!("0x{}", hex::encode(data))},
                    format!("0x{block:x}"),
                ]),
            )
            .await?;
        let text = value.as_str().ok_or(ChainError::Malformed("call result"))?;
        let bytes =
            hex::decode(text.trim_start_matches("0x")).map_err(|_| ChainError::Malformed("hex"))?;
        if bytes.is_empty() {
            // No contract at the address answers nothing.
            return Err(ChainError::Malformed("empty result"));
        }
        Ok(bytes)
    }

    async fn pool_word(
        &self,
        selector: [u8; 4],
        arguments: &[[u8; 32]],
        block: u64,
    ) -> Result<Vec<u8>, ChainError> {
        let pool = self
            .config
            .operator_pool
            .ok_or_else(|| ChainError::Transport("no operator pool".into()))?;
        self.call(&pool, &calldata(selector, arguments), block)
            .await
    }

    /// A quantity answered by `method` with `params`.
    async fn quantity_with(&self, method: &str, params: Value) -> Result<u128, ChainError> {
        quantity(&self.request(method, params).await?)
    }

    async fn word(&self, data: &[u8], block: u64) -> Result<u64, ChainError> {
        uint(&self.call(&self.config.grant_issuer, data, block).await?)
    }

    async fn shop_word(&self, selector: [u8; 4], block: u64) -> Result<Vec<u8>, ChainError> {
        self.call(&self.config.book_shop, &calldata(selector, &[]), block)
            .await
    }
}

/// A JSON-RPC quantity (`0x…`) of at most 128 bits.
fn quantity(value: &Value) -> Result<u128, ChainError> {
    let text = value.as_str().ok_or(ChainError::Malformed("quantity"))?;
    u128::from_str_radix(text.trim_start_matches("0x"), 16)
        .map_err(|_| ChainError::Malformed("quantity"))
}

fn hex0x(bytes: impl AsRef<[u8]>) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn word32(words: &[u8], at: usize) -> Result<[u8; 32], ChainError> {
    words
        .get(at * 32..at * 32 + 32)
        .and_then(|word| word.try_into().ok())
        .ok_or(ChainError::Malformed("words"))
}

/// A return value of 32 bytes read as an unsigned integer of at most 64 bits.
fn uint(word: &[u8]) -> Result<u64, ChainError> {
    let word: &[u8; 32] = word.try_into().map_err(|_| ChainError::Malformed("word"))?;
    if word[..24].iter().any(|b| *b != 0) {
        return Err(ChainError::Malformed("wide word"));
    }
    let mut low = [0; 8];
    low.copy_from_slice(&word[24..]);
    Ok(u64::from_be_bytes(low))
}

/// A return value of 32 bytes read as an unsigned integer of at most 128 bits.
fn uint128(word: &[u8]) -> Result<u128, ChainError> {
    let word: &[u8; 32] = word.try_into().map_err(|_| ChainError::Malformed("word"))?;
    if word[..16].iter().any(|b| *b != 0) {
        return Err(ChainError::Malformed("beyond 128 bits"));
    }
    let mut low = [0; 16];
    low.copy_from_slice(&word[16..]);
    Ok(u128::from_be_bytes(low))
}

/// Calldata: a selector and its 32-byte arguments.
fn calldata(selector: [u8; 4], arguments: &[[u8; 32]]) -> Vec<u8> {
    let mut data = selector.to_vec();
    for argument in arguments {
        data.extend_from_slice(argument);
    }
    data
}

fn uint_argument(value: u64) -> [u8; 32] {
    let mut word = [0; 32];
    word[24..].copy_from_slice(&value.to_be_bytes());
    word
}

fn address_argument(address: &Address) -> [u8; 32] {
    let mut word = [0; 32];
    word[12..].copy_from_slice(address);
    word
}

/// `(bytes32[] commitments, uint64 next)` as `activeUnits` returns them.
fn decode_units(words: &[u8]) -> Result<(Vec<[u8; 32]>, u64), ChainError> {
    let word = |at: usize| -> Result<u64, ChainError> {
        words
            .get(at * 32..at * 32 + 32)
            .ok_or(ChainError::Malformed("units words"))
            .and_then(uint)
    };
    if word(0)? != 0x40 {
        return Err(ChainError::Malformed("units offset"));
    }
    let next = word(1)?;
    let count = usize::try_from(word(2)?).map_err(|_| ChainError::Malformed("units count"))?;
    if words.len() != 96 + count * 32 {
        return Err(ChainError::Malformed("units length"));
    }
    let units = words[96..]
        .chunks_exact(32)
        .map(|unit| {
            let mut commitment = [0; 32];
            commitment.copy_from_slice(unit);
            commitment
        })
        .collect();
    Ok((units, next))
}

/// The words `BookShop.books` returns; a zero key is a book nobody bought.
fn decode_book(words: &[u8]) -> Result<Option<BookRecord>, ChainError> {
    if words.len() != 96 {
        return Err(ChainError::Malformed("book words"));
    }
    if words[..12].iter().any(|b| *b != 0) {
        return Err(ChainError::Malformed("key word"));
    }
    let mut key = [0; 20];
    key.copy_from_slice(&words[12..32]);
    let count = u32::try_from(uint(&words[32..64])?).map_err(|_| ChainError::Malformed("count"))?;
    let valid_until = uint(&words[64..96])?;
    if key == [0; 20] {
        return Ok(None);
    }
    Ok(Some(BookRecord {
        key,
        count,
        valid_until,
    }))
}

#[async_trait::async_trait]
impl Chain for Rpc {
    async fn book(&self, book: [u8; 32]) -> Result<Option<BookRecord>, ChainError> {
        let block = self.confirmed().await?;
        let words = self
            .call(&self.config.book_shop, &calldata(BOOKS, &[book]), block)
            .await?;
        decode_book(&words)
    }

    async fn grant_day(&self, server: Account, day: u64) -> Result<GrantDay, ChainError> {
        let block = self.confirmed().await?;
        let book_size = self.word(&calldata(BOOK_SIZE, &[]), block).await?;
        let max_validity_days = self.word(&calldata(MAX_VALIDITY_DAYS, &[]), block).await?;
        let today = self.word(&calldata(TODAY, &[]), block).await?;
        let active = self
            .word(
                &calldata(
                    ISSUER_ACTIVE_ON,
                    &[address_argument(&server), uint_argument(day)],
                ),
                block,
            )
            .await?;
        let cap_coins = self
            .word(&calldata(CAP_FOR_DAY, &[uint_argument(day)]), block)
            .await?;
        Ok(GrantDay {
            active: match active {
                0 => false,
                1 => true,
                _ => return Err(ChainError::Malformed("bool")),
            },
            cap_coins,
            book_size: u32::try_from(book_size).map_err(|_| ChainError::Malformed("book size"))?,
            max_validity_days,
            today,
        })
    }

    async fn shop(&self) -> Result<ShopTerms, ChainError> {
        let block = self.confirmed().await?;
        let usdc = self.shop_word(USDC, block).await?;
        let usdc: &[u8; 32] = usdc
            .as_slice()
            .try_into()
            .map_err(|_| ChainError::Malformed("word"))?;
        if usdc[..12].iter().any(|b| *b != 0) {
            return Err(ChainError::Malformed("address"));
        }
        let quote = match self.shop_word(QUOTE, block).await {
            Ok(word) => Some(uint128(&word)?),
            // Without a fresh rate only USDC buys.
            Err(ChainError::Reverted) => None,
            Err(error) => return Err(error),
        };
        let book_size = uint(&self.shop_word(BOOK_SIZE, block).await?)?;
        Ok(ShopTerms {
            address: self.config.book_shop,
            chain_id: self.config.chain_id,
            price_usdc: uint128(&self.shop_word(PRICE_USDC, block).await?)?,
            usdc: usdc[12..]
                .try_into()
                .map_err(|_| ChainError::Malformed("address"))?,
            quote,
            book_size: u32::try_from(book_size).map_err(|_| ChainError::Malformed("book size"))?,
            validity: uint(&self.shop_word(VALIDITY, block).await?)?,
        })
    }

    async fn units(&self) -> Result<Vec<[u8; 32]>, ChainError> {
        let block = self.confirmed().await?;
        let mut units = Vec::new();
        let mut start = 0u64;
        loop {
            let data = calldata(
                ACTIVE_UNITS,
                &[uint_argument(start), uint_argument(UNITS_PAGE)],
            );
            let words = self.call(&self.config.registry, &data, block).await?;
            let (page, next) = decode_units(&words)?;
            units.extend(page);
            // Short pages are fine: exited units are skipped. The page
            // ends the registry once `next` stops short of the scan.
            if next < start + UNITS_PAGE || next >= MAX_POSITIONS {
                return Ok(units);
            }
            start = next;
        }
    }

    fn has_pool(&self) -> bool {
        self.config.operator_pool.is_some()
    }

    async fn pool(&self) -> Result<PoolTerms, ChainError> {
        let block = self.confirmed().await?;
        let address = self
            .config
            .operator_pool
            .ok_or_else(|| ChainError::Transport("no operator pool".into()))?;
        let win_threshold = word32(&self.pool_word(WIN_THRESHOLD, &[], block).await?, 0)?;
        let prize_usdc = uint128(&self.pool_word(PRIZE_USDC, &[], block).await?)?;
        let ticket_lifetime = uint(&self.pool_word(TICKET_LIFETIME, &[], block).await?)?;
        Ok(PoolTerms {
            address,
            win_threshold,
            prize_usdc,
            ticket_lifetime,
        })
    }

    async fn seed_state(&self, day: u64) -> Result<SeedState, ChainError> {
        let block = self.confirmed().await?;
        let words = self.pool_word(SEEDS, &[uint_argument(day)], block).await?;
        if words.len() != 64 {
            return Err(ChainError::Malformed("seed words"));
        }
        let seed = word32(&words, 1)?;
        Ok(SeedState {
            target: uint(&words[..32])?,
            seed: (seed != [0; 32]).then_some(seed),
            head: block + self.config.confirmations,
        })
    }

    async fn registry_unit(
        &self,
        commitment: [u8; 32],
    ) -> Result<Option<RegistryUnit>, ChainError> {
        let block = self.confirmed().await?;
        let registry = self.config.registry;
        let next = uint(
            &self
                .call(&registry, &calldata(NEXT_INDEX, &[]), block)
                .await?,
        )?;
        for index in 0..next.min(MAX_POSITIONS) {
            let words = self
                .call(&registry, &calldata(UNIT, &[uint_argument(index)]), block)
                .await?;
            if words.len() != 160 {
                return Err(ChainError::Malformed("unit words"));
            }
            // State `None` (0) is a position never bonded.
            if word32(&words, 1)? == commitment && uint(&words[128..])? != 0 {
                return Ok(Some(RegistryUnit {
                    index: u32::try_from(index).map_err(|_| ChainError::Malformed("index"))?,
                    owner: words[12..32]
                        .try_into()
                        .map_err(|_| ChainError::Malformed("owner"))?,
                }));
            }
        }
        Ok(None)
    }

    async fn owed(&self, unit: u32) -> Result<u128, ChainError> {
        let block = self.confirmed().await?;
        uint128(
            &self
                .pool_word(OWED, &[uint_argument(u64::from(unit))], block)
                .await?,
        )
    }

    async fn gas_balance(&self, account: Account) -> Result<u128, ChainError> {
        self.confirmed().await?;
        self.quantity_with("eth_getBalance", json!([hex0x(account), "latest"]))
            .await
    }

    async fn check(&self, from: Account, to: Address, data: Vec<u8>) -> Result<(), ChainError> {
        self.confirmed().await?;
        self.quantity_with(
            "eth_estimateGas",
            json!([{"from": hex0x(from), "to": hex0x(to), "data": hex0x(&data)}]),
        )
        .await
        .map(|_| ())
    }

    /// A fifth more gas than the estimate, a fee cap of twice the base fee
    /// plus the tip, the pending nonce; nothing is sent the account cannot
    /// pay for.
    async fn transact(
        &self,
        key: &HolderKey,
        to: Address,
        data: Vec<u8>,
    ) -> Result<[u8; 32], ChainError> {
        self.confirmed().await?;
        let from = hex0x(key.account());
        let estimate = self
            .quantity_with(
                "eth_estimateGas",
                json!([{"from": from, "to": hex0x(to), "data": hex0x(&data)}]),
            )
            .await?;
        let gas =
            u64::try_from(estimate + estimate / 5).map_err(|_| ChainError::Malformed("gas"))?;
        let nonce = self
            .quantity_with("eth_getTransactionCount", json!([from, "pending"]))
            .await?;
        let tip = self
            .quantity_with("eth_maxPriorityFeePerGas", json!([]))
            .await?;
        let latest = self
            .request("eth_getBlockByNumber", json!(["latest", false]))
            .await?;
        let base = quantity(&latest["baseFeePerGas"])?;
        let max_fee = base * 2 + tip;
        let balance = self
            .quantity_with("eth_getBalance", json!([from, "latest"]))
            .await?;
        if balance < u128::from(gas) * max_fee {
            return Err(ChainError::NoGas);
        }
        let raw = signed_transaction(
            &Transaction {
                chain_id: self.config.chain_id,
                nonce: u64::try_from(nonce).map_err(|_| ChainError::Malformed("nonce"))?,
                max_priority_fee: tip,
                max_fee,
                gas,
                to,
                data,
            },
            key,
        );
        let sent = self
            .request("eth_sendRawTransaction", json!([hex0x(raw)]))
            .await?;
        let hash: [u8; 32] = sent
            .as_str()
            .and_then(|text| hex::decode(text.trim_start_matches("0x")).ok())
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(ChainError::Malformed("transaction hash"))?;
        for _ in 0..RECEIPT_POLLS {
            let receipt = self
                .request("eth_getTransactionReceipt", json!([hex0x(hash)]))
                .await?;
            if !receipt.is_null() {
                return if receipt["status"] == "0x1" {
                    Ok(hash)
                } else {
                    Err(ChainError::Reverted)
                };
            }
            tokio::time::sleep(RECEIPT_EVERY).await;
        }
        Err(ChainError::Transport("no receipt yet".into()))
    }
}

/// The calldata of `OperatorPool.claim(unit, transportKey, tickets)`.
pub(crate) fn claim_calldata(unit: u32, transport: &[u8; 32], tickets: &[TicketClaim]) -> Vec<u8> {
    let count = u64::try_from(tickets.len()).unwrap_or(u64::MAX);
    let mut data = calldata(
        CLAIM,
        &[
            uint_argument(u64::from(unit)),
            *transport,
            uint_argument(0x60),
            uint_argument(count),
        ],
    );
    for ticket in tickets {
        let mut r = [0; 32];
        let mut s = [0; 32];
        r.copy_from_slice(&ticket.signature[..32]);
        s.copy_from_slice(&ticket.signature[32..64]);
        let mut words = vec![
            ticket.book,
            uint_argument(u64::from(ticket.index)),
            ticket.mailbox,
            uint_argument(ticket.period),
            ticket.envelope,
        ];
        words.extend(ticket.holders);
        words.extend([
            uint_argument(u64::from(ticket.position)),
            uint_argument(u64::from(ticket.signature[64])),
            r,
            s,
        ]);
        for word in words {
            data.extend_from_slice(&word);
        }
    }
    data
}

/// The calldata of `OperatorPool.armSeed(day)`.
pub(crate) fn arm_seed_calldata(day: u64) -> Vec<u8> {
    calldata(ARM_SEED, &[uint_argument(day)])
}

/// The calldata of `OperatorPool.captureSeed(day)`.
pub(crate) fn capture_seed_calldata(day: u64) -> Vec<u8> {
    calldata(CAPTURE_SEED, &[uint_argument(day)])
}

/// The calldata of `OperatorPool.payOut(unit, transportKey)`.
pub(crate) fn pay_out_calldata(unit: u32, transport: &[u8; 32]) -> Vec<u8> {
    calldata(PAY_OUT, &[uint_argument(u64::from(unit)), *transport])
}

/// RLP of a byte string.
fn rlp_bytes(bytes: &[u8]) -> Vec<u8> {
    match bytes {
        [byte] if *byte < 0x80 => vec![*byte],
        _ => [rlp_length(bytes.len(), 0x80), bytes.to_vec()].concat(),
    }
}

/// RLP of an unsigned integer: its big-endian bytes without leading zeros.
fn rlp_uint(value: u128) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|b| *b != 0).unwrap_or(bytes.len());
    rlp_bytes(&bytes[first..])
}

/// RLP of a list of encoded items.
fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload = items.concat();
    [rlp_length(payload.len(), 0xc0), payload].concat()
}

fn rlp_length(length: usize, offset: u8) -> Vec<u8> {
    if length <= 55 {
        vec![offset + u8::try_from(length).unwrap_or(0)]
    } else {
        let bytes = (length as u64).to_be_bytes();
        let first = bytes.iter().position(|b| *b != 0).unwrap_or(bytes.len());
        let long = &bytes[first..];
        [
            vec![offset + 55 + u8::try_from(long.len()).unwrap_or(0)],
            long.to_vec(),
        ]
        .concat()
    }
}

/// The raw EIP-1559 transaction `key` signs.
pub(crate) fn signed_transaction(tx: &Transaction, key: &HolderKey) -> Vec<u8> {
    let fields = vec![
        rlp_uint(u128::from(tx.chain_id)),
        rlp_uint(u128::from(tx.nonce)),
        rlp_uint(tx.max_priority_fee),
        rlp_uint(tx.max_fee),
        rlp_uint(u128::from(tx.gas)),
        rlp_bytes(&tx.to),
        rlp_uint(0),
        rlp_bytes(&tx.data),
        rlp_list(&[]),
    ];
    let hash = alloy_primitives::keccak256([&[0x02][..], &rlp_list(&fields)].concat()).0;
    let signature = key.sign_transaction(&hash);
    let mut signed = fields;
    signed.push(rlp_uint(u128::from(signature[64].saturating_sub(27))));
    signed.push(rlp_word(&signature[..32]));
    signed.push(rlp_word(&signature[32..64]));
    [&[0x02][..], &rlp_list(&signed)].concat()
}

/// RLP of a 256-bit integer given big-endian.
fn rlp_word(word: &[u8]) -> Vec<u8> {
    let first = word.iter().position(|b| *b != 0).unwrap_or(word.len());
    rlp_bytes(&word[first..])
}

/// The calldata of `BookShop.buy(key, salt)` a wallet sends with ETH.
pub(crate) fn buy_calldata(key: &Account, salt: &[u8; 32]) -> Vec<u8> {
    calldata(BUY, &[address_argument(key), *salt])
}

/// The calldata of `BookShop.buyWithUsdc(key, salt)`.
pub(crate) fn buy_with_usdc_calldata(key: &Account, salt: &[u8; 32]) -> Vec<u8> {
    calldata(BUY_WITH_USDC, &[address_argument(key), *salt])
}

/// The calldata of the token's `approve(spender, amount)`.
pub(crate) fn approve_calldata(spender: &Address, amount: u128) -> Vec<u8> {
    let mut word = [0; 32];
    word[16..].copy_from_slice(&amount.to_be_bytes());
    calldata(APPROVE, &[address_argument(spender), word])
}

/// The chain `serve` reads from: every flag or none. Addresses are 0x-hex.
pub(crate) fn chain_config(flags: &ChainFlags<'_>) -> Result<Option<ChainConfig>, String> {
    let address = |text: &str| -> Result<Address, String> {
        hex::decode(text.trim_start_matches("0x"))
            .ok()
            .and_then(|bytes| Address::try_from(bytes).ok())
            .ok_or_else(|| format!("not a 20-byte 0x address: {text}"))
    };
    match (
        flags.rpc,
        flags.chain_id,
        flags.book_shop,
        flags.grant_issuer,
        flags.registry,
    ) {
        (None, None, None, None, None) if flags.operator_pool.is_none() => Ok(None),
        (Some(url), Some(chain_id), Some(book_shop), Some(grant_issuer), Some(registry)) => {
            if flags.confirmations == 0 {
                return Err("chain confirmations must be at least 1".into());
            }
            Ok(Some(ChainConfig {
                url: url.to_owned(),
                chain_id,
                book_shop: address(book_shop)?,
                grant_issuer: address(grant_issuer)?,
                registry: address(registry)?,
                operator_pool: flags.operator_pool.map(address).transpose()?,
                confirmations: flags.confirmations,
            }))
        }
        _ => Err(
            "give --chain-rpc, --chain-id, --book-shop, --grant-issuer and --registry together"
                .into(),
        ),
    }
}

#[cfg(test)]
#[path = "chain_tests.rs"]
mod tests;
