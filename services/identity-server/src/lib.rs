//! Identity server: grants coins to accounts verified by Google or GitHub.
//!
//! It only mints. Each grant is a [`GrantBook`] for the claimant's stamp-book
//! account, numbered per UTC day below the network's daily emission cap. The
//! server never takes part in sending or bootstrap. An account proven to
//! spend a grant twice is banned and its grants are revoked
//! (Docs/V1_IDENTITY_PENALTIES_2026_09_30.md).

use agentic_grant_book::{
    Account, GrantRevocation, GrantRules, GrantTerms, SECONDS_PER_DAY, SecpKey,
};
use agentic_mailbox_swarm::proof::SenderEquivocation;
use agentic_mailbox_swarm::stamp::Stamp;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    future::Future,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
};
use url::Url;

pub use agentic_grant_book::{ClaimRequest, GrantBook};

/// Unix seconds; injected so tests can move across UTC days.
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

pub type RulesFuture<'a> = Pin<Box<dyn Future<Output = Result<GrantRules, String>> + Send + 'a>>;

/// Network rules for grants of `server` on `day`, as holders will check them.
pub trait NetworkRules: Send + Sync {
    fn rules(&self, server: Account, day: u64) -> RulesFuture<'_>;
}

/// Rules fixed at start-up: a daily cap schedule (the latest entry at or
/// before a day applies), a book size and a maximum grant validity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticRules {
    pub domain: [u8; 32],
    pub caps: BTreeMap<u64, u64>,
    pub book_size: u32,
    pub max_validity_days: u64,
}

impl NetworkRules for StaticRules {
    fn rules(&self, _server: Account, day: u64) -> RulesFuture<'_> {
        let cap_coins = self
            .caps
            .range(..=day)
            .next_back()
            .map_or(0, |(_, cap)| *cap);
        let rules = GrantRules {
            domain: self.domain,
            issuer_active: true,
            cap_coins,
            book_size: self.book_size,
            max_validity_days: self.max_validity_days,
        };
        Box::pin(async move { Ok(rules) })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
}

/// GitHub's OAuth app: its answers carry no signature, so the token and API
/// endpoints must be TLS (or loopback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
    /// The REST API base (`https://api.github.com`).
    pub api_url: String,
}

pub struct Config {
    /// Base URL browsers use to reach this server, without a trailing slash.
    pub public_url: String,
    pub domain: [u8; 32],
    pub signing_secret: [u8; 32],
    pub database: PathBuf,
    pub google: GoogleConfig,
    /// GitHub besides Google, when configured.
    pub github: Option<GitHubConfig>,
    pub rules: Arc<dyn NetworkRules>,
    /// Grant lifetime; never beyond the network's maximum validity.
    pub grant_validity_days: u64,
    /// One grant per account per this many days.
    pub claim_interval_days: u64,
    /// How long a claim request and its login link stay usable.
    pub claim_ttl_secs: u64,
    pub clock: Clock,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid configuration: {0}")]
    Config(&'static str),
    #[error("storage: {0}")]
    Storage(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

/// A running server; dropping it does not stop it, [`Server::shutdown`] does.
pub struct Server {
    pub addr: SocketAddr,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub async fn start(config: Config, listener: tokio::net::TcpListener) -> Result<Self, Error> {
        let app = Arc::new(App::new(config)?);
        let addr = listener.local_addr()?;
        let router = Router::new()
            .route("/v1/policy", get(policy))
            .route("/v1/claims", post(create_claim))
            .route("/v1/claims/{id}", get(claim_status))
            .route("/v1/claims/{id}/login", get(login_page))
            .route("/v1/claims/{id}/login/google", get(login))
            .route("/v1/claims/{id}/login/github", get(github_login))
            .route("/v1/oauth/google/callback", get(callback))
            .route("/v1/oauth/github/callback", get(github_callback))
            .route("/v1/reports", post(report))
            .route("/v1/revocations", get(revocations))
            .with_state(app);
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let shutdown = async {
                let _ = stopped.await;
            };
            if let Err(error) = axum::serve(listener, router)
                .with_graceful_shutdown(shutdown)
                .await
            {
                eprintln!("identity server stopped: {error}");
            }
        });
        Ok(Self { addr, stop, task })
    }

    pub async fn shutdown(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}

const GOOGLE_ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];
/// Claim requests may be dated this far ahead of the server clock.
const CLOCK_SKEW_SECS: u64 = 60;
/// Most revocations in one page.
const REVOCATION_PAGE: u64 = 256;
/// How long an account's first proven double spend keeps it from grants;
/// a grant got after that spent twice bans it for good.
const FIRST_BAN_SECS: u64 = 90 * SECONDS_PER_DAY;

struct App {
    public_url: String,
    domain: [u8; 32],
    key: SecpKey,
    server: Account,
    google: GoogleConfig,
    github: Option<GitHubConfig>,
    rules: Arc<dyn NetworkRules>,
    grant_validity_days: u64,
    claim_interval_days: u64,
    claim_ttl_secs: u64,
    clock: Clock,
    db: Mutex<Connection>,
    http: reqwest::Client,
}

impl App {
    fn new(config: Config) -> Result<Self, Error> {
        let token = Url::parse(&config.google.token_url)
            .map_err(|_| Error::Config("google token URL is not a URL"))?;
        if !(token.scheme() == "https" || token.scheme() == "http" && is_loopback(&token)) {
            return Err(Error::Config(
                "the identity token must be fetched over https (or loopback)",
            ));
        }
        Url::parse(&config.google.authorize_url)
            .map_err(|_| Error::Config("google authorize URL is not a URL"))?;
        if let Some(github) = &config.github {
            for (url, what) in [
                (&github.token_url, "github token URL"),
                (&github.api_url, "github API URL"),
            ] {
                let url = Url::parse(url).map_err(|_| Error::Config("github URL is not a URL"))?;
                if !(url.scheme() == "https" || url.scheme() == "http" && is_loopback(&url)) {
                    eprintln!("{what} is not https");
                    return Err(Error::Config(
                        "GitHub's token and API must be reached over https (or loopback)",
                    ));
                }
            }
            Url::parse(&github.authorize_url)
                .map_err(|_| Error::Config("github authorize URL is not a URL"))?;
        }
        Url::parse(&config.public_url).map_err(|_| Error::Config("public URL is not a URL"))?;
        if config.grant_validity_days == 0 || config.claim_ttl_secs == 0 {
            return Err(Error::Config(
                "grant validity and claim TTL must be positive",
            ));
        }
        let key = SecpKey::from_secret(&config.signing_secret)
            .ok_or(Error::Config("signing secret is not a secp256k1 key"))?;
        let db = open(&config.database)?;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| Error::Config("cannot build an HTTP client"))?;
        Ok(Self {
            public_url: config.public_url.trim_end_matches('/').to_owned(),
            domain: config.domain,
            server: key.account(),
            key,
            google: config.google,
            github: config.github,
            rules: config.rules,
            grant_validity_days: config.grant_validity_days,
            claim_interval_days: config.claim_interval_days,
            claim_ttl_secs: config.claim_ttl_secs,
            clock: config.clock,
            db: Mutex::new(db),
            http,
        })
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    fn db(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A panic while holding the lock leaves SQLite consistent: every
        // change runs inside a transaction that is rolled back on drop.
        self.db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(name)) => name == "localhost",
        Some(url::Host::Ipv4(ip)) => IpAddr::V4(ip).is_loopback(),
        Some(url::Host::Ipv6(ip)) => IpAddr::V6(ip).is_loopback(),
        None => false,
    }
}

fn open(path: &std::path::Path) -> Result<Connection, Error> {
    let db = Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS claims(
             id TEXT PRIMARY KEY,
             book BLOB NOT NULL,
             nonce BLOB NOT NULL,
             created_at INTEGER NOT NULL,
             expires_at INTEGER NOT NULL,
             status TEXT NOT NULL,
             reason TEXT,
             grant_json TEXT,
             UNIQUE(book, nonce));
         CREATE TABLE IF NOT EXISTS logins(
             state TEXT PRIMARY KEY,
             claim_id TEXT NOT NULL REFERENCES claims(id),
             nonce TEXT NOT NULL,
             verifier TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS github_logins(
             state TEXT PRIMARY KEY,
             claim_id TEXT NOT NULL REFERENCES claims(id),
             verifier TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS accounts(
             subject_hash BLOB PRIMARY KEY,
             last_grant_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS days(
             day INTEGER PRIMARY KEY,
             issued INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS grants(
             book_id BLOB PRIMARY KEY,
             subject_hash BLOB NOT NULL,
             issued_at INTEGER NOT NULL,
             expiry INTEGER NOT NULL,
             grant_json TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS grants_subject ON grants(subject_hash);
         CREATE TABLE IF NOT EXISTS bans(
             subject_hash BLOB PRIMARY KEY,
             banned_at INTEGER NOT NULL,
             until INTEGER,
             book_id BLOB NOT NULL);
         CREATE TABLE IF NOT EXISTS revocations(
             seq INTEGER PRIMARY KEY AUTOINCREMENT,
             book_id BLOB NOT NULL UNIQUE,
             expiry INTEGER NOT NULL,
             revocation_json TEXT NOT NULL);",
    )?;
    Ok(db)
}

fn random_token() -> Result<String, Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Error::Config("no system randomness"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn hex0x(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn sql(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn unsql(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn json_error(status: StatusCode, error: &str) -> Response {
    (status, Json(json!({ "error": error }))).into_response()
}

fn page(status: StatusCode, title: &str, text: &str) -> Response {
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title></head>\
         <body><h1>{title}</h1><p>{text}</p></body></html>"
    );
    (status, Html(body)).into_response()
}

fn internal(error: impl std::fmt::Display) -> Response {
    eprintln!("identity server error: {error}");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
}

async fn policy(State(app): State<Arc<App>>) -> Response {
    let day = app.now() / SECONDS_PER_DAY;
    let rules = match app.rules.rules(app.server, day).await {
        Ok(rules) => rules,
        Err(error) => {
            eprintln!("network rules unavailable: {error}");
            return json_error(StatusCode::SERVICE_UNAVAILABLE, "rules_unavailable");
        }
    };
    let issued: Option<i64> = match app
        .db()
        .query_row("SELECT issued FROM days WHERE day=?1", [sql(day)], |row| {
            row.get(0)
        })
        .optional()
    {
        Ok(issued) => issued,
        Err(error) => return internal(error),
    };
    Json(json!({
        "domain": hex0x(&app.domain),
        "server": hex0x(&app.server),
        "day": day,
        "bookSize": rules.book_size,
        "capCoins": rules.cap_coins,
        "grantsIssued": issued.map_or(0, unsql),
        "maxValidityDays": rules.max_validity_days,
    }))
    .into_response()
}

#[derive(Deserialize)]
struct CreateClaim {
    request: ClaimRequest,
}

async fn create_claim(State(app): State<Arc<App>>, Json(body): Json<CreateClaim>) -> Response {
    let request = body.request;
    if request.verify().is_err() {
        return json_error(StatusCode::BAD_REQUEST, "bad_signature");
    }
    if request.domain != app.domain {
        return json_error(StatusCode::BAD_REQUEST, "wrong_domain");
    }
    let now = app.now();
    if request.created_at > now.saturating_add(CLOCK_SKEW_SECS)
        || request.created_at.saturating_add(app.claim_ttl_secs) < now
    {
        return json_error(StatusCode::BAD_REQUEST, "stale_request");
    }
    let id = match random_token() {
        Ok(id) => id,
        Err(error) => return internal(error),
    };
    let expires_at = now.saturating_add(app.claim_ttl_secs);
    let db = app.db();
    let existing: Result<Option<(String, i64)>, _> = db
        .query_row(
            "SELECT id, expires_at FROM claims WHERE book=?1 AND nonce=?2",
            params![request.book.as_slice(), request.nonce.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional();
    let (id, expires_at, status) = match existing {
        Ok(Some((id, expires_at))) => (id, unsql(expires_at), StatusCode::OK),
        Ok(None) => {
            if let Err(error) = db.execute(
                "INSERT INTO claims(id, book, nonce, created_at, expires_at, status)
                 VALUES(?1, ?2, ?3, ?4, ?5, 'pending')",
                params![
                    id,
                    request.book.as_slice(),
                    request.nonce.as_slice(),
                    sql(request.created_at),
                    sql(expires_at)
                ],
            ) {
                return internal(error);
            }
            (id, expires_at, StatusCode::CREATED)
        }
        Err(error) => return internal(error),
    };
    drop(db);
    (
        status,
        Json(json!({
            "claimId": id,
            "loginUrl": format!("{}/v1/claims/{id}/login", app.public_url),
            "expiresAt": expires_at,
        })),
    )
        .into_response()
}

struct ClaimRow {
    status: String,
    reason: Option<String>,
    grant: Option<String>,
    expires_at: u64,
}

fn load_claim(db: &Connection, id: &str) -> rusqlite::Result<Option<ClaimRow>> {
    db.query_row(
        "SELECT status, reason, grant_json, expires_at FROM claims WHERE id=?1",
        [id],
        |row| {
            Ok(ClaimRow {
                status: row.get(0)?,
                reason: row.get(1)?,
                grant: row.get(2)?,
                expires_at: unsql(row.get(3)?),
            })
        },
    )
    .optional()
}

/// Denies a still-pending claim; a granted or denied claim keeps its outcome.
fn deny(db: &Connection, id: &str, reason: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE claims SET status='denied', reason=?2 WHERE id=?1 AND status='pending'",
        params![id, reason],
    )?;
    Ok(())
}

/// Marks a pending claim past its TTL as expired.
fn expire_if_due(
    db: &Connection,
    id: &str,
    claim: &mut ClaimRow,
    now: u64,
) -> rusqlite::Result<()> {
    if claim.status == "pending" && now >= claim.expires_at {
        deny(db, id, "claim_expired")?;
        claim.status = "denied".into();
        claim.reason = Some("claim_expired".into());
    }
    Ok(())
}

async fn claim_status(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let now = app.now();
    let db = app.db();
    let mut claim = match load_claim(&db, &id) {
        Ok(Some(claim)) => claim,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "unknown_claim"),
        Err(error) => return internal(error),
    };
    if let Err(error) = expire_if_due(&db, &id, &mut claim, now) {
        return internal(error);
    }
    drop(db);
    let mut body = json!({ "status": claim.status });
    if let Some(reason) = claim.reason {
        body["reason"] = json!(reason);
    }
    if let Some(grant) = claim.grant {
        match serde_json::from_str::<Value>(&grant) {
            Ok(grant) => body["grant"] = grant,
            Err(error) => return internal(error),
        }
    }
    Json(body).into_response()
}

/// Why a login may not start for this claim (unknown, finished or expired),
/// as the page that says so; `None` for a pending claim.
fn closed_claim(db: &Connection, id: &str, now: u64) -> Option<Response> {
    let mut claim = match load_claim(db, id) {
        Ok(Some(claim)) => claim,
        Ok(None) => {
            return Some(page(
                StatusCode::NOT_FOUND,
                "Unknown claim",
                "Start again from your terminal.",
            ));
        }
        Err(error) => return Some(internal(error)),
    };
    if let Err(error) = expire_if_due(db, id, &mut claim, now) {
        return Some(internal(error));
    }
    match claim.status.as_str() {
        "pending" => None,
        "denied" if claim.reason.as_deref() == Some("claim_expired") => Some(page(
            StatusCode::GONE,
            "Link expired",
            "Start a new claim from your terminal.",
        )),
        _ => Some(page(
            StatusCode::CONFLICT,
            "Claim finished",
            "Return to your terminal.",
        )),
    }
}

/// The login link: the providers to choose from, or Google straight away
/// when it is the only one.
async fn login_page(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    if app.github.is_none() {
        return login(State(app), Path(id)).await;
    }
    if let Some(page) = closed_claim(&app.db(), &id, app.now()) {
        return page;
    }
    // The id is one this server issued (URL-safe base64), checked above.
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Log in</title></head>\
         <body><h1>Log in to get your coins</h1>\
         <p><a href=\"/v1/claims/{id}/login/google\">Log in with Google</a></p>\
         <p><a href=\"/v1/claims/{id}/login/github\">Log in with GitHub</a></p></body></html>"
    );
    (StatusCode::OK, Html(body)).into_response()
}

async fn login(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let now = app.now();
    let (state, nonce, verifier) = match (random_token(), random_token(), random_token()) {
        (Ok(state), Ok(nonce), Ok(verifier)) => (state, nonce, verifier),
        _ => return internal("no system randomness"),
    };
    let db = app.db();
    if let Some(page) = closed_claim(&db, &id, now) {
        return page;
    }
    if let Err(error) = db.execute(
        "INSERT INTO logins(state, claim_id, nonce, verifier) VALUES(?1, ?2, ?3, ?4)",
        params![state, id, nonce, verifier],
    ) {
        return internal(error);
    }
    drop(db);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let redirect = format!("{}/v1/oauth/google/callback", app.public_url);
    let Ok(mut target) = Url::parse(&app.google.authorize_url) else {
        return internal("google authorize URL");
    };
    target
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &app.google.client_id)
        .append_pair("redirect_uri", &redirect)
        .append_pair("scope", "openid email")
        .append_pair("state", &state)
        .append_pair("nonce", &nonce)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("prompt", "select_account");
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, target.to_string())],
    )
        .into_response()
}

#[derive(Deserialize)]
struct Callback {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

/// A Google identity the server accepts, or the reason it does not.
enum Identity {
    Verified { subject_hash: [u8; 32] },
    Denied(&'static str),
}

fn identity(id_token: &str, client_id: &str, nonce: &str, now: u64) -> Identity {
    let invalid = Identity::Denied("invalid_identity_token");
    // The token comes straight from Google's token endpoint over TLS, which
    // Google documents as sufficient without verifying its signature.
    let Some(payload) = id_token.split('.').nth(1) else {
        return invalid;
    };
    let Some(claims) = URL_SAFE_NO_PAD
        .decode(payload)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        return invalid;
    };
    let audience = match &claims["aud"] {
        Value::String(aud) => aud == client_id,
        Value::Array(auds) => auds.iter().any(|aud| aud == client_id),
        _ => false,
    };
    let issuer = claims["iss"]
        .as_str()
        .is_some_and(|iss| GOOGLE_ISSUERS.contains(&iss));
    let fresh = claims["exp"].as_u64().is_some_and(|exp| exp > now);
    let bound = claims["nonce"].as_str() == Some(nonce);
    let Some(subject) = claims["sub"].as_str().filter(|sub| !sub.is_empty()) else {
        return invalid;
    };
    if !(audience && issuer && fresh && bound) {
        return invalid;
    }
    let verified = match &claims["email_verified"] {
        Value::Bool(verified) => *verified,
        Value::String(verified) => verified == "true",
        _ => false,
    };
    if !verified {
        return Identity::Denied("email_not_verified");
    }
    Identity::Verified {
        subject_hash: Sha256::digest(subject.as_bytes()).into(),
    }
}

async fn callback(State(app): State<Arc<App>>, Query(query): Query<Callback>) -> Response {
    let now = app.now();
    let Some(state) = query.state else {
        return page(
            StatusCode::BAD_REQUEST,
            "Unknown login",
            "Start again from your terminal.",
        );
    };
    // The state is consumed first: a login redirect is honoured at most once.
    let login = {
        let db = app.db();
        let login: rusqlite::Result<Option<(String, String, String)>> = db
            .query_row(
                "DELETE FROM logins WHERE state=?1 RETURNING claim_id, nonce, verifier",
                [&state],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional();
        let (claim_id, nonce, verifier) = match login {
            Ok(Some(login)) => login,
            Ok(None) => {
                return page(
                    StatusCode::BAD_REQUEST,
                    "Unknown login",
                    "Start again from your terminal.",
                );
            }
            Err(error) => return internal(error),
        };
        let mut claim = match load_claim(&db, &claim_id) {
            Ok(Some(claim)) => claim,
            Ok(None) => return internal("login without claim"),
            Err(error) => return internal(error),
        };
        if let Err(error) = expire_if_due(&db, &claim_id, &mut claim, now) {
            return internal(error);
        }
        if claim.status != "pending" {
            return page(
                StatusCode::CONFLICT,
                "Claim finished",
                "Return to your terminal.",
            );
        }
        (claim_id, nonce, verifier)
    };
    let (claim_id, nonce, verifier) = login;
    if query.error.is_some() {
        return page(
            StatusCode::BAD_REQUEST,
            "Sign-in cancelled",
            "Open the link from your terminal again to retry.",
        );
    }
    let Some(code) = query.code else {
        return page(
            StatusCode::BAD_REQUEST,
            "Sign-in failed",
            "Open the link again to retry.",
        );
    };
    let redirect = format!("{}/v1/oauth/google/callback", app.public_url);
    let exchange = app
        .http
        .post(&app.google.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", app.google.client_id.as_str()),
            ("client_secret", app.google.client_secret.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await;
    let tokens: Option<Value> = match exchange {
        Ok(response) if response.status().is_success() => response.json().await.ok(),
        _ => None,
    };
    let Some(id_token) = tokens
        .as_ref()
        .and_then(|tokens| tokens["id_token"].as_str())
    else {
        return page(
            StatusCode::BAD_GATEWAY,
            "Google did not confirm the sign-in",
            "Open the link from your terminal again to retry.",
        );
    };
    let subject_hash = match identity(id_token, &app.google.client_id, &nonce, now) {
        Identity::Verified { subject_hash } => subject_hash,
        Identity::Denied(reason) => {
            if let Err(error) = deny(&app.db(), &claim_id, reason) {
                return internal(error);
            }
            return page(StatusCode::FORBIDDEN, "No coins granted", reason);
        }
    };
    grant_for(&app, &claim_id, subject_hash, now).await
}

/// The network's rules for today, then the grant or the reason there is none.
async fn grant_for(app: &App, claim_id: &str, subject_hash: [u8; 32], now: u64) -> Response {
    let day = now / SECONDS_PER_DAY;
    let rules = match app.rules.rules(app.server, day).await {
        Ok(rules) => rules,
        Err(error) => {
            eprintln!("network rules unavailable: {error}");
            return page(
                StatusCode::SERVICE_UNAVAILABLE,
                "Try again later",
                "The network rules cannot be read right now; open the link again later.",
            );
        }
    };
    if rules.domain != app.domain {
        eprintln!("network rules belong to another domain");
        return page(
            StatusCode::SERVICE_UNAVAILABLE,
            "Try again later",
            "The server is misconfigured; no coins were spent.",
        );
    }
    match issue(app, claim_id, subject_hash, day, now, &rules) {
        Ok(None) => page(
            StatusCode::OK,
            "Coins granted",
            "Return to your terminal; your agent can use them now.",
        ),
        Ok(Some("claim_finished")) => page(
            StatusCode::CONFLICT,
            "Claim finished",
            "Return to your terminal.",
        ),
        Ok(Some(reason)) => page(StatusCode::FORBIDDEN, "No coins granted", reason),
        Err(error) => internal(error),
    }
}

async fn github_login(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let Some(github) = app.github.clone() else {
        return page(
            StatusCode::NOT_FOUND,
            "Not found",
            "GitHub is not offered here.",
        );
    };
    let now = app.now();
    let (state, verifier) = match (random_token(), random_token()) {
        (Ok(state), Ok(verifier)) => (state, verifier),
        _ => return internal("no system randomness"),
    };
    let db = app.db();
    if let Some(page) = closed_claim(&db, &id, now) {
        return page;
    }
    if let Err(error) = db.execute(
        "INSERT INTO github_logins(state, claim_id, verifier) VALUES(?1, ?2, ?3)",
        params![state, id, verifier],
    ) {
        return internal(error);
    }
    drop(db);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let redirect = format!("{}/v1/oauth/github/callback", app.public_url);
    let Ok(mut target) = Url::parse(&github.authorize_url) else {
        return internal("github authorize URL");
    };
    target
        .query_pairs_mut()
        .append_pair("client_id", &github.client_id)
        .append_pair("redirect_uri", &redirect)
        .append_pair("scope", "read:user user:email")
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("allow_signup", "false");
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, target.to_string())],
    )
        .into_response()
}

/// A GitHub account the server accepts: its immutable id, if a primary
/// verified email vouches for it. `Err` keeps the claim open (GitHub did not
/// answer as expected), `Ok(None)` denies it.
async fn github_account(
    app: &App,
    github: &GitHubConfig,
    code: &str,
    verifier: &str,
) -> Result<Option<[u8; 32]>, ()> {
    let redirect = format!("{}/v1/oauth/github/callback", app.public_url);
    let tokens: Value = app
        .http
        .post(&github.token_url)
        .header(header::ACCEPT, "application/json")
        .form(&[
            ("client_id", github.client_id.as_str()),
            ("client_secret", github.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect.as_str()),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|_| ())?
        .error_for_status()
        .map_err(|_| ())?
        .json()
        .await
        .map_err(|_| ())?;
    let token = tokens["access_token"].as_str().ok_or(())?;
    let get = |path: &str| {
        app.http
            .get(format!("{}{path}", github.api_url.trim_end_matches('/')))
            .bearer_auth(token)
            .header(header::USER_AGENT, "agentic-identity-server")
            .header(header::ACCEPT, "application/vnd.github+json")
            .send()
    };
    let user: Value = get("/user")
        .await
        .map_err(|_| ())?
        .error_for_status()
        .map_err(|_| ())?
        .json()
        .await
        .map_err(|_| ())?;
    let id = user["id"].as_u64().ok_or(())?;
    let emails: Value = get("/user/emails")
        .await
        .map_err(|_| ())?
        .error_for_status()
        .map_err(|_| ())?
        .json()
        .await
        .map_err(|_| ())?;
    let verified = emails.as_array().ok_or(())?.iter().any(|email| {
        email["primary"].as_bool() == Some(true) && email["verified"].as_bool() == Some(true)
    });
    // The id, not the login or an email: those can change.
    Ok(verified.then(|| Sha256::digest(format!("github:{id}").as_bytes()).into()))
}

async fn github_callback(State(app): State<Arc<App>>, Query(query): Query<Callback>) -> Response {
    let now = app.now();
    let Some(github) = app.github.clone() else {
        return page(
            StatusCode::NOT_FOUND,
            "Not found",
            "GitHub is not offered here.",
        );
    };
    let Some(state) = query.state else {
        return page(
            StatusCode::BAD_REQUEST,
            "Unknown login",
            "Start again from your terminal.",
        );
    };
    // The state is consumed first: a login redirect is honoured at most once,
    // and only at the callback of the provider it was started for.
    let (claim_id, verifier) = {
        let db = app.db();
        let login: rusqlite::Result<Option<(String, String)>> = db
            .query_row(
                "DELETE FROM github_logins WHERE state=?1 RETURNING claim_id, verifier",
                [&state],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional();
        let (claim_id, verifier) = match login {
            Ok(Some(login)) => login,
            Ok(None) => {
                return page(
                    StatusCode::BAD_REQUEST,
                    "Unknown login",
                    "Start again from your terminal.",
                );
            }
            Err(error) => return internal(error),
        };
        if let Some(page) = closed_claim(&db, &claim_id, now) {
            return page;
        }
        (claim_id, verifier)
    };
    if query.error.is_some() {
        return page(
            StatusCode::BAD_REQUEST,
            "Sign-in cancelled",
            "Open the link from your terminal again to retry.",
        );
    }
    let Some(code) = query.code else {
        return page(
            StatusCode::BAD_REQUEST,
            "Sign-in failed",
            "Open the link again to retry.",
        );
    };
    let subject_hash = match github_account(&app, &github, &code, &verifier).await {
        Ok(Some(subject_hash)) => subject_hash,
        Ok(None) => {
            if let Err(error) = deny(&app.db(), &claim_id, "email_not_verified") {
                return internal(error);
            }
            return page(
                StatusCode::FORBIDDEN,
                "No coins granted",
                "email_not_verified",
            );
        }
        Err(()) => {
            return page(
                StatusCode::BAD_GATEWAY,
                "GitHub did not confirm the sign-in",
                "Open the link from your terminal again to retry.",
            );
        }
    };
    grant_for(&app, &claim_id, subject_hash, now).await
}

/// Grants the claim or records why not, in one SQLite transaction so the
/// account check, the day's serial and the claim outcome cannot race.
fn issue(
    app: &App,
    claim_id: &str,
    subject_hash: [u8; 32],
    day: u64,
    now: u64,
    rules: &GrantRules,
) -> Result<Option<&'static str>, Error> {
    let mut db = app.db();
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let claim: Option<(String, Vec<u8>)> = tx
        .query_row(
            "SELECT status, book FROM claims WHERE id=?1",
            [claim_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((status, book)) = claim else {
        return Err(Error::Storage("claim vanished".into()));
    };
    if status != "pending" {
        return Ok(Some("claim_finished"));
    }
    let book: Account = book
        .try_into()
        .map_err(|_| Error::Storage("stored book is not an account".into()))?;
    let outcome = if !rules.issuer_active {
        Err("issuer_inactive")
    } else if tx
        .query_row(
            "SELECT 1 FROM bans WHERE subject_hash=?1 AND (until IS NULL OR until>?2)",
            params![subject_hash.as_slice(), sql(now)],
            |_| Ok(()),
        )
        .optional()?
        .is_some()
    {
        Err("account_banned")
    } else {
        let last: Option<i64> = tx
            .query_row(
                "SELECT last_grant_at FROM accounts WHERE subject_hash=?1",
                [subject_hash.as_slice()],
                |row| row.get(0),
            )
            .optional()?;
        let interval = app.claim_interval_days.saturating_mul(SECONDS_PER_DAY);
        let issued: u64 = tx
            .query_row("SELECT issued FROM days WHERE day=?1", [sql(day)], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?
            .map_or(0, unsql);
        let fits =
            u128::from(issued + 1) * u128::from(rules.book_size) <= u128::from(rules.cap_coins);
        if last.is_some_and(|last| now < unsql(last).saturating_add(interval)) {
            Err("already_claimed")
        } else if rules.book_size == 0 || !fits {
            Err("daily_cap_reached")
        } else {
            Ok(issued)
        }
    };
    let reason = match outcome {
        Err(reason) => {
            tx.execute(
                "UPDATE claims SET status='denied', reason=?2 WHERE id=?1",
                params![claim_id, reason],
            )?;
            Some(reason)
        }
        Ok(issued) => {
            let serial =
                u32::try_from(issued).map_err(|_| Error::Storage("serial overflow".into()))?;
            let validity = app.grant_validity_days.min(rules.max_validity_days);
            let grant = GrantBook::issue(
                GrantTerms {
                    domain: app.domain,
                    book,
                    day,
                    serial,
                    count: rules.book_size,
                    expiry: now.saturating_add(validity.saturating_mul(SECONDS_PER_DAY)),
                },
                &app.key,
            );
            let grant_json =
                serde_json::to_string(&grant).map_err(|error| Error::Storage(error.to_string()))?;
            tx.execute(
                "INSERT INTO days(day, issued) VALUES(?1, 1)
                 ON CONFLICT(day) DO UPDATE SET issued = issued + 1",
                [sql(day)],
            )?;
            tx.execute(
                "INSERT INTO accounts(subject_hash, last_grant_at) VALUES(?1, ?2)
                 ON CONFLICT(subject_hash) DO UPDATE SET last_grant_at = excluded.last_grant_at",
                params![subject_hash.as_slice(), sql(now)],
            )?;
            // Which account holds the grant, should it be spent twice.
            tx.execute(
                "INSERT INTO grants(book_id, subject_hash, issued_at, expiry, grant_json)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    grant.id().as_slice(),
                    subject_hash.as_slice(),
                    sql(now),
                    sql(grant.expiry),
                    grant_json
                ],
            )?;
            tx.execute(
                "UPDATE claims SET status='granted', grant_json=?2 WHERE id=?1",
                params![claim_id, grant_json],
            )?;
            None
        }
    };
    tx.commit()?;
    Ok(reason)
}

/// A stamp as holders show it over HTTP (the directory's form: plain hex).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StampWire {
    book: String,
    index: u32,
    operation: String,
    signature: String,
}

fn stamp_of(wire: &StampWire) -> Option<Stamp> {
    Some(Stamp {
        book: hex::decode(&wire.book).ok()?.try_into().ok()?,
        index: wire.index,
        operation: hex::decode(&wire.operation).ok()?.try_into().ok()?,
        signature: hex::decode(&wire.signature).ok()?.try_into().ok()?,
        holders: None,
    })
}

/// Two stamps of one slot of `grant`'s book for different operations.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    grant: GrantBook,
    first: StampWire,
    second: StampWire,
}

/// A holder's proof that a grant of this server was spent twice: its
/// account is banned and its live grants revoked.
async fn report(State(app): State<Arc<App>>, Json(body): Json<Report>) -> Response {
    let grant = body.grant;
    if grant.server != app.server || grant.domain != app.domain || grant.verify_signature().is_err()
    {
        return json_error(StatusCode::BAD_REQUEST, "not_ours");
    }
    let (Some(first), Some(second)) = (stamp_of(&body.first), stamp_of(&body.second)) else {
        return json_error(StatusCode::BAD_REQUEST, "no_proof");
    };
    let proof = SenderEquivocation { first, second };
    // One slot of this grant's own book, signed twice by its key.
    if proof.first.book != grant.id() || proof.verify(&app.domain, &grant.book).is_err() {
        return json_error(StatusCode::BAD_REQUEST, "no_proof");
    }
    match penalize(&app, &grant, app.now()) {
        Ok(Some((banned, revoked))) => {
            Json(json!({"banned": banned, "revoked": revoked})).into_response()
        }
        Ok(None) => json_error(StatusCode::BAD_REQUEST, "not_ours"),
        Err(error) => internal(error),
    }
}

/// A new offence of the account holding `grant` (its first, or a grant got
/// after its ban spent twice) bans it for [`FIRST_BAN_SECS`] or for good and
/// revokes its grants still in force; a grant from before accounts were
/// linked is revoked alone. Whether the account is banned now, and how many
/// grants this call revoked; `None` when another grant was issued under the
/// same id.
fn penalize(app: &App, grant: &GrantBook, now: u64) -> Result<Option<(bool, u64)>, Error> {
    let mut db = app.db();
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let book = grant.id();
    let linked: Option<(Vec<u8>, String, i64)> = tx
        .query_row(
            "SELECT subject_hash, grant_json, issued_at FROM grants WHERE book_id=?1",
            [book.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let subject = match linked {
        Some((subject, json, issued_at)) => {
            let issued: GrantBook =
                serde_json::from_str(&json).map_err(|error| Error::Storage(error.to_string()))?;
            // Another grant under the same id (signed with this key outside
            // a claim) is not the account's to answer for.
            if issued != *grant {
                return Ok(None);
            }
            Some((subject, unsql(issued_at)))
        }
        None => None,
    };
    let ban: Option<(i64, Option<i64>)> = match &subject {
        Some((subject, _)) => tx
            .query_row(
                "SELECT banned_at, until FROM bans WHERE subject_hash=?1",
                [subject],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?,
        None => None,
    };
    // A proof about a grant from before the ban is the offence it was for.
    let (offence, banned) = match (&subject, ban) {
        (None, _) => (false, false),
        (Some(_), None) => (true, true),
        (Some(_), Some((_, None))) => (false, true),
        (Some((_, issued_at)), Some((banned_at, Some(until)))) => {
            if *issued_at > unsql(banned_at) {
                (true, true)
            } else {
                (false, now < unsql(until))
            }
        }
    };
    let live: Vec<GrantBook> = match &subject {
        Some((subject, _)) if offence => {
            // The first offence bans for a while, a later one for good.
            let until = ban
                .is_none()
                .then(|| sql(now.saturating_add(FIRST_BAN_SECS)));
            tx.execute(
                "INSERT INTO bans(subject_hash, banned_at, until, book_id) VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(subject_hash) DO UPDATE SET banned_at = excluded.banned_at,
                     until = excluded.until, book_id = excluded.book_id",
                params![subject, sql(now), until, book.as_slice()],
            )?;
            let mut rows = tx.prepare(
                "SELECT grant_json FROM grants WHERE subject_hash=?1 AND expiry>?2 ORDER BY rowid",
            )?;
            let grants = rows
                .query_map(params![subject, sql(now)], |row| row.get::<_, String>(0))?
                .map(|json| {
                    serde_json::from_str(&json?).map_err(|error| Error::Storage(error.to_string()))
                })
                .collect::<Result<Vec<GrantBook>, Error>>()?;
            drop(rows);
            grants
        }
        Some(_) => vec![],
        None if now < grant.expiry => vec![grant.clone()],
        None => vec![],
    };
    let mut revoked = 0;
    for grant in live {
        let revocation = GrantRevocation::issue(&grant, now, &app.key);
        let json = serde_json::to_string(&revocation)
            .map_err(|error| Error::Storage(error.to_string()))?;
        revoked += tx.execute(
            "INSERT INTO revocations(book_id, expiry, revocation_json) VALUES(?1, ?2, ?3)
             ON CONFLICT(book_id) DO NOTHING",
            params![grant.id().as_slice(), sql(grant.expiry), json],
        )? as u64;
    }
    tx.commit()?;
    Ok(Some((banned, revoked)))
}

#[derive(Deserialize)]
struct After {
    after: Option<u64>,
}

/// Revocations after the `after`-th whose grants have not ended, in order,
/// a page at most, with the number of the last one listed.
async fn revocations(State(app): State<Arc<App>>, Query(query): Query<After>) -> Response {
    let after = query.after.unwrap_or(0);
    let now = app.now();
    let db = app.db();
    let page: rusqlite::Result<Vec<(i64, String)>> = db
        .prepare(
            "SELECT seq, revocation_json FROM revocations WHERE seq>?1 AND expiry>?2
             ORDER BY seq LIMIT ?3",
        )
        .and_then(|mut rows| {
            rows.query_map(params![sql(after), sql(now), sql(REVOCATION_PAGE)], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect()
        });
    drop(db);
    let page = match page {
        Ok(page) => page,
        Err(error) => return internal(error),
    };
    let last = page.last().map_or(after, |(seq, _)| unsql(*seq));
    let listed: Result<Vec<Value>, _> = page
        .iter()
        .map(|(_, json)| serde_json::from_str::<Value>(json))
        .collect();
    match listed {
        Ok(listed) => Json(json!({"revocations": listed, "last": last})).into_response(),
        Err(error) => internal(error),
    }
}
