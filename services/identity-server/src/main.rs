//! `agentic-identity-server`: grants coins to Google-verified accounts.
//!
//! Configuration comes from the environment:
//! - `AIN_ID_LISTEN` (e.g. `0.0.0.0:8080`) and `AIN_ID_PUBLIC_URL`;
//! - `AIN_ID_DOMAIN`: the network domain, 32 bytes hex;
//! - `AIN_ID_SIGNING_KEY_FILE`: a file holding the issuer's secp256k1 secret as hex;
//! - `AIN_ID_DATABASE`: the SQLite path;
//! - `AIN_ID_GOOGLE_CLIENT_ID`, `AIN_ID_GOOGLE_CLIENT_SECRET_FILE`;
//! - optionally `AIN_ID_GITHUB_CLIENT_ID` with `AIN_ID_GITHUB_CLIENT_SECRET_FILE`
//!   to offer GitHub besides Google;
//! - `AIN_ID_CAP_COINS`, `AIN_ID_BOOK_SIZE`, `AIN_ID_MAX_VALIDITY_DAYS`: the
//!   network's GrantIssuer rules, mirrored until the server reads the contract;
//! - `AIN_ID_GRANT_VALIDITY_DAYS` (default 30), `AIN_ID_CLAIM_INTERVAL_DAYS`
//!   (default 30), `AIN_ID_CLAIM_TTL_SECS` (default 900).

use agentic_identity_server::{Config, GitHubConfig, GoogleConfig, Server, StaticRules};
use std::{collections::BTreeMap, sync::Arc, time::SystemTime};

const GOOGLE_AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN: &str = "https://oauth2.googleapis.com/token";
const GITHUB_AUTHORIZE: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN: &str = "https://github.com/login/oauth/access_token";
const GITHUB_API: &str = "https://api.github.com";

fn var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is not set"))
}

fn number(name: &str, default: Option<u64>) -> Result<u64, String> {
    match (std::env::var(name), default) {
        (Ok(value), _) => value.parse().map_err(|_| format!("{name} is not a number")),
        (Err(_), Some(default)) => Ok(default),
        (Err(_), None) => Err(format!("{name} is not set")),
    }
}

fn secret_file(name: &str) -> Result<String, String> {
    let path = var(name)?;
    std::fs::read_to_string(&path)
        .map(|text| text.trim().to_owned())
        .map_err(|error| format!("{name}: {error}"))
}

fn bytes32(name: &str, text: &str) -> Result<[u8; 32], String> {
    hex::decode(text.trim_start_matches("0x"))
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| format!("{name} is not 32 bytes of hex"))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn config() -> Result<Config, String> {
    let domain = bytes32("AIN_ID_DOMAIN", &var("AIN_ID_DOMAIN")?)?;
    let book_size = u32::try_from(number("AIN_ID_BOOK_SIZE", None)?)
        .map_err(|_| "AIN_ID_BOOK_SIZE is too large".to_owned())?;
    Ok(Config {
        public_url: var("AIN_ID_PUBLIC_URL")?,
        domain,
        signing_secret: bytes32(
            "AIN_ID_SIGNING_KEY_FILE",
            &secret_file("AIN_ID_SIGNING_KEY_FILE")?,
        )?,
        database: var("AIN_ID_DATABASE")?.into(),
        google: GoogleConfig {
            client_id: var("AIN_ID_GOOGLE_CLIENT_ID")?,
            client_secret: secret_file("AIN_ID_GOOGLE_CLIENT_SECRET_FILE")?,
            authorize_url: GOOGLE_AUTHORIZE.into(),
            token_url: GOOGLE_TOKEN.into(),
        },
        github: match std::env::var("AIN_ID_GITHUB_CLIENT_ID") {
            Ok(client_id) => Some(GitHubConfig {
                client_id,
                client_secret: secret_file("AIN_ID_GITHUB_CLIENT_SECRET_FILE")?,
                authorize_url: GITHUB_AUTHORIZE.into(),
                token_url: GITHUB_TOKEN.into(),
                api_url: GITHUB_API.into(),
            }),
            Err(_) => None,
        },
        rules: Arc::new(StaticRules {
            domain,
            caps: BTreeMap::from([(0, number("AIN_ID_CAP_COINS", None)?)]),
            book_size,
            max_validity_days: number("AIN_ID_MAX_VALIDITY_DAYS", None)?,
        }),
        grant_validity_days: number("AIN_ID_GRANT_VALIDITY_DAYS", Some(30))?,
        claim_interval_days: number("AIN_ID_CLAIM_INTERVAL_DAYS", Some(30))?,
        claim_ttl_secs: number("AIN_ID_CLAIM_TTL_SECS", Some(900))?,
        clock: Arc::new(now),
    })
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let started = async {
        let config = config()?;
        let listen = var("AIN_ID_LISTEN")?;
        let listener = tokio::net::TcpListener::bind(&listen)
            .await
            .map_err(|error| format!("{listen}: {error}"))?;
        let server = Server::start(config, listener)
            .await
            .map_err(|error| error.to_string())?;
        eprintln!("identity server listening on {}", server.addr);
        tokio::signal::ctrl_c()
            .await
            .map_err(|error| error.to_string())?;
        server.shutdown().await;
        Ok::<(), String>(())
    };
    match started.await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("agentic-identity-server: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
