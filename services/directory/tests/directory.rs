//! The discovery service over real HTTP (spec/discovery-v1.md): Google and
//! GitHub sign-ins bind handles to profiles, lookups pay a stamp per handle
//! and find only exact handles, cards pay ten stamps and are found by
//! interest by owners of an active book. A local provider stands in for
//! Google's and GitHub's OAuth endpoints; a ledger stands in for the node
//! that checks stamps and books.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use agentic_directory::{
    BookState, Config, GitHubConfig, GoogleConfig, Ledger, LedgerFuture, Redeemed, Server,
};
use agentic_grant_book::{GrantBook, GrantTerms, SecpKey};
use agentic_mailbox_swarm::access::AccessPass;
use agentic_mailbox_swarm::discover::{card_operation, lookup_operation};
use agentic_mailbox_swarm::stamp::{BookKey, Stamp, ticket_id};
use agentic_protocol::directory::{Card, Request as Signed, handle_digest, verify_binding};
use agentic_protocol::{DocumentDraft, DocumentKind, SignedDocument, network_id};
use axum::{
    Form, Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tempfile::TempDir;
use url::Url;

const DOMAIN: [u8; 32] = [0x5a; 32];
const SERVICE_KEY: [u8; 32] = [0x21; 32];
const PEPPER: [u8; 32] = [0x22; 32];
const CLIENT_ID: &str = "agentic-directory.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "directory-client-secret";
const AUTHORIZE: &str = "https://accounts.example.test/o/oauth2/v2/auth";
const GITHUB_CLIENT_ID: &str = "Iv1.directory-test";
const GITHUB_CLIENT_SECRET: &str = "github-directory-secret";
const GITHUB_AUTHORIZE: &str = "https://github.example.test/login/oauth/authorize";
const DAY: u64 = 86_400;
const NOW: u64 = 20_721 * DAY + 10 * 3_600;

// --- the providers ------------------------------------------------------------

/// Codes issued by the Google stand-in: the PKCE challenge, redirect URI
/// and the ID token claims.
type Google = Arc<Mutex<HashMap<String, (String, String, Value)>>>;

async fn google_token(
    State(google): State<Google>,
    Form(form): Form<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let refuse = |reason: &str| (StatusCode::BAD_REQUEST, Json(json!({"error": reason})));
    if form.get("grant_type").map(String::as_str) != Some("authorization_code") {
        return refuse("unsupported_grant_type");
    }
    if form.get("client_id").map(String::as_str) != Some(CLIENT_ID)
        || form.get("client_secret").map(String::as_str) != Some(CLIENT_SECRET)
    {
        return refuse("invalid_client");
    }
    let Some((challenge, redirect, claims)) = form
        .get("code")
        .and_then(|code| google.lock().unwrap().remove(code))
    else {
        return refuse("invalid_grant");
    };
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    if form.get("redirect_uri") != Some(&redirect)
        || URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != challenge
    {
        return refuse("invalid_grant");
    }
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    let id_token = format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT"})),
        part(&claims),
        URL_SAFE_NO_PAD.encode(b"signature")
    );
    (
        StatusCode::OK,
        Json(json!({"id_token": id_token, "access_token": "a"})),
    )
}

#[derive(Clone)]
struct GitHubUser {
    id: u64,
    login: String,
}

#[derive(Default)]
struct GitHub {
    codes: HashMap<String, (String, String, GitHubUser)>,
    tokens: HashMap<String, GitHubUser>,
}

type GitHubState = Arc<Mutex<GitHub>>;

async fn github_token(
    State(github): State<GitHubState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> Json<Value> {
    // Without it GitHub answers form-encoded.
    if !headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|a| a.contains("application/json"))
    {
        return Json(json!({"error": "not_acceptable"}));
    }
    let mut github = github.lock().unwrap();
    if form.get("client_secret").map(String::as_str) != Some(GITHUB_CLIENT_SECRET) {
        return Json(json!({"error": "incorrect_client_credentials"}));
    }
    let Some((challenge, redirect, user)) = form.get("code").and_then(|c| github.codes.remove(c))
    else {
        return Json(json!({"error": "bad_verification_code"}));
    };
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    if form.get("redirect_uri") != Some(&redirect)
        || URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != challenge
    {
        return Json(json!({"error": "bad_verification_code"}));
    }
    let token = format!("gho_{}", github.tokens.len());
    github.tokens.insert(token.clone(), user);
    Json(json!({"access_token": token, "token_type": "bearer"}))
}

async fn github_user(
    State(github): State<GitHubState>,
    headers: axum::http::HeaderMap,
) -> (StatusCode, Json<Value>) {
    match github_account(&github, &headers) {
        Some(user) => (
            StatusCode::OK,
            Json(json!({"id": user.id, "login": user.login})),
        ),
        None => (StatusCode::UNAUTHORIZED, Json(json!({}))),
    }
}

fn github_account(github: &GitHubState, headers: &axum::http::HeaderMap) -> Option<GitHubUser> {
    // GitHub's API refuses requests without a User-Agent.
    headers.get("user-agent")?;
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| github.lock().unwrap().tokens.get(token).cloned())
}

// --- the ledger -----------------------------------------------------------------

/// A book as the node knows it.
#[derive(Clone)]
struct KnownBook {
    key: agentic_mailbox_swarm::Account,
    count: u32,
    valid_until: u64,
    blocked: bool,
}

/// The node beside the service: it checks stamps against their books and
/// records each ticket once, as a notary does. A grant shown with a stamp
/// funds its book once the node knows it (here at once).
#[derive(Default)]
struct TestLedger {
    books: Mutex<BTreeMap<[u8; 32], KnownBook>>,
    tickets: Mutex<BTreeMap<[u8; 32], [u8; 32]>>,
    redeemed: AtomicU64,
    now: Arc<AtomicU64>,
    /// The node does not answer.
    down: std::sync::atomic::AtomicBool,
}

impl TestLedger {
    fn learn(&self, grant: &GrantBook) {
        self.books
            .lock()
            .unwrap()
            .entry(grant.id())
            .or_insert(KnownBook {
                key: grant.book,
                count: grant.count,
                valid_until: grant.expiry,
                blocked: false,
            });
    }
}

impl Ledger for TestLedger {
    fn redeem<'a>(
        &'a self,
        stamps: &'a [Stamp],
        grants: &'a [GrantBook],
    ) -> LedgerFuture<'a, Vec<Redeemed>> {
        Box::pin(async move {
            if self.down.load(Ordering::SeqCst) {
                return Err("node unavailable".into());
            }
            for grant in grants {
                self.learn(grant);
            }
            let now = self.now.load(Ordering::SeqCst);
            let books = self.books.lock().unwrap();
            let mut tickets = self.tickets.lock().unwrap();
            Ok(stamps
                .iter()
                .map(|stamp| {
                    let Some(book) = books.get(&stamp.book) else {
                        return Redeemed::Refused("unknown_book".into());
                    };
                    if book.blocked {
                        return Redeemed::Refused("blocked".into());
                    }
                    let terms = agentic_mailbox_swarm::stamp::BookTerms {
                        key: book.key,
                        count: book.count,
                        valid_until: book.valid_until,
                    };
                    if stamp.verify(&DOMAIN, &terms, now).is_err() {
                        return Redeemed::Refused("stamp".into());
                    }
                    let ticket = ticket_id(&DOMAIN, &stamp.book, stamp.index);
                    match tickets.get(&ticket) {
                        Some(operation) if *operation != stamp.operation => {
                            Redeemed::Refused("conflict".into())
                        }
                        Some(_) => Redeemed::Ok,
                        None => {
                            tickets.insert(ticket, stamp.operation);
                            self.redeemed.fetch_add(1, Ordering::SeqCst);
                            Redeemed::Ok
                        }
                    }
                })
                .collect())
        })
    }

    fn book<'a>(&'a self, book: [u8; 32], grant: Option<GrantBook>) -> LedgerFuture<'a, BookState> {
        Box::pin(async move {
            if self.down.load(Ordering::SeqCst) {
                return Err("node unavailable".into());
            }
            if let Some(grant) = grant.filter(|g| g.id() == book) {
                self.learn(&grant);
            }
            let now = self.now.load(Ordering::SeqCst);
            Ok(match self.books.lock().unwrap().get(&book) {
                None => BookState::Unknown,
                Some(known) if known.blocked => BookState::Blocked,
                Some(known) if now >= known.valid_until => BookState::Ended,
                Some(known) => BookState::Active(known.key),
            })
        })
    }
}

// --- a profile ------------------------------------------------------------------

struct Profile {
    root: SigningKey,
    book: BookKey,
    book_id: [u8; 32],
    /// The identity server's grant the book is, if it is one.
    grant: Option<GrantBook>,
    next_slot: u32,
}

impl Profile {
    /// A profile whose bought book the node knows.
    fn new(seed: u8, ledger: &TestLedger) -> Self {
        let profile = Self::unknown(seed);
        ledger.books.lock().unwrap().insert(
            profile.book_id,
            KnownBook {
                key: profile.book.account(),
                count: 1_000,
                valid_until: NOW + 60 * DAY,
                blocked: false,
            },
        );
        profile
    }

    /// A profile with a book the node has not heard of.
    fn unknown(seed: u8) -> Self {
        Self {
            root: SigningKey::from_bytes(&[seed; 32]),
            book: BookKey::from_bytes(&[seed.wrapping_add(100); 32]).unwrap(),
            book_id: [seed; 32],
            grant: None,
            next_slot: 0,
        }
    }

    /// A newcomer whose only book is a grant of the identity server.
    fn granted(seed: u8) -> Self {
        let mut profile = Self::unknown(seed);
        let grant = GrantBook::issue(
            GrantTerms {
                domain: DOMAIN,
                book: profile.book.account(),
                day: NOW / DAY,
                serial: u32::from(seed),
                count: 50,
                expiry: NOW + 30 * DAY,
            },
            &SecpKey::from_secret(&[0x31; 32]).unwrap(),
        );
        profile.book_id = grant.id();
        profile.grant = Some(grant);
        profile
    }

    fn id(&self) -> String {
        network_id(&self.root.verifying_key().to_bytes())
    }

    fn signed(&self, body: Vec<u8>, now: u64) -> String {
        hex::encode(
            SignedDocument::sign(
                DocumentDraft {
                    domain: DOMAIN,
                    kind: DocumentKind::Directory,
                    authority_epoch: 0,
                    issued_at: now,
                    expires_at: None,
                    body,
                    extensions: BTreeMap::new(),
                },
                &self.root,
            )
            .unwrap()
            .to_wire(),
        )
    }

    /// A stamp of the next slot for `operation`.
    fn stamp(&mut self, operation: [u8; 32]) -> Value {
        let stamp = Stamp::sign(&DOMAIN, self.book_id, self.next_slot, operation, &self.book);
        self.next_slot += 1;
        wire(&stamp)
    }

    fn grants(&self) -> Value {
        json!(self.grant.iter().collect::<Vec<_>>())
    }

    fn pass(&self, nonce: [u8; 32], now: u64) -> Value {
        let pass = AccessPass::sign(&DOMAIN, self.book_id, nonce, now / DAY, &self.book);
        json!({
            "book": hex::encode(pass.book),
            "peer": hex::encode(pass.peer),
            "day": pass.day,
            "signature": hex::encode(pass.signature),
            "grant": self.grant,
        })
    }
}

fn wire(stamp: &Stamp) -> Value {
    json!({
        "book": hex::encode(stamp.book),
        "index": stamp.index,
        "operation": hex::encode(stamp.operation),
        "signature": hex::encode(stamp.signature),
    })
}

/// What a lookup sends for a handle: the digest of its normal form.
fn digest(kind: &str, handle: &str) -> [u8; 32] {
    handle_digest(kind, handle).unwrap()
}

// --- the harness ----------------------------------------------------------------

struct Harness {
    dir: TempDir,
    now: Arc<AtomicU64>,
    google: Google,
    github: GitHubState,
    provider: String,
    ledger: Arc<TestLedger>,
    server: Option<Server>,
    public_url: String,
    http: reqwest::Client,
    codes: AtomicU64,
}

impl Harness {
    async fn start() -> Self {
        let google: Google = Arc::default();
        let github: GitHubState = Arc::default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let provider = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new()
            .route("/token", post(google_token))
            .with_state(google.clone())
            .merge(
                Router::new()
                    .route("/github/token", post(github_token))
                    .route("/github/api/user", get(github_user))
                    .with_state(github.clone()),
            );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let now = Arc::new(AtomicU64::new(NOW));
        let ledger = Arc::new(TestLedger {
            now: now.clone(),
            ..TestLedger::default()
        });
        let mut harness = Self {
            dir: TempDir::new().unwrap(),
            now,
            google,
            github,
            provider,
            ledger,
            server: None,
            public_url: String::new(),
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            codes: AtomicU64::new(0),
        };
        harness.launch().await;
        harness
    }

    async fn launch(&mut self) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        self.public_url = format!("http://{}", listener.local_addr().unwrap());
        let clock = self.now.clone();
        let config = Config {
            public_url: self.public_url.clone(),
            domain: DOMAIN,
            signing_secret: SERVICE_KEY,
            pepper: PEPPER,
            database: self.dir.path().join("directory.db"),
            google: GoogleConfig {
                client_id: CLIENT_ID.into(),
                client_secret: CLIENT_SECRET.into(),
                authorize_url: AUTHORIZE.into(),
                token_url: format!("{}/token", self.provider),
            },
            github: Some(GitHubConfig {
                client_id: GITHUB_CLIENT_ID.into(),
                client_secret: GITHUB_CLIENT_SECRET.into(),
                authorize_url: GITHUB_AUTHORIZE.into(),
                token_url: format!("{}/github/token", self.provider),
                api_url: format!("{}/github/api", self.provider),
            }),
            ledger: self.ledger.clone(),
            link_ttl_secs: 900,
            clock: Arc::new(move || clock.load(Ordering::SeqCst)),
        };
        self.server = Some(Server::start(config, listener).await.unwrap());
    }

    async fn restart(&mut self) {
        self.server.take().unwrap().shutdown().await;
        self.launch().await;
    }

    fn base(&self) -> String {
        self.public_url.clone()
    }

    fn now(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }

    async fn call(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        let url = format!("{}{path}", self.base());
        let request = match method {
            "GET" => self.http.get(url),
            _ => self.http.post(url),
        };
        let request = match body {
            Some(body) => request.json(&body),
            None => request,
        };
        let response = request.send().await.unwrap();
        let status = response.status().as_u16();
        let body = response.json().await.unwrap_or(Value::Null);
        (status, body)
    }

    async fn key(&self) -> [u8; 32] {
        let (_, policy) = self.call("GET", "/v1/policy", None).await;
        hex::decode(policy["key"].as_str().unwrap())
            .unwrap()
            .try_into()
            .unwrap()
    }

    /// A consent signed now; a consent is taken once, so each is signed a
    /// second after the one before.
    fn consent(&self, profile: &Profile, link: bool, kind: &str, service: &str) -> String {
        self.now.fetch_add(1, Ordering::SeqCst);
        let body = if link {
            Signed::Link {
                kind: kind.into(),
                service: service.into(),
            }
        } else {
            Signed::Unlink {
                kind: kind.into(),
                service: service.into(),
            }
        };
        profile.signed(body.encode(), self.now())
    }

    /// A login link for `profile` to sign in with `kind`.
    async fn open_link(&self, profile: &Profile, kind: &str) -> Value {
        let consent = self.consent(profile, true, kind, &self.base());
        let (status, opened) = self
            .call("POST", "/v1/links", Some(json!({ "consent": consent })))
            .await;
        assert_eq!(status, 201, "{opened}");
        opened
    }

    /// A browser opening a login link: the page it shows, the cookie it
    /// sets and the token its confirmation sends.
    async fn open_login(&self, url: &str) -> (String, String, String) {
        let page = self.http.get(url).send().await.unwrap();
        assert_eq!(page.status(), 200);
        let set = page.headers()["set-cookie"].to_str().unwrap().to_owned();
        // Sent back on the provider's redirect to the callback, a top-level
        // navigation from another site; never to scripts.
        for attribute in ["HttpOnly", "SameSite=Lax", "Path=/v1/"] {
            assert!(set.contains(attribute), "{set}");
        }
        let cookie = set.split(';').next().unwrap().to_owned();
        let page = page.text().await.unwrap();
        let token = page
            .split_once("name=\"token\" value=\"")
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(token, _)| token.to_owned())
            .unwrap_or_else(|| panic!("{page}"));
        (page, cookie, token)
    }

    /// Confirming on the login page with its `token`; a browser that did not
    /// open the page sends no cookie, or another one.
    async fn confirm(&self, url: &str, cookie: Option<&str>, token: &str) -> reqwest::Response {
        let mut request = self.http.post(url).form(&[("token", token)]);
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        request.send().await.unwrap()
    }

    /// Bind `profile` to what the human signs in with: `kind`'s login link
    /// is followed like a browser, and the provider vouches for `account`.
    /// The callback page's status and the link's.
    async fn link(&self, profile: &Profile, kind: &str, account: Account) -> (u16, Value) {
        let opened = self.open_link(profile, kind).await;
        let link = opened["linkId"].as_str().unwrap().to_owned();
        let url = opened["loginUrl"].as_str().unwrap();
        let (_, cookie, token) = self.open_login(url).await;
        let sent = self.confirm(url, Some(&cookie), &token).await;
        assert!(sent.status().is_redirection(), "{}", sent.status());
        let target = Url::parse(sent.headers()["location"].to_str().unwrap()).unwrap();
        let callback = self.sign_in(&target, account, Some(&cookie)).await;
        let (_, status) = self.call("GET", &format!("/v1/links/{link}"), None).await;
        (callback, status)
    }

    /// At the provider's page `target`, the human signs in as `account`
    /// and the provider sends the browser back: the callback page's status.
    async fn sign_in(&self, target: &Url, account: Account, cookie: Option<&str>) -> u16 {
        let query: HashMap<String, String> = target.query_pairs().into_owned().collect();
        let code = format!("code-{}", self.codes.fetch_add(1, Ordering::SeqCst));
        let redirect = query["redirect_uri"].clone();
        let challenge = query["code_challenge"].clone();
        match account {
            Account::Google(email, verified) => {
                let claims = json!({
                    "iss": "https://accounts.google.com",
                    "aud": CLIENT_ID,
                    "sub": format!("sub-{email}"),
                    "email": email,
                    "email_verified": verified,
                    "nonce": query["nonce"],
                    "exp": self.now() + 3_600,
                });
                self.google
                    .lock()
                    .unwrap()
                    .insert(code.clone(), (challenge, redirect.clone(), claims));
            }
            Account::GitHub(id, login) => {
                self.github.lock().unwrap().codes.insert(
                    code.clone(),
                    (
                        challenge,
                        redirect.clone(),
                        GitHubUser {
                            id,
                            login: login.into(),
                        },
                    ),
                );
            }
        }
        let mut back = self
            .http
            .get(format!("{redirect}?state={}&code={code}", query["state"]));
        if let Some(cookie) = cookie {
            back = back.header("cookie", cookie);
        }
        back.send().await.unwrap().status().as_u16()
    }

    fn day(&self) -> u64 {
        self.now() / DAY
    }

    /// A lookup of `handles` paid by `payer` today.
    fn paid_lookup(&self, payer: &mut Profile, handles: &[(&str, &str)]) -> Value {
        let day = self.day();
        let handles: Vec<(String, [u8; 32])> = handles
            .iter()
            .map(|(kind, handle)| ((*kind).to_owned(), digest(kind, handle)))
            .collect();
        let stamps: Vec<Value> = handles
            .iter()
            .map(|(kind, d)| payer.stamp(lookup_operation(&DOMAIN, kind, d, day)))
            .collect();
        json!({
            "handles": handles
                .iter()
                .map(|(kind, d)| json!({"kind": kind, "digest": hex::encode(d)}))
                .collect::<Vec<_>>(),
            "day": day,
            "stamps": stamps,
            "grants": payer.grants(),
        })
    }

    async fn lookup(&self, payer: &mut Profile, handles: &[(&str, &str)]) -> (u16, Value) {
        let body = self.paid_lookup(payer, handles);
        self.call("POST", "/v1/lookup", Some(body)).await
    }

    async fn found(&self, payer: &mut Profile, kind: &str, handle: &str) -> Option<String> {
        let (status, body) = self.lookup(payer, &[(kind, handle)]).await;
        assert_eq!(status, 200, "{body}");
        let binding = &body["results"][0]["binding"];
        if binding.is_null() {
            return None;
        }
        let wire = hex::decode(binding.as_str().unwrap()).unwrap();
        let verified = verify_binding(&wire, DOMAIN, self.now(), &self.key().await).unwrap();
        assert_eq!(
            (verified.kind.as_str(), verified.digest),
            (kind, digest(kind, handle))
        );
        Some(verified.network_id)
    }

    fn redeemed(&self) -> u64 {
        self.ledger.redeemed.load(Ordering::SeqCst)
    }
}

enum Account {
    Google(&'static str, bool),
    GitHub(u64, &'static str),
}

// --- bindings -------------------------------------------------------------------

#[tokio::test]
async fn a_google_account_binds_its_profile_and_is_found_only_by_its_exact_address() {
    let h = Harness::start().await;
    let ann = Profile::new(1, &h.ledger);
    let mut asker = Profile::new(2, &h.ledger);
    // An unverified address binds nothing.
    let (page, denied) = h
        .link(&ann, "google", Account::Google("ann@example.org", false))
        .await;
    assert_eq!(
        (page, denied["status"].as_str()),
        (403, Some("denied")),
        "{denied}"
    );
    let (page, linked) = h
        .link(
            &ann,
            "google",
            Account::Google("Ann.Lee+work@GoogleMail.com", true),
        )
        .await;
    assert_eq!(
        (page, linked["status"].as_str()),
        (200, Some("linked")),
        "{linked}"
    );
    // The Gmail forms of one address are one handle; nothing else matches.
    for form in [
        "annlee@gmail.com",
        "ann.lee@gmail.com",
        "Ann.Lee+other@googlemail.com",
    ] {
        assert_eq!(
            h.found(&mut asker, "google", form).await,
            Some(ann.id()),
            "{form}"
        );
    }
    assert_eq!(h.found(&mut asker, "google", "ann@example.org").await, None);
    // Every address asked for paid its stamp, found or not.
    assert_eq!(h.redeemed(), 4);
    // The service keeps no address, nor its digest: a stolen database and a
    // list of addresses do not say whose profile an address is.
    let kept = digest("google", "annlee@gmail.com");
    for file in std::fs::read_dir(h.dir.path()).unwrap() {
        let path = file.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(path).unwrap();
        let text = String::from_utf8_lossy(&bytes).to_lowercase();
        assert!(
            !text.contains("annlee") && !text.contains("ann.lee"),
            "an address was kept"
        );
        assert!(
            !bytes.windows(32).any(|w| w == kept) && !text.contains(&hex::encode(kept)),
            "an address's digest was kept"
        );
    }
    // A consent to another service binds nothing here, and a consent is
    // taken once.
    let elsewhere = h.consent(&ann, true, "google", "https://other.example");
    let (status, body) = h
        .call("POST", "/v1/links", Some(json!({ "consent": elsewhere })))
        .await;
    assert_eq!((status, body["error"].as_str()), (400, Some("bad_consent")));
    let once = h.consent(&ann, true, "google", &h.base());
    let (status, _) = h
        .call("POST", "/v1/links", Some(json!({ "consent": once })))
        .await;
    assert_eq!(status, 201);
    let (status, body) = h
        .call("POST", "/v1/links", Some(json!({ "consent": once })))
        .await;
    assert_eq!((status, body["error"].as_str()), (400, Some("bad_consent")));
    // A profile has one Google binding: signing in with another account
    // replaces the first.
    h.link(&ann, "google", Account::Google("ann@example.org", true))
        .await;
    assert_eq!(
        h.found(&mut asker, "google", "annlee@gmail.com").await,
        None
    );
    assert_eq!(
        h.found(&mut asker, "google", "ann@example.org").await,
        Some(ann.id())
    );
    // An unlinking consent is taken only while fresh: an old one cannot be
    // replayed after the profile linked again.
    let stale = h.consent(&ann, false, "google", &h.base());
    h.now.fetch_add(16 * 60, Ordering::SeqCst);
    let (status, body) = h
        .call("POST", "/v1/unlink", Some(json!({ "consent": stale })))
        .await;
    assert_eq!((status, body["error"].as_str()), (400, Some("bad_consent")));
    // Unlinked, the address finds nobody.
    let consent = h.consent(&ann, false, "google", &h.base());
    let (status, body) = h
        .call(
            "POST",
            "/v1/unlink",
            Some(json!({ "consent": consent.clone() })),
        )
        .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(h.found(&mut asker, "google", "ann@example.org").await, None);
    // Linked again, the same unlinking consent, still fresh, is refused.
    h.link(&ann, "google", Account::Google("ann@example.org", true))
        .await;
    let (status, body) = h
        .call("POST", "/v1/unlink", Some(json!({ "consent": consent })))
        .await;
    assert_eq!((status, body["error"].as_str()), (400, Some("bad_consent")));
    assert_eq!(
        h.found(&mut asker, "google", "ann@example.org").await,
        Some(ann.id())
    );
}

/// Someone who opens a login link for their own profile and hands it, or
/// the provider's page it leads to, to another human does not get that
/// human's account bound to their profile.
#[tokio::test]
async fn a_login_binds_only_in_the_browser_that_confirmed_the_profile_it_names() {
    let h = Harness::start().await;
    let mallory = Profile::new(1, &h.ledger);
    let mut asker = Profile::new(2, &h.ledger);
    for (kind, ann, handle) in [
        (
            "google",
            Account::Google("ann@example.org", true),
            "ann@example.org",
        ),
        ("github", Account::GitHub(41, "ann-lee"), "ann-lee"),
    ] {
        let opened = h.open_link(&mallory, kind).await;
        let url = opened["loginUrl"].as_str().unwrap();
        // The page names the profile being linked and the code its terminal
        // shows, and binds nothing by being opened.
        let (page, cookie, token) = h.open_login(url).await;
        let code = opened["code"].as_str().unwrap();
        assert!(
            page.contains(&mallory.id()) && page.contains(code),
            "{page}"
        );
        // A page elsewhere confirming it in Ann's browser: without the
        // page's cookie, or with the cookie of Ann's own visit, refused.
        assert_eq!(h.confirm(url, None, &token).await.status(), 403);
        let (_, ann_cookie, _) = h.open_login(url).await;
        assert_eq!(
            h.confirm(url, Some(&ann_cookie), &token).await.status(),
            403
        );
        // Mallory confirms in her own browser and hands Ann the provider's
        // page: Ann's sign-in comes back to a browser that confirmed nothing.
        let sent = h.confirm(url, Some(&cookie), &token).await;
        assert!(sent.status().is_redirection(), "{}", sent.status());
        let target = Url::parse(sent.headers()["location"].to_str().unwrap()).unwrap();
        assert_eq!(h.sign_in(&target, ann, None).await, 400, "{kind}");
        let link = opened["linkId"].as_str().unwrap();
        let (_, status) = h.call("GET", &format!("/v1/links/{link}"), None).await;
        assert_eq!(status["status"], "pending", "{status}");
        assert_eq!(h.found(&mut asker, kind, handle).await, None, "{kind}");
    }
}

#[tokio::test]
async fn a_lookup_pays_one_stamp_per_address_a_day_and_a_retry_nothing_more() {
    let h = Harness::start().await;
    let ann = Profile::new(1, &h.ledger);
    h.link(&ann, "google", Account::Google("ann@example.org", true))
        .await;
    let mut asker = Profile::new(2, &h.ledger);
    let (ann_digest, bob_digest) = (
        digest("google", "ann@example.org"),
        digest("google", "bob@example.org"),
    );
    let day = h.day();
    // Two addresses, one stamp: refused, and nothing is spent.
    let one = asker.stamp(lookup_operation(&DOMAIN, "google", &ann_digest, day));
    let (status, body) = h
        .call(
            "POST",
            "/v1/lookup",
            Some(json!({
                "handles": [
                    {"kind": "google", "digest": hex::encode(ann_digest)},
                    {"kind": "google", "digest": hex::encode(bob_digest)},
                ],
                "day": day,
                "stamps": [one],
            })),
        )
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (402, Some("payment_required"))
    );
    // A stamp paying for another address is no payment for this one.
    let other = asker.stamp(lookup_operation(&DOMAIN, "google", &bob_digest, day));
    let (status, body) = h
        .call(
            "POST",
            "/v1/lookup",
            Some(json!({
                "handles": [{"kind": "google", "digest": hex::encode(ann_digest)}],
                "day": day,
                "stamps": [other],
            })),
        )
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (402, Some("payment_required"))
    );
    assert_eq!(h.redeemed(), 0);
    // Paid: answered; the same request again, even the next day, is
    // answered again and spends nothing more.
    let paid = h.paid_lookup(&mut asker, &[("google", "ann@example.org")]);
    let (status, first) = h.call("POST", "/v1/lookup", Some(paid.clone())).await;
    assert_eq!(status, 200, "{first}");
    assert!(first["results"][0]["binding"].is_string());
    h.now.fetch_add(DAY, Ordering::SeqCst);
    let (status, again) = h.call("POST", "/v1/lookup", Some(paid.clone())).await;
    assert_eq!(status, 200, "{again}");
    let binding = hex::decode(again["results"][0]["binding"].as_str().unwrap()).unwrap();
    let verified = verify_binding(&binding, DOMAIN, h.now(), &h.key().await).unwrap();
    assert_eq!(
        (verified.digest, verified.network_id),
        (ann_digest, ann.id())
    );
    assert_eq!(h.redeemed(), 1);
    // A day later still, it is no payment: an address is not watched for
    // one stamp.
    h.now.fetch_add(DAY, Ordering::SeqCst);
    let (status, body) = h.call("POST", "/v1/lookup", Some(paid.clone())).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (402, Some("payment_required")),
        "{body}"
    );
    // Nor is a stamp dated ahead: it would answer for three days.
    let ahead = h.day() + 1;
    let (status, body) = h
        .call(
            "POST",
            "/v1/lookup",
            Some(json!({
                "handles": [{"kind": "google", "digest": hex::encode(ann_digest)}],
                "day": ahead,
                "stamps": [asker.stamp(lookup_operation(&DOMAIN, "google", &ann_digest, ahead))],
            })),
        )
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (402, Some("payment_required")),
        "{body}"
    );
    assert_eq!(h.redeemed(), 1);
    // The slot spent on Ann's address, shown again for another: the node
    // refuses it as a double spend.
    let carol = digest("google", "carol@example.org");
    let reused = Stamp::sign(
        &DOMAIN,
        asker.book_id,
        u32::try_from(paid["stamps"][0]["index"].as_u64().unwrap()).unwrap(),
        lookup_operation(&DOMAIN, "google", &carol, h.day()),
        &asker.book,
    );
    let (status, body) = h
        .call(
            "POST",
            "/v1/lookup",
            Some(json!({
                "handles": [{"kind": "google", "digest": hex::encode(carol)}],
                "day": h.day(),
                "stamps": [wire(&reused)],
            })),
        )
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (402, Some("stamp_refused")),
        "{body}"
    );
    // A book the node has not read yet: try again later.
    let mut stranger = Profile::unknown(3);
    let (status, body) = h
        .lookup(&mut stranger, &[("google", "ann@example.org")])
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (409, Some("unknown_book")),
        "{body}"
    );
    // A newcomer pays with the grant it was given, shown with its stamps.
    let mut newcomer = Profile::granted(4);
    assert_eq!(
        h.found(&mut newcomer, "google", "ann@example.org").await,
        Some(ann.id())
    );
    // Without its node the service answers nothing, free or not.
    h.ledger.down.store(true, Ordering::SeqCst);
    let (status, body) = h.lookup(&mut asker, &[("google", "ann@example.org")]).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (503, Some("ledger_unavailable"))
    );
}

#[tokio::test]
async fn a_github_login_binds_a_rename_moves_it_and_another_profile_takes_the_account_over() {
    let h = Harness::start().await;
    let first = Profile::new(1, &h.ledger);
    let second = Profile::new(2, &h.ledger);
    let mut asker = Profile::new(3, &h.ledger);
    let (_, linked) = h
        .link(&first, "github", Account::GitHub(583_231, "Octo-Cat"))
        .await;
    assert_eq!(linked["status"], "linked", "{linked}");
    assert_eq!(
        h.found(&mut asker, "github", "octo-cat").await,
        Some(first.id())
    );
    // Renamed on GitHub and signed in again: the old login, which GitHub may
    // give to someone else, finds nobody.
    h.link(&first, "github", Account::GitHub(583_231, "Octo-Dog"))
        .await;
    assert_eq!(
        h.found(&mut asker, "github", "octo-dog").await,
        Some(first.id())
    );
    assert_eq!(h.found(&mut asker, "github", "octo-cat").await, None);
    // The same account signs in for another profile: it moves.
    h.link(&second, "github", Account::GitHub(583_231, "Octo-Dog"))
        .await;
    assert_eq!(
        h.found(&mut asker, "github", "octo-dog").await,
        Some(second.id())
    );
}

// --- cards ----------------------------------------------------------------------

fn group_card(name: &str, about: &str, tags: &[&str], langs: &[&str]) -> Card {
    Card::Group {
        group_id: [44; 32],
        name: name.into(),
        about: about.into(),
        tags: tags.iter().map(|t| (*t).to_owned()).collect(),
        langs: langs.iter().map(|l| (*l).to_owned()).collect(),
    }
}

#[tokio::test]
async fn a_channels_card_is_found_as_a_channel() {
    let h = Harness::start().await;
    let mut owner = Profile::new(1, &h.ledger);
    let searcher = Profile::new(2, &h.ledger);
    let group = group_card("Rustaceans", "Rust and agents", &["rust"], &["de"]);
    assert_eq!(publish(&h, &mut owner, &group, 10).await.1, 201);
    // A node before channels published it as a group: the channel's card
    // takes its place.
    let stale = Card::Group {
        group_id: [45; 32],
        name: "Rust News".into(),
        about: "The week in review".into(),
        tags: vec!["rust".into()],
        langs: vec!["de".into()],
    };
    assert_eq!(publish(&h, &mut owner, &stale, 10).await.1, 201);
    let news = Card::Channel {
        group_id: [45; 32],
        name: "Rust News".into(),
        about: "The week in review".into(),
        tags: vec!["rust".into()],
        langs: vec!["de".into()],
    };
    let (_, status, published) = publish(&h, &mut owner, &news, 10).await;
    assert_eq!(status, 201, "{published}");
    let (_, channels) = search(
        &h,
        &searcher,
        json!({"query": "rust", "kind": "channel"}),
        1,
    )
    .await;
    assert_eq!(names(&channels), ["Rust News"]);
    let (_, groups) = search(&h, &searcher, json!({"query": "rust", "kind": "group"}), 2).await;
    assert_eq!(names(&groups), ["Rustaceans"]);
}

/// What publishing `card` by `author`, signed at `signed_at`, sends: paid
/// with `stamps` of the ten it needs.
fn card_request(author: &mut Profile, card: &Card, signed_at: u64, stamps: usize) -> Value {
    let wire = author.signed(Signed::Card(card.clone()).encode(), signed_at);
    let id: [u8; 32] = Sha256::digest(hex::decode(&wire).unwrap()).into();
    let stamps: Vec<Value> = (0..stamps)
        .map(|i| author.stamp(card_operation(&DOMAIN, &id, i as u32)))
        .collect();
    json!({"card": wire, "stamps": stamps, "grants": author.grants()})
}

/// Publish `card` by `author`, paying with `stamps` of the ten it needs; the
/// card's wire, and the answer.
async fn publish(
    h: &Harness,
    author: &mut Profile,
    card: &Card,
    stamps: usize,
) -> (String, u16, Value) {
    let request = card_request(author, card, h.now(), stamps);
    let (status, body) = h.call("POST", "/v1/cards", Some(request.clone())).await;
    (request["card"].as_str().unwrap().to_owned(), status, body)
}

async fn search(h: &Harness, searcher: &Profile, query: Value, nonce: u8) -> (u16, Value) {
    let mut query = query;
    query["pass"] = searcher.pass([nonce; 32], h.now());
    h.call("POST", "/v1/search", Some(query)).await
}

fn names(found: &Value) -> Vec<String> {
    found["cards"]
        .as_array()
        .unwrap_or_else(|| panic!("{found}"))
        .iter()
        .map(|c| {
            let wire = hex::decode(c["card"].as_str().unwrap()).unwrap();
            // Read a year on: some cards are signed days after NOW.
            let document =
                agentic_protocol::VerifiedDocument::decode(&wire, DOMAIN, NOW + 365 * DAY).unwrap();
            match Signed::decode(document.body()).unwrap() {
                Signed::Card(card) => card.name().to_owned(),
                _ => panic!("not a card"),
            }
        })
        .collect()
}

#[tokio::test]
async fn a_card_costs_ten_stamps_lives_thirty_days_and_is_found_by_interest() {
    let h = Harness::start().await;
    let mut owner = Profile::new(1, &h.ledger);
    let searcher = Profile::new(2, &h.ledger);
    let rustaceans = group_card(
        "Rustaceans",
        "Rust and agents on weekends",
        &["rust", "agents"],
        &["de"],
    );
    let (_, status, body) = publish(&h, &mut owner, &rustaceans, 9).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (402, Some("payment_required"))
    );
    // A card that does not hold its own rules is refused.
    let loud = group_card("Loud", "x", &["UPPER CASE"], &[]);
    assert_eq!(
        publish(&h, &mut owner, &loud, 10).await.2["error"],
        "bad_card"
    );
    // Refused, nothing was spent.
    assert_eq!(h.redeemed(), 0);
    let (wire, status, published) = publish(&h, &mut owner, &rustaceans, 10).await;
    assert_eq!(status, 201, "{published}");
    assert_eq!(h.redeemed(), 10);
    assert_eq!(published["expiresAt"], h.now() + 30 * DAY);
    let id = published["id"].as_str().unwrap().to_owned();
    let (status, one) = h.call("GET", &format!("/v1/cards/{id}"), None).await;
    assert_eq!(status, 200, "{one}");
    assert_eq!(
        (one["card"].as_str(), &one["expiresAt"]),
        (Some(wire.as_str()), &published["expiresAt"])
    );
    let mut bard = Profile::new(3, &h.ledger);
    let poets = Card::Profile {
        name: "Bard".into(),
        about: "I write poems on request".into(),
        tags: vec!["poetry".into()],
        langs: vec!["de".into(), "en".into()],
    };
    assert_eq!(publish(&h, &mut bard, &poets, 10).await.1, 201);
    // Found by words, tag, language and kind.
    let (status, found) = search(&h, &searcher, json!({"query": "agents"}), 1).await;
    assert_eq!(status, 200, "{found}");
    assert_eq!(names(&found), ["Rustaceans"]);
    assert_eq!(
        names(
            &search(&h, &searcher, json!({"query": "", "tag": "poetry"}), 2)
                .await
                .1
        ),
        ["Bard"]
    );
    assert_eq!(
        names(
            &search(&h, &searcher, json!({"query": "", "lang": "en"}), 3)
                .await
                .1
        ),
        ["Bard"]
    );
    assert_eq!(
        names(
            &search(&h, &searcher, json!({"query": "", "kind": "group"}), 4)
                .await
                .1
        ),
        ["Rustaceans"]
    );
    // Published again, the owner's card for the group replaces the old one.
    let renamed = group_card("Rust Club", "Rust and agents", &["rust"], &["de"]);
    assert_eq!(publish(&h, &mut owner, &renamed, 10).await.1, 201);
    assert_eq!(
        names(&search(&h, &searcher, json!({"query": "rust"}), 5).await.1),
        ["Rust Club"]
    );
    assert_eq!(h.call("GET", &format!("/v1/cards/{id}"), None).await.0, 404);
    // Only its author withdraws a card.
    let (_, found) = search(&h, &searcher, json!({"query": "Bard"}), 6).await;
    let bard_card = found["cards"][0]["id"].as_str().unwrap().to_owned();
    let by_owner = owner.signed(
        Signed::Withdraw {
            card: bard_card.clone(),
        }
        .encode(),
        h.now(),
    );
    let (status, body) = h
        .call(
            "POST",
            "/v1/withdraw",
            Some(json!({ "withdrawal": by_owner })),
        )
        .await;
    assert_eq!((status, body["error"].as_str()), (400, Some("not_found")));
    let by_bard = bard.signed(
        Signed::Withdraw {
            card: bard_card.clone(),
        }
        .encode(),
        h.now(),
    );
    let (status, body) = h
        .call(
            "POST",
            "/v1/withdraw",
            Some(json!({ "withdrawal": by_bard })),
        )
        .await;
    assert_eq!(status, 200, "{body}");
    assert!(names(&search(&h, &searcher, json!({"query": "Bard"}), 7).await.1).is_empty());
    assert_eq!(
        h.call("GET", &format!("/v1/cards/{bard_card}"), None)
            .await
            .0,
        404
    );
    // Thirty days on, the card is gone unless published again.
    h.now.fetch_add(30 * DAY, Ordering::SeqCst);
    assert!(names(&search(&h, &searcher, json!({"query": "rust"}), 8).await.1).is_empty());
}

/// A card is paid for once: showing it again, as a retry does, answers
/// the same and spends nothing, but neither renews it, nor lifts it above
/// newer cards, nor brings it back once replaced or withdrawn.
#[tokio::test]
async fn a_card_shown_again_neither_renews_nor_rises_nor_returns() {
    let h = Harness::start().await;
    let mut owner = Profile::new(1, &h.ledger);
    let mut other = Profile::new(3, &h.ledger);
    let searcher = Profile::new(2, &h.ledger);
    let rustaceans = group_card("Rustaceans", "Rust", &["rust"], &[]);
    let request = card_request(&mut owner, &rustaceans, h.now(), 10);
    let (status, published) = h.call("POST", "/v1/cards", Some(request.clone())).await;
    assert_eq!(status, 201, "{published}");
    // A retry at once: the same card, nothing more spent.
    let (status, again) = h.call("POST", "/v1/cards", Some(request.clone())).await;
    assert_eq!((status, &again), (200, &published));
    assert_eq!(h.redeemed(), 10);
    // Ten days on another card is published; the first shown again keeps
    // its end and its place below the newer one.
    h.now.fetch_add(10 * DAY, Ordering::SeqCst);
    let crabs = Card::Profile {
        name: "Crab".into(),
        about: "Rust at work".into(),
        tags: vec!["rust".into()],
        langs: vec![],
    };
    assert_eq!(publish(&h, &mut other, &crabs, 10).await.1, 201);
    let (status, again) = h.call("POST", "/v1/cards", Some(request.clone())).await;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["expiresAt"], published["expiresAt"]);
    assert_eq!(h.redeemed(), 20);
    assert_eq!(
        names(&search(&h, &searcher, json!({"query": "rust"}), 1).await.1),
        ["Crab", "Rustaceans"]
    );
    // The owner renames the group's card; the old one shown again does not
    // take its place back.
    h.now.fetch_add(60, Ordering::SeqCst);
    let club = group_card("Rust Club", "Rust", &["rust"], &[]);
    let renamed = card_request(&mut owner, &club, h.now(), 10);
    let (status, current) = h.call("POST", "/v1/cards", Some(renamed.clone())).await;
    assert_eq!(status, 201, "{current}");
    let (status, body) = h.call("POST", "/v1/cards", Some(request)).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (410, Some("card_ended")),
        "{body}"
    );
    assert_eq!(
        names(&search(&h, &searcher, json!({"query": "rust"}), 2).await.1),
        ["Rust Club", "Crab"]
    );
    // Withdrawn, the renamed card does not come back either.
    let id = current["id"].as_str().unwrap().to_owned();
    let withdrawal = owner.signed(Signed::Withdraw { card: id }.encode(), h.now());
    let (status, _) = h
        .call(
            "POST",
            "/v1/withdraw",
            Some(json!({ "withdrawal": withdrawal })),
        )
        .await;
    assert_eq!(status, 200);
    let (status, body) = h.call("POST", "/v1/cards", Some(renamed)).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (410, Some("card_ended")),
        "{body}"
    );
    assert_eq!(
        names(&search(&h, &searcher, json!({"query": "rust"}), 3).await.1),
        ["Crab"]
    );
    // A card first shown longer than a link's life after it was signed is
    // refused, unpaid.
    let spent = h.redeemed();
    let old = card_request(&mut owner, &rustaceans, h.now() - 16 * 60, 10);
    let (status, body) = h.call("POST", "/v1/cards", Some(old)).await;
    assert_eq!((status, body["error"].as_str()), (400, Some("bad_card")));
    assert_eq!(h.redeemed(), spent);
}

#[tokio::test]
async fn searching_is_for_owners_of_an_active_book_and_a_pass_is_taken_once() {
    let h = Harness::start().await;
    let mut owner = Profile::new(1, &h.ledger);
    let (_, status, _) = publish(
        &h,
        &mut owner,
        &group_card("Rustaceans", "Rust", &["rust"], &[]),
        10,
    )
    .await;
    assert_eq!(status, 201);
    // A binding is not a card: searching finds only what was published.
    let ann = Profile::new(5, &h.ledger);
    h.link(&ann, "google", Account::Google("ann@example.org", true))
        .await;
    let searcher = Profile::new(2, &h.ledger);
    let query = json!({"query": "rust"});
    let (status, found) = search(&h, &searcher, query.clone(), 1).await;
    assert_eq!(
        (status, names(&found)),
        (200, vec!["Rustaceans".to_owned()])
    );
    for (nonce, words) in [(11, "ann"), (12, "example.org"), (13, ann.id().as_str())] {
        let (_, found) = search(&h, &searcher, json!({ "query": words }), nonce).await;
        assert!(names(&found).is_empty(), "{words}");
    }
    // The same pass again is refused.
    let (status, body) = search(&h, &searcher, query.clone(), 1).await;
    assert_eq!((status, body["error"].as_str()), (401, Some("bad_pass")));
    // No pass, a pass of an ended or blocked book, a pass signed by a key
    // that is not the book's: nothing is found.
    let (status, body) = h.call("POST", "/v1/search", Some(query.clone())).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (401, Some("book_required"))
    );
    let ended = Profile::new(6, &h.ledger);
    h.ledger
        .books
        .lock()
        .unwrap()
        .get_mut(&ended.book_id)
        .unwrap()
        .valid_until = h.now();
    let (status, body) = search(&h, &ended, query.clone(), 2).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (401, Some("book_required"))
    );
    h.ledger
        .books
        .lock()
        .unwrap()
        .get_mut(&searcher.book_id)
        .unwrap()
        .blocked = true;
    let (status, body) = search(&h, &searcher, query.clone(), 3).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (401, Some("book_required"))
    );
    let impostor = Profile {
        book: BookKey::from_bytes(&[77; 32]).unwrap(),
        ..Profile::new(3, &h.ledger)
    };
    let (status, body) = search(&h, &impostor, query.clone(), 4).await;
    assert_eq!((status, body["error"].as_str()), (401, Some("bad_pass")));
    // A newcomer searches with the grant it was given.
    let newcomer = Profile::granted(7);
    let (status, found) = search(&h, &newcomer, query, 5).await;
    assert_eq!(
        (status, names(&found)),
        (200, vec!["Rustaceans".to_owned()])
    );
}

/// Behind the service's proxy every searcher comes from one address: the
/// limit is each book's, so one searcher past it holds back nobody else.
#[tokio::test]
async fn searches_are_limited_by_book_and_to_a_few_words() {
    let h = Harness::start().await;
    let query = json!({"query": "rust"});
    let busy = Profile::new(1, &h.ledger);
    let other = Profile::new(2, &h.ledger);
    for n in 0..30 {
        assert_eq!(search(&h, &busy, query.clone(), n).await.0, 200);
    }
    let (status, body) = search(&h, &busy, query.clone(), 100).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (429, Some("rate_limited"))
    );
    assert_eq!(search(&h, &other, query, 101).await.0, 200);
    let (status, body) = search(&h, &other, json!({"query": "a b c d e f g h i"}), 102).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (400, Some("invalid_request"))
    );
}

#[tokio::test]
async fn bindings_cards_and_taken_passes_survive_a_restart() {
    let mut h = Harness::start().await;
    let ann = Profile::new(1, &h.ledger);
    let mut owner = Profile::new(2, &h.ledger);
    let mut asker = Profile::new(3, &h.ledger);
    h.link(&ann, "github", Account::GitHub(7, "ann")).await;
    let (_, status, _) = publish(
        &h,
        &mut owner,
        &group_card("Rustaceans", "Rust", &["rust"], &[]),
        10,
    )
    .await;
    assert_eq!(status, 201);
    assert_eq!(search(&h, &asker, json!({"query": "rust"}), 1).await.0, 200);
    h.restart().await;
    assert_eq!(h.found(&mut asker, "github", "ann").await, Some(ann.id()));
    assert_eq!(
        names(&search(&h, &asker, json!({"query": "rust"}), 2).await.1),
        ["Rustaceans"]
    );
    let (status, body) = search(&h, &asker, json!({"query": "rust"}), 1).await;
    assert_eq!((status, body["error"].as_str()), (401, Some("bad_pass")));
}
