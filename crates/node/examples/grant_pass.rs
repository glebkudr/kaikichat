//! A directory search that pays with a grant signed by an issuer's key, as a
//! thief holding the identity server's hot key would make one (V1-AF05).
//!
//! `grant_pass KEY_FILE SERIAL BOOK_SEED` reads the issuer's secp256k1 key
//! (hex) from `KEY_FILE`, grants the book whose key is `BOOK_SEED` (32 bytes of
//! hex) 10000 coins with `SERIAL` for today and prints the body of
//! `POST /v1/search` whose pass that book signs. The network takes such a
//! grant only while `(SERIAL + 1) × 10000` is within the `GrantIssuer` day's
//! cap. The key is never printed.
//!
//! The same arguments give the same grant all day (the signature is
//! deterministic): two different grants of one serial and day would prove the
//! issuer dishonest and end it the next day.
use agentic_grant_book::{GrantBook, GrantTerms, SecpKey};
use agentic_mailbox_swarm::access::AccessPass;
use agentic_mailbox_swarm::stamp::BookKey;
use std::time::{SystemTime, UNIX_EPOCH};

const DAY: u64 = 86_400;
/// The grant book of the testnet's `GrantIssuer` (deployments/base-sepolia.json).
const BOOK_SIZE: u32 = 10_000;

fn random() -> agentic_node::Result<[u8; 32]> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn main() -> agentic_node::Result<()> {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(serial), Some(seed)) = (args.next(), args.next(), args.next()) else {
        return Err("usage: grant_pass KEY_FILE SERIAL BOOK_SEED".into());
    };
    let serial: u32 = serial.parse()?;
    let text = std::fs::read_to_string(&path)?;
    let secret: [u8; 32] = hex::decode(text.trim().trim_start_matches("0x"))?
        .try_into()
        .map_err(|_| "the key file holds 32 bytes of hex")?;
    let issuer = SecpKey::from_secret(&secret).ok_or("not a secp256k1 key")?;
    let seed: [u8; 32] = hex::decode(seed)?
        .try_into()
        .map_err(|_| "BOOK_SEED is 32 bytes of hex")?;
    let book = BookKey::from_bytes(&seed).ok_or("not a book key")?;
    let day = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() / DAY;
    let grant = GrantBook::issue(
        GrantTerms {
            domain: agentic_node::NETWORK_DOMAIN,
            book: book.account(),
            day,
            serial,
            count: BOOK_SIZE,
            expiry: (day + 30) * DAY,
        },
        &issuer,
    );
    let pass = AccessPass::sign(
        &agentic_node::NETWORK_DOMAIN,
        grant.id(),
        random()?,
        day,
        &book,
    );
    let body = serde_json::json!({
        "query": "lobby",
        "pass": {
            "book": hex::encode(pass.book),
            "peer": hex::encode(pass.peer),
            "day": pass.day,
            "signature": hex::encode(pass.signature),
            "grant": grant,
        },
    });
    println!("{body}");
    Ok(())
}
