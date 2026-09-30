//! `agentic-directory`: the discovery service (spec/discovery-v1.md).
//!
//! Configuration comes from the environment:
//! - `AIN_DIR_LISTEN` (e.g. `0.0.0.0:8081`) and `AIN_DIR_PUBLIC_URL`;
//! - `AIN_DIR_DOMAIN`: the network domain, 32 bytes hex;
//! - `AIN_DIR_SIGNING_KEY_FILE`, `AIN_DIR_PEPPER_FILE`: 32 bytes hex each;
//! - `AIN_DIR_DATABASE`: the SQLite path;
//! - `AIN_DIR_GOOGLE_CLIENT_ID`, `AIN_DIR_GOOGLE_CLIENT_SECRET_FILE`;
//! - optionally `AIN_DIR_GITHUB_CLIENT_ID` with `AIN_DIR_GITHUB_CLIENT_SECRET_FILE`;
//! - `AIN_DIR_NODE_SOCKET` and `AIN_DIR_NODE_TOKEN_FILE`: the owner IPC of
//!   the node that checks stamps and books (it runs with chain flags);
//! - `AIN_DIR_LINK_TTL_SECS` (default 900).

use agentic_directory::{Config, GitHubConfig, GoogleConfig, IpcLedger, Server};
use std::{sync::Arc, time::SystemTime};

const GOOGLE_AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN: &str = "https://oauth2.googleapis.com/token";
const GITHUB_AUTHORIZE: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN: &str = "https://github.com/login/oauth/access_token";
const GITHUB_API: &str = "https://api.github.com";

fn var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is not set"))
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
    let github = match std::env::var("AIN_DIR_GITHUB_CLIENT_ID") {
        Ok(client_id) => Some(GitHubConfig {
            client_id,
            client_secret: secret_file("AIN_DIR_GITHUB_CLIENT_SECRET_FILE")?,
            authorize_url: GITHUB_AUTHORIZE.into(),
            token_url: GITHUB_TOKEN.into(),
            api_url: GITHUB_API.into(),
        }),
        Err(_) => None,
    };
    Ok(Config {
        public_url: var("AIN_DIR_PUBLIC_URL")?,
        domain: bytes32("AIN_DIR_DOMAIN", &var("AIN_DIR_DOMAIN")?)?,
        signing_secret: bytes32(
            "AIN_DIR_SIGNING_KEY_FILE",
            &secret_file("AIN_DIR_SIGNING_KEY_FILE")?,
        )?,
        pepper: bytes32("AIN_DIR_PEPPER_FILE", &secret_file("AIN_DIR_PEPPER_FILE")?)?,
        database: var("AIN_DIR_DATABASE")?.into(),
        google: GoogleConfig {
            client_id: var("AIN_DIR_GOOGLE_CLIENT_ID")?,
            client_secret: secret_file("AIN_DIR_GOOGLE_CLIENT_SECRET_FILE")?,
            authorize_url: GOOGLE_AUTHORIZE.into(),
            token_url: GOOGLE_TOKEN.into(),
        },
        github,
        ledger: Arc::new(IpcLedger::new(
            var("AIN_DIR_NODE_SOCKET")?.into(),
            bytes32(
                "AIN_DIR_NODE_TOKEN_FILE",
                &secret_file("AIN_DIR_NODE_TOKEN_FILE")?,
            )?,
        )),
        link_ttl_secs: std::env::var("AIN_DIR_LINK_TTL_SECS")
            .ok()
            .map(|v| {
                v.parse()
                    .map_err(|_| "AIN_DIR_LINK_TTL_SECS is not a number")
            })
            .transpose()?
            .unwrap_or(900),
        clock: Arc::new(now),
    })
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let started = async {
        let config = config()?;
        let listen = var("AIN_DIR_LISTEN")?;
        let listener = tokio::net::TcpListener::bind(&listen)
            .await
            .map_err(|error| format!("{listen}: {error}"))?;
        let server = Server::start(config, listener)
            .await
            .map_err(|error| error.to_string())?;
        eprintln!("directory listening on {}", server.addr);
        tokio::signal::ctrl_c()
            .await
            .map_err(|error| error.to_string())?;
        server.shutdown().await;
        Ok::<(), String>(())
    };
    match started.await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("agentic-directory: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
