//! The discovery service (spec/discovery-v1.md): Google and GitHub
//! sign-ins bind handles to profiles; lookups pay a stamp per handle and
//! find only exact handles; cards pay ten stamps and are found by interest
//! by owners of an active book. It is an index: every card is signed by its
//! author, every binding by this service; it keeps no handle, only a keyed
//! hash of its digest.

use agentic_grant_book::GrantBook;
use agentic_mailbox_swarm::access::AccessPass;
use agentic_mailbox_swarm::discover::{card_operation, lookup_operation};
use agentic_mailbox_swarm::stamp::Stamp;
use agentic_protocol::directory::{
    Binding, Card, KINDS, Request, normalize_handle, verify_request,
};
use agentic_protocol::group::group_ref;
use agentic_protocol::{DocumentDraft, DocumentKind, SignedDocument, network_id};
use axum::{
    Form, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::SigningKey;
use hmac::{Hmac, KeyInit, Mac};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    net::SocketAddr,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use url::Url;

/// Unix seconds; injected so tests can move across days.
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

pub type LedgerFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// What the node beside the service says of a stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Redeemed {
    Ok,
    /// `unknown_book` (read meanwhile; retry), `conflict`, `blocked`,
    /// `expired`, `index`, `signature`.
    Refused(String),
}

/// What the node knows of a book.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BookState {
    /// Active, signed by the key of this account.
    Active(agentic_mailbox_swarm::Account),
    Unknown,
    Ended,
    Blocked,
}

/// The node beside the service (owner IPC `redeem_stamps`, `book_status`).
pub trait Ledger: Send + Sync {
    /// Check `stamps`, learning the books of `grants` it does not know.
    fn redeem<'a>(
        &'a self,
        stamps: &'a [Stamp],
        grants: &'a [GrantBook],
    ) -> LedgerFuture<'a, Vec<Redeemed>>;
    fn book<'a>(&'a self, book: [u8; 32], grant: Option<GrantBook>) -> LedgerFuture<'a, BookState>;
}

/// The node beside the service, over its owner IPC.
pub struct IpcLedger {
    socket: PathBuf,
    token: [u8; 32],
}

impl IpcLedger {
    pub fn new(socket: PathBuf, token: [u8; 32]) -> Self {
        Self { socket, token }
    }

    async fn call(&self, method: &str, request: Value) -> Result<Value, String> {
        let answer = agentic_node::ipc::call(&self.socket, &self.token, method, request)
            .await
            .map_err(|error| error.to_string())?;
        answer.get("result").cloned().ok_or_else(|| {
            answer["error"]["code"]
                .as_str()
                .unwrap_or("node_error")
                .to_owned()
        })
    }
}

impl Ledger for IpcLedger {
    fn redeem<'a>(
        &'a self,
        stamps: &'a [Stamp],
        grants: &'a [GrantBook],
    ) -> LedgerFuture<'a, Vec<Redeemed>> {
        Box::pin(async move {
            let wires: Vec<Value> = stamps
                .iter()
                .map(|s| {
                    json!({
                        "book": hex::encode(s.book),
                        "index": s.index,
                        "operation": hex::encode(s.operation),
                        "signature": hex::encode(s.signature),
                    })
                })
                .collect();
            let result = self
                .call(
                    "redeem_stamps",
                    json!({ "stamps": wires, "grants": grants }),
                )
                .await?;
            result["results"]
                .as_array()
                .ok_or("no results")?
                .iter()
                .map(|r| match r.as_str() {
                    Some("ok") => Ok(Redeemed::Ok),
                    Some(code) => Ok(Redeemed::Refused(code.to_owned())),
                    None => Err("malformed result".to_owned()),
                })
                .collect()
        })
    }

    fn book<'a>(&'a self, book: [u8; 32], grant: Option<GrantBook>) -> LedgerFuture<'a, BookState> {
        Box::pin(async move {
            let result = self
                .call(
                    "book_status",
                    json!({ "book": hex::encode(book), "grant": grant }),
                )
                .await?;
            Ok(match result["state"].as_str() {
                Some("active") => {
                    let key = result["key"]
                        .as_str()
                        .and_then(|k| hex::decode(k).ok())
                        .and_then(|k| k.try_into().ok())
                        .ok_or("an active book without its key")?;
                    BookState::Active(key)
                }
                Some("ended") => BookState::Ended,
                Some("blocked") => BookState::Blocked,
                _ => BookState::Unknown,
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
    pub api_url: String,
}

pub struct Config {
    /// Base URL browsers and clients reach this service at.
    pub public_url: String,
    pub domain: [u8; 32],
    /// The Ed25519 key bindings are signed with.
    pub signing_secret: [u8; 32],
    /// Keys the hash handles are kept under.
    pub pepper: [u8; 32],
    pub database: PathBuf,
    pub google: GoogleConfig,
    pub github: Option<GitHubConfig>,
    pub ledger: Arc<dyn Ledger>,
    /// How long a consent and its login link stay usable.
    pub link_ttl_secs: u64,
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

/// A running service; [`Server::shutdown`] stops it.
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
            .route("/v1/links", post(create_link))
            .route("/v1/links/{id}", get(link_status))
            .route("/v1/links/{id}/login", get(login).post(confirm))
            .route("/v1/oauth/google/callback", get(google_callback))
            .route("/v1/oauth/github/callback", get(github_callback))
            .route("/v1/unlink", post(unlink))
            .route("/v1/lookup", post(lookup))
            .route("/v1/cards", post(publish))
            .route("/v1/cards/{id}", get(card))
            .route("/v1/withdraw", post(withdraw))
            .route("/v1/search", post(search))
            .layer(DefaultBodyLimit::max(MAX_BODY))
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
                eprintln!("directory stopped: {error}");
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
const LOOKUP_PRICE: usize = 1;
const CARD_PRICE: usize = 10;
const CARD_DAYS: u64 = 30;
const DAY: u64 = 86_400;
const MAX_LOOKUP: usize = 100;
const MAX_FOUND: usize = 50;
/// Searches a book may make in a minute. Not an address's: behind a proxy
/// every searcher has the proxy's.
const SEARCHES_PER_MINUTE: usize = 30;
const MAX_QUERY_BYTES: usize = 200;
const MAX_QUERY_WORDS: usize = 8;
const MAX_FILTER_BYTES: usize = 32;
/// Books that may pay one request.
const MAX_BOOKS: usize = 4;
const MAX_BODY: usize = 256 * 1024;
/// The cookie of the browser that confirmed a login page.
const LOGIN_COOKIE: &str = "ain_login";
/// A consent dated this far ahead of the service's clock is still taken.
const CLOCK_SKEW_SECS: u64 = 60;

struct App {
    public_url: String,
    domain: [u8; 32],
    key: SigningKey,
    pepper: [u8; 32],
    google: GoogleConfig,
    github: Option<GitHubConfig>,
    ledger: Arc<dyn Ledger>,
    link_ttl_secs: u64,
    clock: Clock,
    db: Mutex<Connection>,
    http: reqwest::Client,
    /// Searches in the current minute, by book.
    searches: Mutex<(u64, HashMap<[u8; 32], usize>)>,
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        Some(url::Host::Domain(name)) => name == "localhost",
        None => false,
    }
}

fn secure(url: &str) -> bool {
    Url::parse(url)
        .is_ok_and(|url| url.scheme() == "https" || url.scheme() == "http" && is_loopback(&url))
}

impl App {
    fn new(config: Config) -> Result<Self, Error> {
        if !secure(&config.google.token_url) {
            return Err(Error::Config(
                "Google's token endpoint must be https (or loopback)",
            ));
        }
        if let Some(github) = &config.github
            && !(secure(&github.token_url) && secure(&github.api_url))
        {
            return Err(Error::Config(
                "GitHub's endpoints must be https (or loopback)",
            ));
        }
        Url::parse(&config.public_url).map_err(|_| Error::Config("public URL is not a URL"))?;
        if config.link_ttl_secs == 0 {
            return Err(Error::Config("the link TTL must be positive"));
        }
        let db = open(&config.database)?;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| Error::Config("cannot build an HTTP client"))?;
        Ok(Self {
            public_url: config.public_url.trim_end_matches('/').to_owned(),
            domain: config.domain,
            key: SigningKey::from_bytes(&config.signing_secret),
            pepper: config.pepper,
            google: config.google,
            github: config.github,
            ledger: config.ledger,
            link_ttl_secs: config.link_ttl_secs,
            clock: config.clock,
            db: Mutex::new(db),
            http,
            searches: Mutex::new((0, HashMap::new())),
        })
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    fn db(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The key a handle's binding is kept under: a keyed hash of its kind
    /// and digest, never the handle.
    fn handle_key(&self, kind: &str, digest: &[u8; 32]) -> Vec<u8> {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&self.pepper) else {
            return vec![];
        };
        mac.update(kind.as_bytes());
        mac.update(b":");
        mac.update(digest);
        mac.finalize().into_bytes().to_vec()
    }

    fn account_key(&self, account: &str) -> Vec<u8> {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&self.pepper) else {
            return vec![];
        };
        mac.update(b"account:");
        mac.update(account.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    fn sign(&self, body: Vec<u8>) -> Option<Vec<u8>> {
        SignedDocument::sign(
            DocumentDraft {
                domain: self.domain,
                kind: DocumentKind::Directory,
                authority_epoch: 0,
                issued_at: self.now(),
                expires_at: None,
                body,
                extensions: BTreeMap::new(),
            },
            &self.key,
        )
        .ok()
        .map(|document| document.to_wire())
    }

    /// One more search of `book` this minute, or `false` past the limit.
    fn may_search(&self, book: [u8; 32]) -> bool {
        let minute = self.now() / 60;
        let mut searches = self
            .searches
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if searches.0 != minute {
            *searches = (minute, HashMap::new());
        }
        let count = searches.1.entry(book).or_default();
        if *count >= SEARCHES_PER_MINUTE {
            return false;
        }
        *count += 1;
        true
    }
}

fn open(path: &std::path::Path) -> Result<Connection, Error> {
    let db = Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS links(
             id TEXT PRIMARY KEY,
             network_id TEXT NOT NULL,
             kind TEXT NOT NULL,
             expires_at INTEGER NOT NULL,
             status TEXT NOT NULL,
             reason TEXT);
         CREATE TABLE IF NOT EXISTS logins(
             state TEXT PRIMARY KEY,
             link_id TEXT NOT NULL REFERENCES links(id),
             nonce TEXT NOT NULL,
             verifier TEXT NOT NULL,
             browser BLOB NOT NULL);
         CREATE TABLE IF NOT EXISTS consents(
             key BLOB PRIMARY KEY,
             expires_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS bindings(
             handle_key BLOB PRIMARY KEY,
             kind TEXT NOT NULL,
             network_id TEXT NOT NULL,
             account_key BLOB NOT NULL,
             issued_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS cards(
             id TEXT PRIMARY KEY,
             slot TEXT NOT NULL UNIQUE,
             kind TEXT NOT NULL,
             author TEXT NOT NULL,
             wire BLOB NOT NULL,
             text TEXT NOT NULL,
             tags TEXT NOT NULL,
             langs TEXT NOT NULL,
             published_at INTEGER NOT NULL,
             expires_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS payments(
             id TEXT PRIMARY KEY,
             expires_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS passes(
             nonce BLOB PRIMARY KEY,
             day INTEGER NOT NULL);",
    )?;
    Ok(db)
}

/// Drop what has run out: links a day after their end and their logins,
/// taken consents, cards, and the payments that keep a card from coming
/// back (by then it is too old to be shown for the first time).
fn purge(db: &Connection, now: u64) -> rusqlite::Result<()> {
    let now = sql(now);
    db.execute(
        "DELETE FROM logins WHERE link_id IN (SELECT id FROM links WHERE expires_at <= ?1)",
        params![now],
    )?;
    db.execute(
        "DELETE FROM links WHERE expires_at + 86400 <= ?1",
        params![now],
    )?;
    for table in ["consents", "cards", "payments"] {
        db.execute(
            &format!("DELETE FROM {table} WHERE expires_at <= ?1"),
            params![now],
        )?;
    }
    Ok(())
}

fn random_token() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(URL_SAFE_NO_PAD.encode(bytes))
}

fn sql(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn unsql(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn refuse(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({ "error": code }))).into_response()
}

fn internal(error: impl std::fmt::Display) -> Response {
    eprintln!("directory: {error}");
    refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal")
}

fn page(status: StatusCode, title: &str, text: &str) -> Response {
    (
        status,
        Html(format!(
            "<!doctype html><meta charset=utf-8><title>{title}</title><h1>{title}</h1><p>{text}</p>"
        )),
    )
        .into_response()
}

fn bytes32(text: &str) -> Option<[u8; 32]> {
    hex::decode(text).ok()?.try_into().ok()
}

async fn policy(State(app): State<Arc<App>>) -> Response {
    Json(json!({
        "domain": hex::encode(app.domain),
        "key": hex::encode(app.key.verifying_key().to_bytes()),
        "lookupPrice": LOOKUP_PRICE,
        "cardPrice": CARD_PRICE,
        "cardDays": CARD_DAYS,
    }))
    .into_response()
}

// --- bindings -------------------------------------------------------------------

#[derive(Deserialize)]
struct Consent {
    consent: String,
}

/// The author and kind of a consent to this service, fresh enough and not
/// taken before; taking it here.
fn consent_of(app: &App, wire: &str, link: bool) -> Option<(String, String)> {
    let wire = hex::decode(wire).ok()?;
    let now = app.now();
    let (author, request, issued_at) = verify_request(&wire, app.domain, now).ok()?;
    if issued_at + app.link_ttl_secs < now || issued_at > now + CLOCK_SKEW_SECS {
        return None;
    }
    // What was signed, not its wire: another encoding of one consent is
    // the same consent.
    let key: [u8; 32] = Sha256::new()
        .chain_update(author)
        .chain_update(issued_at.to_be_bytes())
        .chain_update(request.encode())
        .finalize()
        .into();
    let consent = match request {
        Request::Link { kind, service } if link && service == app.public_url => {
            (network_id(&author), kind)
        }
        Request::Unlink { kind, service } if !link && service == app.public_url => {
            (network_id(&author), kind)
        }
        _ => return None,
    };
    // Kept while it could still be taken.
    let taken = app
        .db()
        .execute(
            "INSERT OR IGNORE INTO consents(key, expires_at) VALUES(?1, ?2)",
            params![key.to_vec(), sql(issued_at + app.link_ttl_secs + 1)],
        )
        .ok()?;
    (taken == 1).then_some(consent)
}

/// The code a login page shows and `discover link` prints, so the human
/// tells a link they started from one someone handed them.
fn link_code(link: &str) -> String {
    let digest = hex::encode_upper(&Sha256::digest(link.as_bytes())[..4]);
    format!("{}-{}", &digest[..4], &digest[4..])
}

/// A link still waiting for its sign-in: its profile and kind.
fn pending_link(app: &App, link: &str) -> rusqlite::Result<Option<(String, String)>> {
    app.db()
        .query_row(
            "SELECT network_id, kind FROM links WHERE id = ?1 AND status = 'pending' AND expires_at > ?2",
            params![link, sql(app.now())],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
}

/// The login cookie a browser sent.
fn login_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == LOGIN_COOKIE)
        .map(|(_, value)| value.to_owned())
}

/// What a login keeps of the browser that confirmed it.
fn browser_key(cookie: &str) -> Vec<u8> {
    Sha256::digest(cookie.as_bytes()).to_vec()
}

async fn create_link(State(app): State<Arc<App>>, Json(body): Json<Consent>) -> Response {
    let Some((network, kind)) = consent_of(&app, &body.consent, true) else {
        return refuse(StatusCode::BAD_REQUEST, "bad_consent");
    };
    if kind == "github" && app.github.is_none() {
        return refuse(StatusCode::BAD_REQUEST, "bad_consent");
    }
    let Some(id) = random_token() else {
        return internal("no system randomness");
    };
    let now = app.now();
    let expires_at = now + app.link_ttl_secs;
    let stored = {
        let db = app.db();
        purge(&db, now).and_then(|()| {
            db.execute(
                "INSERT INTO links(id, network_id, kind, expires_at, status) VALUES(?1, ?2, ?3, ?4, 'pending')",
                params![id, network, kind, sql(expires_at)],
            )
        })
    };
    if let Err(error) = stored {
        return internal(error);
    }
    (
        StatusCode::CREATED,
        Json(json!({
            "linkId": id,
            "loginUrl": format!("{}/v1/links/{id}/login", app.public_url),
            "code": link_code(&id),
            "expiresAt": expires_at,
        })),
    )
        .into_response()
}

async fn link_status(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let row: rusqlite::Result<Option<(String, Option<String>)>> = app
        .db()
        .query_row(
            "SELECT status, reason FROM links WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional();
    match row {
        Ok(Some((status, reason))) => {
            Json(json!({"status": status, "reason": reason})).into_response()
        }
        Ok(None) => refuse(StatusCode::NOT_FOUND, "not_found"),
        Err(error) => internal(error),
    }
}

fn provider_name(kind: &str) -> &'static str {
    if kind == "github" { "GitHub" } else { "Google" }
}

/// The login page: it names the profile the account would be bound to and
/// the code its terminal shows, and binds nothing until confirmed in this
/// browser, whose cookie the confirmation and the provider's callback must
/// bring.
async fn login(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let (network, kind) = match pending_link(&app, &id) {
        Ok(Some(link)) => link,
        Ok(None) => {
            return page(
                StatusCode::GONE,
                "Link ended",
                "Start again from your terminal.",
            );
        }
        Err(error) => return internal(error),
    };
    let Some(token) = random_token() else {
        return internal("no system randomness");
    };
    let provider = provider_name(&kind);
    let code = link_code(&id);
    let body = format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\">\
         <title>Link your {provider} account</title><h1>Link your {provider} account</h1>\
         <p>Signing in links your {provider} account to the Kaiki Chat profile <b>{network}</b>: \
         people who know your account will find this profile.</p>\
         <p>Your terminal shows the code <b>{code}</b>. Go on only if you started this yourself \
         and see the same code there. If someone sent you this link, close this page.</p>\
         <form method=post><input type=hidden name=\"token\" value=\"{token}\">\
         <button>Sign in with {provider}</button></form>"
    );
    let secure = if app.public_url.starts_with("https://") {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "{LOGIN_COOKIE}={token}; Path=/v1/; HttpOnly; SameSite=Lax; Max-Age={}{secure}",
        app.link_ttl_secs
    );
    (
        StatusCode::OK,
        [
            (header::SET_COOKIE, cookie),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
        Html(body),
    )
        .into_response()
}

#[derive(Deserialize)]
struct Confirmation {
    token: String,
}

/// The login page confirmed: on to the provider, the login kept for this
/// browser.
async fn confirm(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Form(form): Form<Confirmation>,
) -> Response {
    // The page's token, from the browser it was shown in: a page elsewhere
    // cannot send this cookie with it.
    let Some(cookie) = login_cookie(&headers).filter(|c| !c.is_empty() && *c == form.token) else {
        return page(
            StatusCode::FORBIDDEN,
            "Not confirmed",
            "Open the link from your terminal again.",
        );
    };
    let kind = match pending_link(&app, &id) {
        Ok(Some((_, kind))) => kind,
        Ok(None) => {
            return page(
                StatusCode::GONE,
                "Link ended",
                "Start again from your terminal.",
            );
        }
        Err(error) => return internal(error),
    };
    let (Some(state), Some(nonce), Some(verifier)) =
        (random_token(), random_token(), random_token())
    else {
        return internal("no system randomness");
    };
    let stored = {
        let db = app.db();
        db.execute("DELETE FROM logins WHERE link_id = ?1", params![id])
            .and_then(|_| {
                db.execute(
                    "INSERT INTO logins(state, link_id, nonce, verifier, browser) VALUES(?1, ?2, ?3, ?4, ?5)",
                    params![state, id, nonce, verifier, browser_key(&cookie)],
                )
            })
    };
    if let Err(error) = stored {
        return internal(error);
    }
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let target = if kind == "google" {
        let Ok(mut target) = Url::parse(&app.google.authorize_url) else {
            return internal("google authorize URL");
        };
        target
            .query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &app.google.client_id)
            .append_pair(
                "redirect_uri",
                &format!("{}/v1/oauth/google/callback", app.public_url),
            )
            .append_pair("scope", "openid email")
            .append_pair("state", &state)
            .append_pair("nonce", &nonce)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("prompt", "select_account");
        target
    } else {
        let Some(github) = &app.github else {
            return page(
                StatusCode::NOT_FOUND,
                "Not found",
                "GitHub is not offered here.",
            );
        };
        let Ok(mut target) = Url::parse(&github.authorize_url) else {
            return internal("github authorize URL");
        };
        target
            .query_pairs_mut()
            .append_pair("client_id", &github.client_id)
            .append_pair(
                "redirect_uri",
                &format!("{}/v1/oauth/github/callback", app.public_url),
            )
            .append_pair("scope", "")
            .append_pair("state", &state)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("allow_signup", "false");
        target
    };
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
}

/// The login a callback answers, in the browser that confirmed it,
/// consumed: its link, profile, nonce and verifier.
fn take_login(
    app: &App,
    state: &str,
    headers: &HeaderMap,
) -> Option<(String, String, String, String)> {
    let browser = browser_key(&login_cookie(headers)?);
    let db = app.db();
    let login: Option<(String, String, String)> = db
        .query_row(
            "SELECT link_id, nonce, verifier FROM logins WHERE state = ?1 AND browser = ?2",
            params![state, browser],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .ok()?;
    let (link, nonce, verifier) = login?;
    db.execute("DELETE FROM logins WHERE state = ?1", params![state])
        .ok()?;
    let network: String = db
        .query_row(
            "SELECT network_id FROM links WHERE id = ?1 AND status = 'pending' AND expires_at > ?2",
            params![link, sql(app.now())],
            |row| row.get(0),
        )
        .ok()?;
    Some((link, network, nonce, verifier))
}

fn deny(app: &App, link: &str, reason: &str) -> Response {
    let _ = app.db().execute(
        "UPDATE links SET status = 'denied', reason = ?2 WHERE id = ?1",
        params![link, reason],
    );
    page(
        StatusCode::FORBIDDEN,
        "Not linked",
        "The account was not linked. Go back to your terminal.",
    )
}

/// Bind `handle` of `kind` to `network`, replacing what the handle, or the
/// account behind it, bound before.
fn bind(app: &App, link: &str, network: &str, kind: &str, handle: &str, account: &str) -> Response {
    let Some(normal) = normalize_handle(kind, handle) else {
        return deny(app, link, "unusable_handle");
    };
    let digest: [u8; 32] = Sha256::digest(normal.as_bytes()).into();
    let handle_key = app.handle_key(kind, &digest);
    let account_key = app.account_key(&format!("{kind}:{account}"));
    let mut db = app.db();
    let result = (|| -> rusqlite::Result<()> {
        let tx = db.transaction()?;
        // One handle per account, and per kind of a profile.
        tx.execute(
            "DELETE FROM bindings WHERE account_key = ?1",
            params![account_key],
        )?;
        tx.execute(
            "DELETE FROM bindings WHERE kind = ?1 AND network_id = ?2",
            params![kind, network],
        )?;
        // Not the binding: it holds the digest, which is signed anew for
        // each paid lookup that asks for it.
        tx.execute(
            "INSERT OR REPLACE INTO bindings(handle_key, kind, network_id, account_key, issued_at) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![handle_key, kind, network, account_key, sql(app.now())],
        )?;
        tx.execute(
            "UPDATE links SET status = 'linked' WHERE id = ?1",
            params![link],
        )?;
        tx.commit()
    })();
    match result {
        Ok(()) => page(
            StatusCode::OK,
            "Linked",
            "Your account is linked. Go back to your terminal.",
        ),
        Err(error) => internal(error),
    }
}

async fn google_callback(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Query(query): Query<Callback>,
) -> Response {
    let (Some(state), Some(code)) = (query.state, query.code) else {
        return page(
            StatusCode::BAD_REQUEST,
            "Unknown login",
            "Start again from your terminal.",
        );
    };
    let Some((link, network, nonce, verifier)) = take_login(&app, &state, &headers) else {
        return page(
            StatusCode::BAD_REQUEST,
            "Unknown login",
            "Start again from your terminal.",
        );
    };
    let redirect = format!("{}/v1/oauth/google/callback", app.public_url);
    let answer: Option<Value> = async {
        app.http
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
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    let Some(claims) = answer
        .as_ref()
        .and_then(|a| a["id_token"].as_str())
        .and_then(|token| token.split('.').nth(1))
        .and_then(|payload| URL_SAFE_NO_PAD.decode(payload).ok())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        return deny(&app, &link, "invalid_identity_token");
    };
    // The token comes straight from Google's token endpoint over TLS.
    let audience = match &claims["aud"] {
        Value::String(aud) => *aud == app.google.client_id,
        Value::Array(auds) => auds.iter().any(|aud| *aud == app.google.client_id),
        _ => false,
    };
    let issuer = claims["iss"]
        .as_str()
        .is_some_and(|iss| GOOGLE_ISSUERS.contains(&iss));
    let fresh = claims["exp"].as_u64().is_some_and(|exp| exp > app.now());
    let bound = claims["nonce"].as_str() == Some(nonce.as_str());
    let (Some(subject), Some(email)) = (claims["sub"].as_str(), claims["email"].as_str()) else {
        return deny(&app, &link, "invalid_identity_token");
    };
    if !(audience && issuer && fresh && bound) {
        return deny(&app, &link, "invalid_identity_token");
    }
    let verified = match &claims["email_verified"] {
        Value::Bool(verified) => *verified,
        Value::String(verified) => verified == "true",
        _ => false,
    };
    if !verified {
        return deny(&app, &link, "email_not_verified");
    }
    bind(&app, &link, &network, "google", email, subject)
}

async fn github_callback(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    Query(query): Query<Callback>,
) -> Response {
    let Some(github) = app.github.clone() else {
        return page(
            StatusCode::NOT_FOUND,
            "Not found",
            "GitHub is not offered here.",
        );
    };
    let (Some(state), Some(code)) = (query.state, query.code) else {
        return page(
            StatusCode::BAD_REQUEST,
            "Unknown login",
            "Start again from your terminal.",
        );
    };
    let Some((link, network, _, verifier)) = take_login(&app, &state, &headers) else {
        return page(
            StatusCode::BAD_REQUEST,
            "Unknown login",
            "Start again from your terminal.",
        );
    };
    let redirect = format!("{}/v1/oauth/github/callback", app.public_url);
    let user: Option<Value> = async {
        let tokens: Value = app
            .http
            .post(&github.token_url)
            .header(header::ACCEPT, "application/json")
            .form(&[
                ("client_id", github.client_id.as_str()),
                ("client_secret", github.client_secret.as_str()),
                ("code", code.as_str()),
                ("redirect_uri", redirect.as_str()),
                ("code_verifier", verifier.as_str()),
            ])
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json()
            .await
            .ok()?;
        let token = tokens["access_token"].as_str()?.to_owned();
        app.http
            .get(format!("{}/user", github.api_url.trim_end_matches('/')))
            .bearer_auth(token)
            .header(header::USER_AGENT, "agentic-directory")
            .header(header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json()
            .await
            .ok()
    }
    .await;
    let (Some(id), Some(login)) = (
        user.as_ref().and_then(|u| u["id"].as_u64()),
        user.as_ref().and_then(|u| u["login"].as_str()),
    ) else {
        return deny(&app, &link, "github_unavailable");
    };
    // The login is the handle; the id keeps one binding per account.
    bind(&app, &link, &network, "github", login, &id.to_string())
}

async fn unlink(State(app): State<Arc<App>>, Json(body): Json<Consent>) -> Response {
    let Some((network, kind)) = consent_of(&app, &body.consent, false) else {
        return refuse(StatusCode::BAD_REQUEST, "bad_consent");
    };
    match app.db().execute(
        "DELETE FROM bindings WHERE kind = ?1 AND network_id = ?2",
        params![kind, network],
    ) {
        Ok(_) => Json(json!({})).into_response(),
        Err(error) => internal(error),
    }
}

// --- payment --------------------------------------------------------------------

#[derive(Deserialize)]
struct StampWire {
    book: String,
    index: u32,
    operation: String,
    signature: String,
}

fn stamp_of(wire: &StampWire) -> Option<Stamp> {
    Some(Stamp {
        book: bytes32(&wire.book)?,
        index: wire.index,
        operation: bytes32(&wire.operation)?,
        signature: hex::decode(&wire.signature).ok()?.try_into().ok()?,
        holders: None,
    })
}

/// Stamps paying exactly for `operations`, one each in order, redeemed at
/// the node; the refusal to answer otherwise.
async fn pay(
    app: &App,
    stamps: &[StampWire],
    grants: &[GrantBook],
    operations: &[[u8; 32]],
) -> Result<(), Response> {
    let stamps: Option<Vec<Stamp>> = stamps.iter().map(stamp_of).collect();
    let Some(stamps) = stamps.filter(|stamps| {
        stamps.len() == operations.len()
            && stamps
                .iter()
                .zip(operations)
                .all(|(stamp, operation)| stamp.operation == *operation)
    }) else {
        return Err(refuse(StatusCode::PAYMENT_REQUIRED, "payment_required"));
    };
    // A few books, and only their grants: each unknown one costs the node a
    // read or a check.
    let mut books: Vec<[u8; 32]> = stamps.iter().map(|stamp| stamp.book).collect();
    books.sort_unstable();
    books.dedup();
    if books.len() > MAX_BOOKS {
        return Err(refuse(StatusCode::BAD_REQUEST, "invalid_request"));
    }
    let grants: Vec<GrantBook> = grants
        .iter()
        .filter(|grant| books.contains(&grant.id()))
        .take(MAX_BOOKS)
        .cloned()
        .collect();
    let redeemed = app
        .ledger
        .redeem(&stamps, &grants)
        .await
        .map_err(|_| refuse(StatusCode::SERVICE_UNAVAILABLE, "ledger_unavailable"))?;
    if redeemed.len() != stamps.len() {
        return Err(refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "ledger_unavailable",
        ));
    }
    if redeemed
        .iter()
        .any(|r| *r == Redeemed::Refused("unknown_book".into()))
    {
        return Err(refuse(StatusCode::CONFLICT, "unknown_book"));
    }
    if redeemed.iter().any(|r| *r != Redeemed::Ok) {
        return Err(refuse(StatusCode::PAYMENT_REQUIRED, "stamp_refused"));
    }
    Ok(())
}

// --- lookups --------------------------------------------------------------------

#[derive(Deserialize)]
struct Handle {
    kind: String,
    digest: String,
}

#[derive(Deserialize)]
struct Lookup {
    handles: Vec<Handle>,
    /// The UTC day the stamps were made for.
    day: u64,
    stamps: Vec<StampWire>,
    #[serde(default)]
    grants: Vec<GrantBook>,
}

async fn lookup(State(app): State<Arc<App>>, Json(body): Json<Lookup>) -> Response {
    if body.handles.is_empty() || body.handles.len() > MAX_LOOKUP {
        return refuse(StatusCode::BAD_REQUEST, "invalid_request");
    }
    let handles: Option<Vec<(String, [u8; 32])>> = body
        .handles
        .iter()
        .map(|h| {
            KINDS
                .contains(&h.kind.as_str())
                .then(|| bytes32(&h.digest).map(|d| (h.kind.clone(), d)))
                .flatten()
        })
        .collect();
    let Some(handles) = handles else {
        return refuse(StatusCode::BAD_REQUEST, "invalid_request");
    };
    // Today's stamps, or yesterday's: a stamp answers for two days at most.
    let today = app.now() / DAY;
    if body.day != today && body.day.saturating_add(1) != today {
        return refuse(StatusCode::PAYMENT_REQUIRED, "payment_required");
    }
    let operations: Vec<[u8; 32]> = handles
        .iter()
        .map(|(kind, digest)| lookup_operation(&app.domain, kind, digest, body.day))
        .collect();
    if let Err(refusal) = pay(&app, &body.stamps, &body.grants, &operations).await {
        return refusal;
    }
    let db = app.db();
    let mut results = vec![];
    for (kind, digest) in &handles {
        let bound: rusqlite::Result<Option<(String, i64)>> = db
            .query_row(
                "SELECT network_id, issued_at FROM bindings WHERE handle_key = ?1",
                params![app.handle_key(kind, digest)],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional();
        let binding = match bound {
            Ok(None) => None,
            // Signed for the digest asked about; none is kept.
            Ok(Some((network_id, issued_at))) => {
                let binding = Binding {
                    kind: kind.clone(),
                    digest: *digest,
                    network_id,
                    issued_at: unsql(issued_at),
                };
                let Some(wire) = app.sign(binding.encode()) else {
                    return internal("cannot sign a binding");
                };
                Some(hex::encode(wire))
            }
            Err(error) => return internal(error),
        };
        results.push(json!({
            "kind": kind,
            "digest": hex::encode(digest),
            "binding": binding,
        }));
    }
    Json(json!({ "results": results })).into_response()
}

// --- cards ----------------------------------------------------------------------

#[derive(Deserialize)]
struct Publish {
    card: String,
    stamps: Vec<StampWire>,
    #[serde(default)]
    grants: Vec<GrantBook>,
}

async fn publish(State(app): State<Arc<App>>, Json(body): Json<Publish>) -> Response {
    let now = app.now();
    let Some(wire) = hex::decode(&body.card).ok() else {
        return refuse(StatusCode::BAD_REQUEST, "bad_card");
    };
    let Ok((author, Request::Card(card), issued_at)) = verify_request(&wire, app.domain, now)
    else {
        return refuse(StatusCode::BAD_REQUEST, "bad_card");
    };
    if card.check().is_err() {
        return refuse(StatusCode::BAD_REQUEST, "bad_card");
    }
    let id: [u8; 32] = Sha256::digest(&wire).into();
    // A card is paid once: shown again, as a retry does, it is answered as
    // it stands, neither renewed nor brought back once replaced or
    // withdrawn.
    let paid: rusqlite::Result<Option<Option<i64>>> = app
        .db()
        .query_row(
            "SELECT cards.expires_at FROM payments LEFT JOIN cards ON cards.id = payments.id
             WHERE payments.id = ?1 AND payments.expires_at > ?2",
            params![hex::encode(id), sql(now)],
            |row| row.get(0),
        )
        .optional();
    match paid {
        Ok(Some(Some(expires_at))) => {
            return Json(json!({"id": hex::encode(id), "expiresAt": unsql(expires_at)}))
                .into_response();
        }
        Ok(Some(None)) => return refuse(StatusCode::GONE, "card_ended"),
        Ok(None) => {}
        Err(error) => return internal(error),
    }
    // First shown, a card is fresh: by the time its payment is forgotten it
    // is far too old to be paid for again.
    if issued_at + app.link_ttl_secs < now || issued_at > now + CLOCK_SKEW_SECS {
        return refuse(StatusCode::BAD_REQUEST, "bad_card");
    }
    let operations: Vec<[u8; 32]> = (0..CARD_PRICE as u32)
        .map(|i| card_operation(&app.domain, &id, i))
        .collect();
    if let Err(refusal) = pay(&app, &body.stamps, &body.grants, &operations).await {
        return refusal;
    }
    let owner = network_id(&author);
    // A channel's card takes the slot of its group's: one card per G.
    let (kind, slot) = match &card {
        Card::Group { group_id, .. } | Card::Channel { group_id, .. } => (
            if matches!(card, Card::Channel { .. }) {
                "channel"
            } else {
                "group"
            },
            format!(
                "group:{}",
                hex::encode(group_ref(&app.domain, &author, group_id))
            ),
        ),
        Card::Profile { .. } => ("profile", format!("profile:{owner}")),
    };
    let text = format!("{} {}", card.name(), card.about()).to_lowercase();
    let tags = format!(" {} ", card.tags().join(" "));
    let langs = format!(" {} ", card.langs().join(" "));
    let expires_at = now + CARD_DAYS * DAY;
    let mut db = app.db();
    let result = (|| -> rusqlite::Result<()> {
        let tx = db.transaction()?;
        purge(&tx, now)?;
        tx.execute(
            "DELETE FROM cards WHERE slot = ?1 OR id = ?2",
            params![slot, hex::encode(id)],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO payments(id, expires_at) VALUES(?1, ?2)",
            params![hex::encode(id), sql(expires_at)],
        )?;
        tx.execute(
            "INSERT INTO cards(id, slot, kind, author, wire, text, tags, langs, published_at, expires_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                hex::encode(id),
                slot,
                kind,
                owner,
                wire,
                text,
                tags,
                langs,
                sql(now),
                sql(expires_at)
            ],
        )?;
        tx.commit()
    })();
    match result {
        Ok(()) => (
            StatusCode::CREATED,
            Json(json!({"id": hex::encode(id), "expiresAt": expires_at})),
        )
            .into_response(),
        Err(error) => internal(error),
    }
}

fn card_json(id: String, wire: Vec<u8>, published_at: i64, expires_at: i64) -> Value {
    json!({
        "id": id,
        "card": hex::encode(wire),
        "publishedAt": unsql(published_at),
        "expiresAt": unsql(expires_at),
    })
}

async fn card(State(app): State<Arc<App>>, Path(id): Path<String>) -> Response {
    let row: rusqlite::Result<Option<(Vec<u8>, i64, i64)>> = app
        .db()
        .query_row(
            "SELECT wire, published_at, expires_at FROM cards WHERE id = ?1 AND expires_at > ?2",
            params![id, sql(app.now())],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional();
    match row {
        Ok(Some((wire, published_at, expires_at))) => {
            Json(card_json(id, wire, published_at, expires_at)).into_response()
        }
        Ok(None) => refuse(StatusCode::NOT_FOUND, "not_found"),
        Err(error) => internal(error),
    }
}

#[derive(Deserialize)]
struct Withdrawal {
    withdrawal: String,
}

async fn withdraw(State(app): State<Arc<App>>, Json(body): Json<Withdrawal>) -> Response {
    let Some(wire) = hex::decode(&body.withdrawal).ok() else {
        return refuse(StatusCode::BAD_REQUEST, "bad_consent");
    };
    let Ok((author, Request::Withdraw { card }, _)) = verify_request(&wire, app.domain, app.now())
    else {
        return refuse(StatusCode::BAD_REQUEST, "bad_consent");
    };
    match app.db().execute(
        "DELETE FROM cards WHERE id = ?1 AND author = ?2",
        params![card, network_id(&author)],
    ) {
        Ok(1) => Json(json!({})).into_response(),
        Ok(_) => refuse(StatusCode::BAD_REQUEST, "not_found"),
        Err(error) => internal(error),
    }
}

// --- search ---------------------------------------------------------------------

#[derive(Deserialize)]
struct PassWire {
    book: String,
    peer: String,
    day: u64,
    signature: String,
    #[serde(default)]
    grant: Option<GrantBook>,
}

#[derive(Deserialize)]
struct Search {
    query: String,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    pass: Option<PassWire>,
}

async fn search(State(app): State<Arc<App>>, Json(body): Json<Search>) -> Response {
    let now = app.now();
    let words: Vec<String> = body
        .query
        .to_lowercase()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let long =
        |filter: &Option<String>| filter.as_ref().is_some_and(|f| f.len() > MAX_FILTER_BYTES);
    if body.query.len() > MAX_QUERY_BYTES
        || words.len() > MAX_QUERY_WORDS
        || long(&body.tag)
        || long(&body.lang)
        || long(&body.kind)
    {
        return refuse(StatusCode::BAD_REQUEST, "invalid_request");
    }
    let Some(wire) = body.pass else {
        return refuse(StatusCode::UNAUTHORIZED, "book_required");
    };
    let pass = (|| {
        Some(AccessPass {
            book: bytes32(&wire.book)?,
            peer: bytes32(&wire.peer)?,
            day: wire.day,
            signature: hex::decode(&wire.signature).ok()?.try_into().ok()?,
        })
    })();
    let Some(pass) = pass else {
        return refuse(StatusCode::UNAUTHORIZED, "bad_pass");
    };
    let key = match app.ledger.book(pass.book, wire.grant).await {
        Ok(BookState::Active(key)) => key,
        Ok(_) => return refuse(StatusCode::UNAUTHORIZED, "book_required"),
        Err(_) => return refuse(StatusCode::SERVICE_UNAVAILABLE, "ledger_unavailable"),
    };
    let terms = agentic_mailbox_swarm::stamp::BookTerms {
        key,
        count: 0,
        valid_until: u64::MAX,
    };
    if pass.verify(&app.domain, &terms, now).is_err() {
        return refuse(StatusCode::UNAUTHORIZED, "bad_pass");
    }
    let taken = {
        let db = app.db();
        let _ = db.execute(
            "DELETE FROM passes WHERE day + 1 < ?1",
            params![sql(now / DAY)],
        );
        db.execute(
            "INSERT OR IGNORE INTO passes(nonce, day) VALUES(?1, ?2)",
            params![pass.peer.to_vec(), sql(pass.day)],
        )
    };
    match taken {
        Ok(1) => {}
        Ok(_) => return refuse(StatusCode::UNAUTHORIZED, "bad_pass"),
        Err(error) => return internal(error),
    }
    if !app.may_search(pass.book) {
        return refuse(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    }
    let db = app.db();
    let mut statement = match db.prepare(
        "SELECT id, wire, published_at, expires_at, text, tags, langs, kind FROM cards
         WHERE expires_at > ?1 ORDER BY published_at DESC, id",
    ) {
        Ok(statement) => statement,
        Err(error) => return internal(error),
    };
    let rows = statement.query_map(params![sql(now)], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Vec<u8>>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
        ))
    });
    let rows = match rows {
        Ok(rows) => rows,
        Err(error) => return internal(error),
    };
    let mut cards = vec![];
    for row in rows {
        let Ok((id, wire, published_at, expires_at, text, tags, langs, kind)) = row else {
            continue;
        };
        let matches = words
            .iter()
            .all(|word| text.contains(word.as_str()) || tags.contains(&format!(" {word} ")))
            && body
                .tag
                .as_ref()
                .is_none_or(|tag| tags.contains(&format!(" {} ", tag.to_lowercase())))
            && body
                .lang
                .as_ref()
                .is_none_or(|lang| langs.contains(&format!(" {} ", lang.to_lowercase())))
            && body.kind.as_ref().is_none_or(|k| *k == kind);
        if matches {
            cards.push(card_json(id, wire, published_at, expires_at));
            if cards.len() == MAX_FOUND {
                break;
            }
        }
    }
    Json(json!({ "cards": cards })).into_response()
}
