//! The identity server over real HTTP. A local OpenID provider stands in for
//! Google's token endpoint and enforces the client secret (form or Basic),
//! redirect URI, PKCE verifier and single-use codes the way Google does.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use agentic_grant_book::{
    Account, ClaimRequest, GrantBook, GrantRevocation, GrantRules, GrantTerms, SECONDS_PER_DAY,
    SecpKey,
};
use agentic_identity_server::{
    Config, GitHubConfig, GoogleConfig, NetworkRules, RulesFuture, Server, StaticRules,
};
use agentic_mailbox_swarm::stamp::{BookKey, Stamp};
use axum::{
    Form, Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tempfile::TempDir;
use url::Url;

const DOMAIN: [u8; 32] = [0x5a; 32];
const SERVER_SECRET: [u8; 32] = [0x21; 32];
const CLIENT_ID: &str = "agentic-test.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "test-client-secret";
const AUTHORIZE: &str = "https://accounts.example.test/o/oauth2/v2/auth";
const GITHUB_CLIENT_ID: &str = "Iv1.agentic-test";
const GITHUB_CLIENT_SECRET: &str = "github-test-secret";
const GITHUB_AUTHORIZE: &str = "https://github.example.test/login/oauth/authorize";
const DAY: u64 = 20_721;
const TEN_AM: u64 = DAY * SECONDS_PER_DAY + 10 * 3_600;
const TTL: u64 = 900;
const GRANT_VALIDITY_DAYS: u64 = 14;
const MAX_VALIDITY_DAYS: u64 = 30;
const CLAIM_INTERVAL_DAYS: u64 = 30;

type Edit = (&'static str, fn(&mut Value, u64));

struct Issued {
    challenge: String,
    redirect_uri: String,
    claims: Value,
}

type Provider = Arc<Mutex<HashMap<String, Issued>>>;

fn client_authenticated(headers: &HeaderMap, form: &HashMap<String, String>) -> bool {
    let basic = format!(
        "Basic {}",
        STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"))
    );
    let in_header = headers
        .get("authorization")
        .is_some_and(|value| value.to_str().ok() == Some(basic.as_str()));
    let in_form = form.get("client_id").map(String::as_str) == Some(CLIENT_ID)
        && form.get("client_secret").map(String::as_str) == Some(CLIENT_SECRET);
    in_header || in_form
}

async fn token(
    State(provider): State<Provider>,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let refuse = |reason: &str| (StatusCode::BAD_REQUEST, Json(json!({"error": reason})));
    if form.get("grant_type").map(String::as_str) != Some("authorization_code") {
        return refuse("unsupported_grant_type");
    }
    if !client_authenticated(&headers, &form) {
        return refuse("invalid_client");
    }
    let Some(issued) = form
        .get("code")
        .and_then(|code| provider.lock().unwrap().remove(code))
    else {
        return refuse("invalid_grant");
    };
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    if form.get("redirect_uri") != Some(&issued.redirect_uri)
        || URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != issued.challenge
    {
        return refuse("invalid_grant");
    }
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    let id_token = format!(
        "{}.{}.{}",
        part(&json!({"alg":"RS256","kid":"test","typ":"JWT"})),
        part(&issued.claims),
        URL_SAFE_NO_PAD.encode(b"provider-signature")
    );
    (
        StatusCode::OK,
        Json(
            json!({"access_token":"access","token_type":"Bearer","expires_in":3599,
            "scope":"openid email","id_token":id_token}),
        ),
    )
}

/// A GitHub account as its API answers: the numeric id and its emails.
#[derive(Clone)]
struct GitHubUser {
    id: u64,
    /// The name, which the account's owner can change at any time.
    login: String,
    emails: Value,
}

#[derive(Default)]
struct GitHub {
    /// Codes GitHub issued: the PKCE challenge, redirect URI and account.
    codes: HashMap<String, (String, String, GitHubUser)>,
    tokens: HashMap<String, GitHubUser>,
}

type GitHubState = Arc<Mutex<GitHub>>;

/// GitHub's token endpoint: the client secret, redirect URI, PKCE verifier and
/// single-use codes, answered as JSON (the server asks for it).
async fn github_token(
    State(github): State<GitHubState>,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let refuse = |reason: &str| (StatusCode::OK, Json(json!({"error": reason})));
    // Without it GitHub answers form-encoded, which the server would not read.
    if !headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|accept| accept.contains("application/json"))
    {
        return (StatusCode::NOT_ACCEPTABLE, Json(json!({})));
    }
    if form.get("client_id").map(String::as_str) != Some(GITHUB_CLIENT_ID)
        || form.get("client_secret").map(String::as_str) != Some(GITHUB_CLIENT_SECRET)
    {
        return refuse("incorrect_client_credentials");
    }
    let mut github = github.lock().unwrap();
    let Some((challenge, redirect_uri, user)) =
        form.get("code").and_then(|code| github.codes.remove(code))
    else {
        return refuse("bad_verification_code");
    };
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    if form.get("redirect_uri") != Some(&redirect_uri)
        || URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != challenge
    {
        return refuse("bad_verification_code");
    }
    let token = format!("gho_{}", github.tokens.len());
    github.tokens.insert(token.clone(), user);
    (
        StatusCode::OK,
        Json(
            json!({"access_token": token, "token_type": "bearer", "scope": "read:user,user:email"}),
        ),
    )
}

/// The account behind a token; GitHub's API also refuses requests without a
/// User-Agent.
fn github_account(github: &GitHubState, headers: &HeaderMap) -> Option<GitHubUser> {
    headers.get("user-agent")?;
    let token = headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?;
    github.lock().unwrap().tokens.get(token).cloned()
}

async fn github_user(
    State(github): State<GitHubState>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    match github_account(&github, &headers) {
        Some(user) => (
            StatusCode::OK,
            Json(json!({"id": user.id, "login": user.login})),
        ),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"message": "Bad credentials"})),
        ),
    }
}

async fn github_emails(
    State(github): State<GitHubState>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    match github_account(&github, &headers) {
        Some(user) => (StatusCode::OK, Json(user.emails)),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"message": "Bad credentials"})),
        ),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Offline,
    Inactive,
    Active,
}

/// Network rules that can be taken offline or report the issuer inactive,
/// as a GrantIssuer reader can. Records which issuer the server asked about.
struct TestRules {
    mode: Mutex<Mode>,
    asked: Mutex<Vec<(Account, u64)>>,
}

impl NetworkRules for TestRules {
    fn rules(&self, server: Account, day: u64) -> RulesFuture<'_> {
        self.asked.lock().unwrap().push((server, day));
        let mode = *self.mode.lock().unwrap();
        Box::pin(async move {
            if mode == Mode::Offline {
                return Err("rpc unavailable".to_owned());
            }
            Ok(GrantRules {
                domain: DOMAIN,
                issuer_active: mode == Mode::Active,
                cap_coins: 100,
                book_size: 50,
                max_validity_days: MAX_VALIDITY_DAYS,
            })
        })
    }
}

struct Harness {
    dir: TempDir,
    now: Arc<AtomicU64>,
    provider: Provider,
    token_url: String,
    github: GitHubState,
    github_url: String,
    /// Whether the server offers GitHub besides Google.
    with_github: bool,
    claim_interval_days: u64,
    rules: Arc<dyn NetworkRules>,
    server: Option<Server>,
    http: reqwest::Client,
    codes: AtomicU64,
}

impl Harness {
    async fn start(caps: BTreeMap<u64, u64>) -> Self {
        Self::with_rules(Arc::new(StaticRules {
            domain: DOMAIN,
            caps,
            book_size: 50,
            max_validity_days: MAX_VALIDITY_DAYS,
        }))
        .await
    }

    /// A server that offers only Google.
    async fn google_only(caps: BTreeMap<u64, u64>) -> Self {
        let mut harness = Self::start(caps).await;
        harness.with_github = false;
        harness.restart().await;
        harness
    }

    async fn with_rules(rules: Arc<dyn NetworkRules>) -> Self {
        let provider: Provider = Arc::default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let token_url = format!("http://{}/token", listener.local_addr().unwrap());
        let github_url = format!("http://{}/github", listener.local_addr().unwrap());
        let github: GitHubState = Arc::default();
        let app = Router::new()
            .route("/token", post(token))
            .with_state(provider.clone())
            .merge(
                Router::new()
                    .route("/github/login/oauth/access_token", post(github_token))
                    .route("/github/api/user", get(github_user))
                    .route("/github/api/user/emails", get(github_emails))
                    .with_state(github.clone()),
            );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let mut harness = Self {
            dir: TempDir::new().unwrap(),
            now: Arc::new(AtomicU64::new(TEN_AM)),
            provider,
            token_url,
            github,
            github_url,
            with_github: true,
            claim_interval_days: CLAIM_INTERVAL_DAYS,
            rules,
            server: None,
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            codes: AtomicU64::new(0),
        };
        harness.launch().await;
        harness
    }

    fn config(&self, public_url: String, database: &str) -> Config {
        let now = self.now.clone();
        Config {
            public_url,
            domain: DOMAIN,
            signing_secret: SERVER_SECRET,
            database: self.dir.path().join(database),
            google: GoogleConfig {
                client_id: CLIENT_ID.into(),
                client_secret: CLIENT_SECRET.into(),
                authorize_url: AUTHORIZE.into(),
                token_url: self.token_url.clone(),
            },
            github: self.with_github.then(|| GitHubConfig {
                client_id: GITHUB_CLIENT_ID.into(),
                client_secret: GITHUB_CLIENT_SECRET.into(),
                authorize_url: GITHUB_AUTHORIZE.into(),
                token_url: format!("{}/login/oauth/access_token", self.github_url),
                api_url: format!("{}/api", self.github_url),
            }),
            rules: self.rules.clone(),
            grant_validity_days: GRANT_VALIDITY_DAYS,
            claim_interval_days: self.claim_interval_days,
            claim_ttl_secs: TTL,
            clock: Arc::new(move || now.load(Ordering::SeqCst)),
        }
    }

    async fn launch(&mut self) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let config = self.config(url, "identity.db");
        self.server = Some(Server::start(config, listener).await.unwrap());
    }

    async fn restart(&mut self) {
        self.server.take().unwrap().shutdown().await;
        self.launch().await;
    }

    fn base(&self) -> String {
        format!("http://{}", self.server.as_ref().unwrap().addr)
    }

    fn now(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }

    fn advance(&self, seconds: u64) {
        self.now.fetch_add(seconds, Ordering::SeqCst);
    }

    fn request(&self, book: &SecpKey, nonce: u8) -> ClaimRequest {
        ClaimRequest::sign(DOMAIN, [nonce; 16], self.now(), book)
    }

    async fn submit(&self, request: &ClaimRequest) -> (StatusCode, Value) {
        let response = self
            .http
            .post(format!("{}/v1/claims", self.base()))
            .json(&json!({ "request": request }))
            .send()
            .await
            .unwrap();
        let status = StatusCode::from_u16(response.status().as_u16()).unwrap();
        (status, response.json().await.unwrap())
    }

    async fn claim(&self, book: &SecpKey, nonce: u8) -> String {
        let (status, body) = self.submit(&self.request(book, nonce)).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["claimId"].as_str().unwrap().to_owned()
    }

    async fn status(&self, claim: &str) -> Value {
        let response = self
            .http
            .get(format!("{}/v1/claims/{claim}", self.base()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        response.json().await.unwrap()
    }

    async fn policy(&self) -> Value {
        self.http
            .get(format!("{}/v1/policy", self.base()))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    /// Follows the login link's Google choice like a browser and returns
    /// Google's parameters.
    async fn authorize(&self, claim: &str) -> HashMap<String, String> {
        let response = self
            .http
            .get(format!("{}/v1/claims/{claim}/login/google", self.base()))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_redirection(), "{}", response.status());
        let location = response.headers()["location"].to_str().unwrap().to_owned();
        assert!(location.starts_with(AUTHORIZE), "{location}");
        Url::parse(&location)
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect()
    }

    /// The user signs in at Google as `claims`; returns the code Google will
    /// redirect back with.
    fn sign_in_at_google(&self, params: &HashMap<String, String>, claims: Value) -> String {
        let code = format!("code-{}", self.codes.fetch_add(1, Ordering::SeqCst));
        self.provider.lock().unwrap().insert(
            code.clone(),
            Issued {
                challenge: params["code_challenge"].clone(),
                redirect_uri: params["redirect_uri"].clone(),
                claims,
            },
        );
        code
    }

    async fn callback(&self, pairs: &[(&str, &str)]) -> (u16, String) {
        let url =
            Url::parse_with_params(&format!("{}/v1/oauth/google/callback", self.base()), pairs)
                .unwrap();
        let response = self.http.get(url).send().await.unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    }

    async fn sign_in(&self, params: &HashMap<String, String>, claims: Value) -> (u16, String) {
        let code = self.sign_in_at_google(params, claims);
        self.callback(&[("code", &code), ("state", &params["state"])])
            .await
    }

    fn google(&self, sub: &str, nonce: &str) -> Value {
        json!({"iss":"https://accounts.google.com","aud":CLIENT_ID,"azp":CLIENT_ID,"sub":sub,
            "email":format!("{sub}@gmail.com"),"email_verified":true,
            "iat":self.now(),"exp":self.now() + 3_600,"nonce":nonce})
    }

    /// A holder reports a double spend.
    async fn report(&self, body: &Value) -> (StatusCode, Value) {
        let response = self
            .http
            .post(format!("{}/v1/reports", self.base()))
            .json(body)
            .send()
            .await
            .unwrap();
        let status = StatusCode::from_u16(response.status().as_u16()).unwrap();
        (status, response.json().await.unwrap())
    }

    /// The revocations listed after the `after`-th.
    async fn revocations(&self, after: u64) -> Value {
        let response = self
            .http
            .get(format!("{}/v1/revocations?after={after}", self.base()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
        response.json().await.unwrap()
    }

    /// Claim with a fresh book, sign in as Google account `sub`, return the status.
    async fn claim_as(&self, sub: &str, nonce: u8) -> Value {
        let claim = self.claim(&book(nonce), nonce).await;
        let params = self.authorize(&claim).await;
        self.sign_in(&params, self.google(sub, &params["nonce"]))
            .await;
        self.status(&claim).await
    }
}

impl Harness {
    /// Follows the login link's GitHub choice and returns GitHub's parameters.
    async fn authorize_github(&self, claim: &str) -> HashMap<String, String> {
        let response = self
            .http
            .get(format!("{}/v1/claims/{claim}/login/github", self.base()))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_redirection(), "{}", response.status());
        let location = response.headers()["location"].to_str().unwrap().to_owned();
        assert!(location.starts_with(GITHUB_AUTHORIZE), "{location}");
        Url::parse(&location)
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect()
    }

    /// The user signs in at GitHub as `user`; returns the code GitHub will
    /// redirect back with.
    fn sign_in_at_github(&self, params: &HashMap<String, String>, user: GitHubUser) -> String {
        let code = format!("gh-code-{}", self.codes.fetch_add(1, Ordering::SeqCst));
        self.github.lock().unwrap().codes.insert(
            code.clone(),
            (
                params["code_challenge"].clone(),
                params["redirect_uri"].clone(),
                user,
            ),
        );
        code
    }

    /// GitHub's redirect URI for this server.
    fn github_redirect(&self) -> String {
        format!("{}/v1/oauth/github/callback", self.base())
    }

    async fn github_callback(&self, pairs: &[(&str, &str)]) -> (u16, String) {
        let url =
            Url::parse_with_params(&format!("{}/v1/oauth/github/callback", self.base()), pairs)
                .unwrap();
        let response = self.http.get(url).send().await.unwrap();
        (response.status().as_u16(), response.text().await.unwrap())
    }

    /// Claim with a fresh book, sign in at GitHub as `user`, return the status.
    async fn claim_as_github(&self, user: GitHubUser, nonce: u8) -> Value {
        let claim = self.claim(&book(nonce), nonce).await;
        let params = self.authorize_github(&claim).await;
        let code = self.sign_in_at_github(&params, user);
        self.github_callback(&[("code", &code), ("state", &params["state"])])
            .await;
        self.status(&claim).await
    }
}

/// A GitHub account with one primary email, verified or not.
fn octocat(id: u64, verified: bool) -> GitHubUser {
    GitHubUser {
        id,
        login: format!("user{id}"),
        emails: json!([
            {"email": format!("{id}@users.noreply.github.com"), "primary": false, "verified": true, "visibility": null},
            {"email": format!("user{id}@example.org"), "primary": true, "verified": verified, "visibility": "private"},
        ]),
    }
}

fn book(seed: u8) -> SecpKey {
    SecpKey::from_secret(&[seed.max(1); 32]).unwrap()
}

/// A stamp spending slot `index` of `grant`'s book on `operation`, signed
/// by the key of `book(seed)`.
fn stamp(grant: &GrantBook, seed: u8, index: u32, operation: u8) -> Value {
    let key = BookKey::from_bytes(&[seed.max(1); 32]).unwrap();
    let stamp = Stamp::sign(&DOMAIN, grant.id(), index, [operation; 32], &key);
    // The stamp as holders show it over HTTP (the directory's form).
    json!({"book": hex::encode(stamp.book), "index": stamp.index,
        "operation": hex::encode(stamp.operation), "signature": hex::encode(stamp.signature)})
}

/// Two stamps of one slot of `grant` for different operations.
fn double_spend(grant: &GrantBook, seed: u8) -> Value {
    json!({"grant": grant, "first": stamp(grant, seed, 5, 1), "second": stamp(grant, seed, 5, 2)})
}

fn revocations(listed: &Value) -> Vec<GrantRevocation> {
    serde_json::from_value(listed["revocations"].clone()).unwrap()
}

fn caps(entries: &[(u64, u64)]) -> BTreeMap<u64, u64> {
    entries.iter().copied().collect()
}

fn grant(status: &Value) -> GrantBook {
    assert_eq!(status["status"], "granted", "{status}");
    serde_json::from_value(status["grant"].clone()).unwrap()
}

fn pending(status: &Value) {
    assert_eq!(status["status"], "pending", "{status}");
    assert!(status["grant"].is_null());
}

fn denied(status: &Value, reason: &str) {
    assert_eq!(status["status"], "denied", "{status}");
    assert_eq!(status["reason"], reason, "{status}");
    assert!(status["grant"].is_null());
}

#[tokio::test]
async fn a_verified_google_account_gets_a_grant_the_network_accepts() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let book = book(7);
    let (status, body) = h.submit(&h.request(&book, 7)).await;
    assert_eq!(status, StatusCode::CREATED);
    let claim = body["claimId"].as_str().unwrap().to_owned();
    assert_eq!(
        body["loginUrl"],
        json!(format!("{}/v1/claims/{claim}/login", h.base()))
    );
    assert_eq!(body["expiresAt"], json!(h.now() + TTL));
    pending(&h.status(&claim).await);

    let params = h.authorize(&claim).await;
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["client_id"], CLIENT_ID);
    assert_eq!(
        params["redirect_uri"],
        format!("{}/v1/oauth/google/callback", h.base())
    );
    let scopes: Vec<_> = params["scope"].split(' ').collect();
    assert!(scopes.contains(&"openid") && scopes.contains(&"email"));
    assert_eq!(params["code_challenge_method"], "S256");
    assert!(params["state"].len() >= 32 && params["nonce"].len() >= 32);

    let (code, _) = h
        .sign_in(&params, h.google("alice", &params["nonce"]))
        .await;
    assert_eq!(code, 200);
    let grant = grant(&h.status(&claim).await);
    assert_eq!(grant.book, book.account());
    assert_eq!(
        grant.server,
        SecpKey::from_secret(&SERVER_SECRET).unwrap().account()
    );
    assert_eq!(grant.domain, DOMAIN);
    assert_eq!((grant.day, grant.serial, grant.count), (DAY, 0, 50));
    assert_eq!(
        grant.expiry,
        h.now() + GRANT_VALIDITY_DAYS * SECONDS_PER_DAY
    );
    let rules = GrantRules {
        domain: DOMAIN,
        issuer_active: true,
        cap_coins: 100,
        book_size: 50,
        max_validity_days: MAX_VALIDITY_DAYS,
    };
    assert_eq!(grant.check(&rules, h.now()), Ok(()));
    assert_eq!(
        h.policy().await,
        json!({"domain":format!("0x{}", hex::encode(DOMAIN)),
            "server":format!("0x{}", hex::encode(grant.server)),"day":DAY,"bookSize":50,
            "capCoins":100,"grantsIssued":1,"maxValidityDays":MAX_VALIDITY_DAYS})
    );
}

#[tokio::test]
async fn identity_tokens_google_did_not_vouch_for_are_denied_without_spending_a_serial() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let edits: [Edit; 6] = [
        ("email_not_verified", |c, _| {
            c["email_verified"] = json!(false)
        }),
        ("invalid_identity_token", |c, _| {
            c["aud"] = json!("someone-else.apps.googleusercontent.com")
        }),
        ("invalid_identity_token", |c, _| {
            c["iss"] = json!("https://accounts.example.test")
        }),
        ("invalid_identity_token", |c, now| {
            c["exp"] = json!(now - 3_600)
        }),
        ("invalid_identity_token", |c, _| {
            c["nonce"] = json!("replayed-nonce")
        }),
        ("invalid_identity_token", |c, _| {
            c.as_object_mut().unwrap().remove("sub");
        }),
    ];
    for (n, (reason, edit)) in (1u8..).zip(edits) {
        let claim = h.claim(&book(n), n).await;
        let params = h.authorize(&claim).await;
        let mut claims = h.google(&format!("user{n}"), &params["nonce"]);
        edit(&mut claims, h.now());
        let (code, _) = h.sign_in(&params, claims).await;
        assert_eq!(code, 403, "{reason}");
        denied(&h.status(&claim).await, reason);
    }
    assert_eq!(h.policy().await["grantsIssued"], 0);
    assert_eq!(grant(&h.claim_as("user1", 20).await).serial, 0);
}

#[tokio::test]
async fn a_google_account_is_granted_once_per_interval() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    assert_eq!(grant(&h.claim_as("alice", 1).await).serial, 0);
    // The same Google account under another address is still the same account.
    let claim = h.claim(&book(2), 2).await;
    let params = h.authorize(&claim).await;
    let mut renamed = h.google("alice", &params["nonce"]);
    renamed["email"] = json!("alice.renamed@example.org");
    h.sign_in(&params, renamed).await;
    denied(&h.status(&claim).await, "already_claimed");
    assert_eq!(grant(&h.claim_as("bob", 3).await).serial, 1);
    h.advance((CLAIM_INTERVAL_DAYS - 1) * SECONDS_PER_DAY);
    denied(&h.claim_as("alice", 4).await, "already_claimed");
    h.advance(SECONDS_PER_DAY);
    let again = grant(&h.claim_as("alice", 5).await);
    assert_eq!((again.day, again.serial), (DAY + CLAIM_INTERVAL_DAYS, 0));
}

#[tokio::test]
async fn the_daily_cap_stops_grants_until_the_next_utc_day_and_follows_the_schedule() {
    let h = Harness::start(caps(&[(DAY, 100), (DAY + 1, 150)])).await;
    assert_eq!(grant(&h.claim_as("a", 1).await).serial, 0);
    assert_eq!(grant(&h.claim_as("b", 2).await).serial, 1);
    denied(&h.claim_as("c", 3).await, "daily_cap_reached");
    h.advance(SECONDS_PER_DAY);
    let policy = h.policy().await;
    assert_eq!(
        (&policy["day"], &policy["capCoins"], &policy["grantsIssued"]),
        (&json!(DAY + 1), &json!(150), &json!(0))
    );
    for (n, sub) in [(4, "c"), (5, "d"), (6, "e")] {
        let next = grant(&h.claim_as(sub, n).await);
        assert_eq!((next.day, next.serial), (DAY + 1, u32::from(n) - 4));
    }
    denied(&h.claim_as("f", 7).await, "daily_cap_reached");
    assert_eq!(h.policy().await["grantsIssued"], 3);
}

/// Claims once per Google account in `subs`, then fires every browser
/// callback at once. Claim `n` (from 1) uses `book(n)`.
async fn race(h: Arc<Harness>, subs: Vec<String>) -> Vec<String> {
    let mut claims = Vec::new();
    let mut ready = Vec::new();
    for (n, sub) in (1u8..).zip(&subs) {
        let claim = h.claim(&book(n), n).await;
        let params = h.authorize(&claim).await;
        let code = h.sign_in_at_google(&params, h.google(sub, &params["nonce"]));
        ready.push((code, params["state"].clone()));
        claims.push(claim);
    }
    let mut tasks = Vec::new();
    for (code, state) in ready {
        let h = h.clone();
        tasks.push(tokio::spawn(async move {
            h.callback(&[("code", &code), ("state", &state)]).await
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    claims
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_logins_never_exceed_the_cap_or_reuse_a_serial() {
    let h = Arc::new(Harness::start(caps(&[(DAY, 100)])).await);
    let claims = race(h.clone(), (1..=8).map(|n| format!("user{n}")).collect()).await;
    let mut serials = Vec::new();
    for (n, claim) in (1u8..).zip(&claims) {
        let status = h.status(claim).await;
        if status["status"] == "granted" {
            let grant = grant(&status);
            assert_eq!(
                grant.book,
                book(n).account(),
                "the grant went to its own claim"
            );
            serials.push(grant.serial);
        } else {
            denied(&status, "daily_cap_reached");
        }
    }
    serials.sort_unstable();
    assert_eq!(serials, [0, 1]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_google_account_logging_in_in_parallel_gets_one_grant() {
    let h = Arc::new(Harness::start(caps(&[(DAY, 1_000)])).await);
    let claims = race(h.clone(), vec!["alice".to_owned(); 6]).await;
    let mut granted = 0;
    for claim in &claims {
        let status = h.status(claim).await;
        if status["status"] == "granted" {
            granted += 1;
        } else {
            denied(&status, "already_claimed");
        }
    }
    assert_eq!(granted, 1);
    assert_eq!(h.policy().await["grantsIssued"], 1);
}

#[tokio::test]
async fn rules_that_cannot_be_read_or_an_inactive_key_grant_nothing() {
    let rules = Arc::new(TestRules {
        mode: Mutex::new(Mode::Offline),
        asked: Mutex::default(),
    });
    let h = Harness::with_rules(rules.clone()).await;
    let claim = h.claim(&book(1), 1).await;
    let params = h.authorize(&claim).await;
    let (code, _) = h
        .sign_in(&params, h.google("alice", &params["nonce"]))
        .await;
    assert_eq!(code, 503);
    // A transient failure leaves the claim open for another login.
    pending(&h.status(&claim).await);

    *rules.mode.lock().unwrap() = Mode::Inactive;
    let params = h.authorize(&claim).await;
    let (code, _) = h
        .sign_in(&params, h.google("alice", &params["nonce"]))
        .await;
    assert_eq!(code, 403);
    denied(&h.status(&claim).await, "issuer_inactive");

    *rules.mode.lock().unwrap() = Mode::Active;
    let later = grant(&h.claim_as("alice", 2).await);
    assert_eq!(
        later.serial, 0,
        "failures spent no serial and did not mark alice"
    );
    let server = SecpKey::from_secret(&SERVER_SECRET).unwrap().account();
    let asked = rules.asked.lock().unwrap().clone();
    assert!(!asked.is_empty());
    assert!(
        asked.iter().all(|entry| *entry == (server, DAY)),
        "rules are read for this issuer and the grant day"
    );
}

#[tokio::test]
async fn a_cancelled_or_failed_google_login_leaves_the_claim_open() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let claim = h.claim(&book(1), 1).await;
    let params = h.authorize(&claim).await;
    let (code, _) = h
        .callback(&[("error", "access_denied"), ("state", &params["state"])])
        .await;
    assert_ne!(code, 200);
    pending(&h.status(&claim).await);
    let params = h.authorize(&claim).await;
    let (code, _) = h
        .callback(&[("code", "code-never-issued"), ("state", &params["state"])])
        .await;
    assert_ne!(code, 200);
    pending(&h.status(&claim).await);
    let params = h.authorize(&claim).await;
    h.sign_in(&params, h.google("alice", &params["nonce"]))
        .await;
    assert_eq!(grant(&h.status(&claim).await).serial, 0);
}

#[tokio::test]
async fn grants_accounts_and_serials_survive_a_restart() {
    let mut h = Harness::start(caps(&[(DAY, 150)])).await;
    let book = book(1);
    let claim = h.claim(&book, 1).await;
    let params = h.authorize(&claim).await;
    h.sign_in(&params, h.google("alice", &params["nonce"]))
        .await;
    let before = grant(&h.status(&claim).await);
    h.restart().await;
    assert_eq!(grant(&h.status(&claim).await), before);
    denied(&h.claim_as("alice", 2).await, "already_claimed");
    assert_eq!(grant(&h.claim_as("bob", 3).await).serial, 1);
}

#[tokio::test]
async fn a_login_state_is_single_use() {
    let h = Harness::start(caps(&[(DAY, 150)])).await;
    let claim = h.claim(&book(1), 1).await;
    // The link was opened in two browser tabs before either finished.
    let params = h.authorize(&claim).await;
    let second_tab = h.authorize(&claim).await;
    let claims = h.google("alice", &params["nonce"]);
    assert_eq!(h.sign_in(&params, claims.clone()).await.0, 200);
    let first = grant(&h.status(&claim).await);
    // The other tab finishing, as another account, neither adds a grant to
    // this claim nor overwrites it.
    let bob = h.google("bob", &second_tab["nonce"]);
    assert_ne!(h.sign_in(&second_tab, bob).await.0, 200);
    assert_eq!(grant(&h.status(&claim).await), first);
    // A replayed redirect, even with a fresh valid code, grants nothing new.
    assert_ne!(h.sign_in(&params, claims).await.0, 200);
    assert_eq!(grant(&h.status(&claim).await), first);
    assert_ne!(
        h.callback(&[("code", "code-x"), ("state", "unknown-state")])
            .await
            .0,
        200
    );
    assert_eq!(h.policy().await["grantsIssued"], 1);
    assert_eq!(
        grant(&h.claim_as("bob", 2).await).serial,
        1,
        "bob was not marked by the other tab"
    );
}

#[tokio::test]
async fn claim_requests_must_be_signed_fresh_and_for_this_network() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let key = book(1);
    let mut forged = h.request(&key, 1);
    forged.book = book(2).account();
    let (status, body) = h.submit(&forged).await;
    assert_eq!(
        (status, body["error"].clone()),
        (StatusCode::BAD_REQUEST, json!("bad_signature"))
    );
    let elsewhere = ClaimRequest::sign([0x5b; 32], [2; 16], h.now(), &key);
    let (status, body) = h.submit(&elsewhere).await;
    assert_eq!(
        (status, body["error"].clone()),
        (StatusCode::BAD_REQUEST, json!("wrong_domain"))
    );
    for created_at in [h.now() - TTL - 1, h.now() + 3_600] {
        let stale = ClaimRequest::sign(DOMAIN, [3; 16], created_at, &key);
        let (status, body) = h.submit(&stale).await;
        assert_eq!(
            (status, body["error"].clone()),
            (StatusCode::BAD_REQUEST, json!("stale_request"))
        );
    }
    let request = h.request(&key, 4);
    let (first, one) = h.submit(&request).await;
    let (second, two) = h.submit(&request).await;
    assert_eq!(first, StatusCode::CREATED);
    assert!(second.is_success());
    assert_eq!(
        one["claimId"], two["claimId"],
        "a retried request is the same claim"
    );
}

#[tokio::test]
async fn an_unused_claim_expires() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let claim = h.claim(&book(1), 1).await;
    h.advance(TTL + 1);
    // The CLI polling an unopened link learns it expired.
    denied(&h.status(&claim).await, "claim_expired");
    for path in ["login", "login/google", "login/github"] {
        let response = h
            .http
            .get(format!("{}/v1/claims/{claim}/{path}", h.base()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 410, "{path}");
    }
}

#[tokio::test]
async fn the_identity_token_must_come_over_tls_or_loopback() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = h.config("https://id.example.test".into(), "other.db");
    config.google.token_url = "http://oauth2.example.test/token".into();
    assert!(matches!(
        Server::start(config, listener).await,
        Err(agentic_identity_server::Error::Config(_))
    ));
    // GitHub's answers carry no signature: its token and API endpoints need
    // TLS (or loopback) too.
    for field in ["token", "api"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut config = h.config("https://id.example.test".into(), &format!("{field}.db"));
        let github = config.github.as_mut().unwrap();
        match field {
            "token" => {
                github.token_url = "http://github.example.test/login/oauth/access_token".into()
            }
            _ => github.api_url = "http://api.github.example.test".into(),
        }
        assert!(
            matches!(
                Server::start(config, listener).await,
                Err(agentic_identity_server::Error::Config(_))
            ),
            "{field}"
        );
    }
}

/// Holds an identity server open for a GUI acceptance run (V1-GF01): a
/// stand-in Google signs every visitor in as a new verified account (its
/// authorize page redirects straight back with a code). The network's
/// GrantIssuer rules come from `AIN_GF01_DOMAIN` and the issuer key file
/// `AIN_GF01_ISSUER_KEY_FILE` (book 100, 30 days, 1000 coins a day, as the
/// native network deploys them). Writes `{"url"}` to
/// `$AIN_GF01_DIR/identity.json` and runs until `$AIN_GF01_DIR/stop` appears.
#[tokio::test]
#[ignore = "manual: the identity server of a GUI acceptance run"]
async fn gui_host_identity_server() {
    use axum::{
        extract::Query,
        response::{IntoResponse, Redirect},
    };
    let dir = std::path::PathBuf::from(std::env::var("AIN_GF01_DIR").expect("AIN_GF01_DIR"));
    let hex32 = |text: &str| -> [u8; 32] {
        hex::decode(text.trim().trim_start_matches("0x"))
            .unwrap()
            .try_into()
            .unwrap()
    };
    let domain = hex32(&std::env::var("AIN_GF01_DOMAIN").expect("AIN_GF01_DOMAIN"));
    let secret = hex32(
        &std::fs::read_to_string(std::env::var("AIN_GF01_ISSUER_KEY_FILE").unwrap()).unwrap(),
    );
    let now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
    let provider: Provider = Arc::default();
    let accounts = Arc::new(AtomicU64::new(0));
    let authorize = {
        let provider = provider.clone();
        let accounts = accounts.clone();
        move |Query(params): Query<HashMap<String, String>>| {
            let (provider, accounts) = (provider.clone(), accounts.clone());
            async move {
                let n = accounts.fetch_add(1, Ordering::SeqCst);
                let code = format!("gui-code-{n}");
                let claims = json!({"iss":"https://accounts.google.com","aud":CLIENT_ID,"azp":CLIENT_ID,
                "sub":format!("gui-user-{n}"),"email":format!("gui-user-{n}@gmail.com"),"email_verified":true,
                "iat":now(),"exp":now() + 3_600,"nonce":params["nonce"]});
                provider.lock().unwrap().insert(
                    code.clone(),
                    Issued {
                        challenge: params["code_challenge"].clone(),
                        redirect_uri: params["redirect_uri"].clone(),
                        claims,
                    },
                );
                let mut back = Url::parse(&params["redirect_uri"]).unwrap();
                back.query_pairs_mut()
                    .append_pair("code", &code)
                    .append_pair("state", &params["state"]);
                Redirect::to(back.as_str()).into_response()
            }
        }
    };
    // GitHub: its authorize page signs every visitor in as a new account
    // with a verified primary email.
    let github: GitHubState = Arc::default();
    let github_authorize = {
        let github = github.clone();
        let accounts = accounts.clone();
        move |Query(params): Query<HashMap<String, String>>| {
            let (github, accounts) = (github.clone(), accounts.clone());
            async move {
                let n = accounts.fetch_add(1, Ordering::SeqCst);
                let code = format!("gui-github-{n}");
                github.lock().unwrap().codes.insert(
                    code.clone(),
                    (
                        params["code_challenge"].clone(),
                        params["redirect_uri"].clone(),
                        octocat(1_000 + n, true),
                    ),
                );
                let mut back = Url::parse(&params["redirect_uri"]).unwrap();
                back.query_pairs_mut()
                    .append_pair("code", &code)
                    .append_pair("state", &params["state"]);
                Redirect::to(back.as_str()).into_response()
            }
        }
    };
    let google = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let google_url = format!("http://{}", google.local_addr().unwrap());
    let app = Router::new()
        .route("/token", post(token))
        .with_state(provider)
        .route("/authorize", get(authorize))
        .merge(
            Router::new()
                .route("/github/login/oauth/access_token", post(github_token))
                .route("/github/api/user", get(github_user))
                .route("/github/api/user/emails", get(github_emails))
                .with_state(github)
                .route("/github/authorize", get(github_authorize)),
        );
    tokio::spawn(async move { axum::serve(google, app).await.unwrap() });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let config = Config {
        public_url: url.clone(),
        domain,
        signing_secret: secret,
        database: dir.join("identity.db"),
        google: GoogleConfig {
            client_id: CLIENT_ID.into(),
            client_secret: CLIENT_SECRET.into(),
            authorize_url: format!("{google_url}/authorize"),
            token_url: format!("{google_url}/token"),
        },
        github: Some(GitHubConfig {
            client_id: GITHUB_CLIENT_ID.into(),
            client_secret: GITHUB_CLIENT_SECRET.into(),
            authorize_url: format!("{google_url}/github/authorize"),
            token_url: format!("{google_url}/github/login/oauth/access_token"),
            api_url: format!("{google_url}/github/api"),
        }),
        rules: Arc::new(StaticRules {
            domain,
            caps: BTreeMap::from([(0, 1_000)]),
            book_size: 100,
            max_validity_days: 30,
        }),
        grant_validity_days: 14,
        claim_interval_days: 30,
        claim_ttl_secs: 900,
        clock: Arc::new(now),
    };
    let server = Server::start(config, listener).await.unwrap();
    std::fs::write(dir.join("identity.json"), json!({"url": url}).to_string()).unwrap();
    while !dir.join("stop").exists() {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    server.shutdown().await;
}

#[tokio::test]
async fn the_login_link_offers_google_and_github_and_a_verified_github_account_gets_a_grant() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let book = book(7);
    let claim = h.claim(&book, 7).await;
    let chooser = h
        .http
        .get(format!("{}/v1/claims/{claim}/login", h.base()))
        .send()
        .await
        .unwrap();
    assert_eq!(chooser.status().as_u16(), 200);
    assert!(
        chooser.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    let page = chooser.text().await.unwrap();
    for provider in ["google", "github"] {
        assert!(
            page.contains(&format!("/v1/claims/{claim}/login/{provider}")),
            "{page}"
        );
    }
    pending(&h.status(&claim).await);

    let params = h.authorize_github(&claim).await;
    assert_eq!(params["client_id"], GITHUB_CLIENT_ID);
    assert_eq!(params["redirect_uri"], h.github_redirect());
    assert!(
        params["scope"]
            .split(' ')
            .any(|scope| scope == "user:email")
    );
    assert_eq!(params["code_challenge_method"], "S256");
    assert!(params["state"].len() >= 32);
    let code = h.sign_in_at_github(&params, octocat(4242, true));
    let (status, _) = h
        .github_callback(&[("code", &code), ("state", &params["state"])])
        .await;
    assert_eq!(status, 200);
    let grant = grant(&h.status(&claim).await);
    assert_eq!(grant.book, book.account());
    assert_eq!((grant.day, grant.serial, grant.count), (DAY, 0, 50));
    // A replayed callback grants nothing more.
    let (status, _) = h
        .github_callback(&[("code", &code), ("state", &params["state"])])
        .await;
    assert_ne!(status, 200);
    assert_eq!(h.policy().await["grantsIssued"], 1);
}

#[tokio::test]
async fn a_github_account_needs_a_verified_primary_email_and_is_granted_once_per_interval() {
    let h = Harness::start(caps(&[(DAY, 200)])).await;
    denied(
        &h.claim_as_github(octocat(1, false), 1).await,
        "email_not_verified",
    );
    assert_eq!(
        grant(&h.claim_as_github(octocat(2, true), 2).await).serial,
        0
    );
    // The same GitHub account renamed and under another email is still the
    // same account: its id does not change.
    let mut renamed = octocat(2, true);
    renamed.login = "someone-new".into();
    renamed.emails[1]["email"] = json!("renamed@example.org");
    denied(&h.claim_as_github(renamed, 3).await, "already_claimed");
    // A Google account and a GitHub account are different accounts, even with
    // the same identifier.
    assert_eq!(grant(&h.claim_as("2", 4).await).serial, 1);
    h.advance(CLAIM_INTERVAL_DAYS * SECONDS_PER_DAY);
    let next = grant(&h.claim_as_github(octocat(2, true), 5).await);
    assert_eq!((next.day, next.serial), (DAY + CLAIM_INTERVAL_DAYS, 0));
}

#[tokio::test]
async fn a_cancelled_or_failed_github_login_keeps_the_claim_open() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let claim = h.claim(&book(1), 1).await;
    // The owner cancels at GitHub.
    let params = h.authorize_github(&claim).await;
    let (status, _) = h
        .github_callback(&[("error", "access_denied"), ("state", &params["state"])])
        .await;
    assert_ne!(status, 200);
    pending(&h.status(&claim).await);
    // GitHub refuses the code (it answers 200 with an error, as GitHub does).
    let params = h.authorize_github(&claim).await;
    let (status, _) = h
        .github_callback(&[("code", "never-issued"), ("state", &params["state"])])
        .await;
    assert_ne!(status, 200);
    pending(&h.status(&claim).await);
    // The link still works.
    let params = h.authorize_github(&claim).await;
    let code = h.sign_in_at_github(&params, octocat(5, true));
    h.github_callback(&[("code", &code), ("state", &params["state"])])
        .await;
    grant(&h.status(&claim).await);
}

#[tokio::test]
async fn a_login_started_with_one_provider_is_not_honoured_by_the_other() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let claim = h.claim(&book(1), 1).await;
    let google = h.authorize(&claim).await;
    // A GitHub code good for the GitHub callback, sent with Google's state.
    let mut at_github = google.clone();
    at_github.insert("redirect_uri".into(), h.github_redirect());
    let code = h.sign_in_at_github(&at_github, octocat(9, true));
    let (status, _) = h
        .github_callback(&[("code", &code), ("state", &google["state"])])
        .await;
    assert_ne!(status, 200);
    pending(&h.status(&claim).await);
    // The Google login it was started as still completes.
    let (status, _) = h
        .sign_in(&google, h.google("alice", &google["nonce"]))
        .await;
    assert_eq!(status, 200);
    grant(&h.status(&claim).await);
}

#[tokio::test]
async fn without_github_the_login_link_goes_straight_to_google() {
    let h = Harness::google_only(caps(&[(DAY, 100)])).await;
    let claim = h.claim(&book(1), 1).await;
    let response = h
        .http
        .get(format!("{}/v1/claims/{claim}/login", h.base()))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_redirection(), "{}", response.status());
    assert!(
        response.headers()["location"]
            .to_str()
            .unwrap()
            .starts_with(AUTHORIZE)
    );
    let github = h
        .http
        .get(format!("{}/v1/claims/{claim}/login/github", h.base()))
        .send()
        .await
        .unwrap();
    assert_eq!(github.status().as_u16(), 404);
}

#[tokio::test]
async fn the_login_page_of_an_unknown_claim_is_not_found_and_does_not_echo_it() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let hostile = "%3Cscript%3Ealert(1)%3C%2Fscript%3E";
    let response = h
        .http
        .get(format!("{}/v1/claims/{hostile}/login", h.base()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 404);
    assert!(!response.text().await.unwrap().contains("<script>"));
}

#[tokio::test]
async fn a_double_spend_revokes_the_accounts_grants_and_bans_it_for_ninety_days_then_for_good() {
    let mut h = Harness::start(caps(&[(DAY, 100)])).await;
    h.claim_interval_days = 7;
    h.restart().await;
    // Ends as the third is granted: at that instant it is spent no more.
    let ended = grant(&h.claim_as("mallory", 1).await);
    h.advance(7 * SECONDS_PER_DAY);
    let live = grant(&h.claim_as("mallory", 2).await);
    h.advance(7 * SECONDS_PER_DAY);
    let spent = grant(&h.claim_as("mallory", 3).await);
    let honest = grant(&h.claim_as("alice", 4).await);
    assert!(h.now() >= ended.expiry && h.now() < live.expiry);
    assert_eq!(
        h.revocations(0).await,
        json!({"revocations": [], "last": 0})
    );
    // Holders report an hour later.
    h.advance(3_600);
    let (status, body) = h.report(&double_spend(&spent, 3)).await;
    assert_eq!(
        (status, body),
        (StatusCode::OK, json!({"banned": true, "revoked": 2}))
    );
    // Every grant of the account still in force is revoked by the issuer's
    // key; an ended grant and another account's are not.
    let listed = h.revocations(0).await;
    assert_eq!(listed["last"], json!(2));
    let revoked = revocations(&listed);
    let books: BTreeSet<[u8; 32]> = revoked.iter().map(|r| r.book).collect();
    assert_eq!(books, BTreeSet::from([live.id(), spent.id()]));
    for revocation in &revoked {
        assert!(revocation.revokes(&live) || revocation.revokes(&spent));
        assert!(!revocation.revokes(&ended) && !revocation.revokes(&honest));
        assert_eq!(revocation.revoked_at, h.now());
    }
    assert_eq!(
        h.revocations(2).await,
        json!({"revocations": [], "last": 2})
    );
    // Another holder reports the same: nothing changes.
    let (status, body) = h.report(&double_spend(&spent, 3)).await;
    assert_eq!(
        (status, body),
        (StatusCode::OK, json!({"banned": true, "revoked": 0}))
    );
    assert_eq!(h.revocations(0).await, listed);
    // The ban outlives a restart and the claim interval; others still claim.
    h.restart().await;
    h.advance(6 * SECONDS_PER_DAY);
    // The other grant of the same account proven spent twice while banned
    // (a profile restored from a backup spends both): the same offence.
    assert_eq!(
        h.report(&double_spend(&live, 2)).await,
        (StatusCode::OK, json!({"banned": true, "revoked": 0}))
    );
    h.advance(SECONDS_PER_DAY);
    denied(&h.claim_as("mallory", 5).await, "account_banned");
    assert_eq!(grant(&h.claim_as("alice", 6).await).day, DAY + 21);
    // It lasts ninety days from the first report.
    h.advance(82 * SECONDS_PER_DAY);
    denied(&h.claim_as("mallory", 7).await, "account_banned");
    h.advance(SECONDS_PER_DAY);
    let again = grant(&h.claim_as("mallory", 8).await);
    assert_eq!(again.day, DAY + 104);
    // A proof about a grant from before the ban, reported after it ended,
    // bans nothing again.
    assert_eq!(
        h.report(&double_spend(&live, 2)).await,
        (StatusCode::OK, json!({"banned": false, "revoked": 0}))
    );
    h.advance(7 * SECONDS_PER_DAY);
    let next = grant(&h.claim_as("mallory", 9).await);
    // Spending a grant got after the ban twice bans the account for good.
    assert_eq!(
        h.report(&double_spend(&next, 9)).await,
        (StatusCode::OK, json!({"banned": true, "revoked": 2}))
    );
    // Only revocations of grants still in force are listed.
    let listed = h.revocations(0).await;
    assert_eq!(listed["last"], json!(4));
    let books: BTreeSet<[u8; 32]> = revocations(&listed).iter().map(|r| r.book).collect();
    assert_eq!(books, BTreeSet::from([again.id(), next.id()]));
    h.advance(365 * SECONDS_PER_DAY);
    denied(&h.claim_as("mallory", 10).await, "account_banned");
}

#[tokio::test]
async fn a_report_that_proves_no_double_spend_of_this_servers_grant_changes_nothing() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    let bobs = grant(&h.claim_as("bob", 1).await);
    // Bob's key also spent a slot of another book twice.
    let other = GrantBook::issue(
        GrantTerms {
            serial: 1,
            ..bobs.terms()
        },
        &SecpKey::from_secret(&SERVER_SECRET).unwrap(),
    );
    let forged = GrantBook::issue(bobs.terms(), &SecpKey::from_secret(&[0x77; 32]).unwrap());
    let twin = GrantBook::issue(
        GrantTerms {
            book: book(2).account(),
            ..bobs.terms()
        },
        &SecpKey::from_secret(&SERVER_SECRET).unwrap(),
    );
    assert_eq!(twin.id(), bobs.id());
    let cases = [
        // One stamp shown twice.
        (
            json!({"grant": bobs, "first": stamp(&bobs, 1, 5, 1), "second": stamp(&bobs, 1, 5, 1)}),
            "no_proof",
        ),
        // Two honest stamps of two slots: every holder has such pairs.
        (
            json!({"grant": bobs, "first": stamp(&bobs, 1, 5, 1), "second": stamp(&bobs, 1, 6, 2)}),
            "no_proof",
        ),
        // One slot number in two books of the same key.
        (
            json!({"grant": bobs, "first": stamp(&bobs, 1, 5, 1), "second": stamp(&other, 1, 5, 2)}),
            "no_proof",
        ),
        // Signed by a key that is not the grant's book key.
        (double_spend(&bobs, 9), "no_proof"),
        // A double spend of another book, shown with this grant.
        (
            json!({"grant": bobs, "first": stamp(&other, 1, 5, 1), "second": stamp(&other, 1, 5, 2)}),
            "no_proof",
        ),
        // A grant this server never signed.
        (double_spend(&forged, 1), "not_ours"),
        // Another grant under the id of Bob's, signed with this server's key
        // outside a claim (as test runs do): not Bob's to answer for.
        (double_spend(&twin, 2), "not_ours"),
    ];
    for (body, error) in cases {
        assert_eq!(
            h.report(&body).await,
            (StatusCode::BAD_REQUEST, json!({"error": error})),
            "{body}"
        );
    }
    assert_eq!(
        h.revocations(0).await,
        json!({"revocations": [], "last": 0})
    );
    h.advance(CLAIM_INTERVAL_DAYS * SECONDS_PER_DAY);
    assert_eq!(
        grant(&h.claim_as("bob", 2).await).day,
        DAY + CLAIM_INTERVAL_DAYS
    );
}

#[tokio::test]
async fn a_double_spent_grant_issued_before_accounts_were_linked_is_revoked_alone() {
    let h = Harness::start(caps(&[(DAY, 100)])).await;
    // Signed by this server's key, but never through a claim here.
    let old = GrantBook::issue(
        GrantTerms {
            domain: DOMAIN,
            book: book(8).account(),
            day: DAY - 1,
            serial: 0,
            count: 50,
            expiry: h.now() + SECONDS_PER_DAY,
        },
        &SecpKey::from_secret(&SERVER_SECRET).unwrap(),
    );
    assert_eq!(
        h.report(&double_spend(&old, 8)).await,
        (StatusCode::OK, json!({"banned": false, "revoked": 1}))
    );
    let revoked = revocations(&h.revocations(0).await);
    assert_eq!(revoked.len(), 1);
    assert!(revoked[0].revokes(&old));
}
