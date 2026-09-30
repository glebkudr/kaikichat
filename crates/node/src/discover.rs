//! The discovery service as the owner's CLI and window use it
//! (spec/discovery-v1.md): handles taken from an address book, the
//! service's HTTP API, and the flows over it. The node signs what is sent
//! and spends the stamps, and names the service and its key
//! (`discover_config`); this side talks HTTP.
use crate::NETWORK_DOMAIN;
pub use agentic_protocol::directory::normalize_handle;
use agentic_protocol::directory::{Card, Request, verify_binding, verify_request};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::time::{Duration, Instant};

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;

/// The handles of an address book, each once, in their normal form, in the
/// order found: `EMAIL` lines of a vCard, `github:LOGIN` lines, and every
/// word holding an address in a CSV, a plain list or text pasted from a
/// mail client (`Ann <ann@example.org>; bob@example.org`).
pub fn handles_from(text: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    let mut add = |kind: &str, raw: &str| {
        if let Some(normal) = normalize_handle(kind, raw)
            && !found.iter().any(|(k, h)| k == kind && *h == normal)
        {
            found.push((kind.to_owned(), normal));
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(login) = line.strip_prefix("github:") {
            add("github", login);
            continue;
        }
        // vCard: `[group.]EMAIL[;params]:value`; a parameter may hold dots.
        if let Some((name, value)) = line.split_once(':')
            && name
                .split(';')
                .next()
                .and_then(|n| n.rsplit('.').next())
                .is_some_and(|n| n.eq_ignore_ascii_case("EMAIL"))
        {
            add("google", value);
            continue;
        }
        // Google Contacts puts several addresses of a person in one cell,
        // split by ` ::: `; a mail client puts names and brackets around
        // them.
        for field in csv_fields(line) {
            for word in field.split(|c: char| {
                c.is_whitespace() || matches!(c, ';' | ',' | '<' | '>' | '"' | '(' | ')')
            }) {
                if word.contains('@') {
                    add("google", word);
                }
            }
        }
    }
    found
}

/// The fields of one CSV line, quotes honoured.
fn csv_fields(line: &str) -> Vec<String> {
    let mut fields = vec![];
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(std::mem::take(&mut field)),
            c => field.push(c),
        }
    }
    fields.push(field);
    fields
}

/// The service's HTTP API.
pub struct Directory {
    base: String,
    http: reqwest::Client,
}

/// An answer of the service: its body, or its error code.
pub enum Answer {
    Ok(Value),
    Refused(u16, String),
}

impl Directory {
    pub fn new(base: &str) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            http,
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    async fn answer(response: reqwest::Response) -> Result<Answer, String> {
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if status < 300 {
            return Ok(Answer::Ok(body));
        }
        match body["error"].as_str() {
            Some(code) => Ok(Answer::Refused(status, code.to_owned())),
            None => Err(format!("the directory answered {status}")),
        }
    }

    pub async fn get(&self, path: &str) -> Result<Answer, String> {
        let response = self
            .http
            .get(format!("{}{path}", self.base))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::answer(response).await
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<Answer, String> {
        let response = self
            .http
            .post(format!("{}{path}", self.base))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Self::answer(response).await
    }
}

/// A lookup request for `handles` paid with the node's `payment` (its
/// `discover_stamps` answer: the day, the stamps and their books' grants).
pub fn lookup_request(handles: &[(String, [u8; 32])], payment: &Value) -> Value {
    json!({
        "handles": handles
            .iter()
            .map(|(kind, digest)| json!({"kind": kind, "digest": hex::encode(digest)}))
            .collect::<Vec<_>>(),
        "day": payment["day"],
        "stamps": payment["stamps"],
        "grants": payment["grants"],
    })
}

/// The discovery service a daemon names: from its flags, which the
/// network's preset sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryConfig {
    pub url: String,
    /// The key the service signs bindings with, when the network names it;
    /// a profile set by hand without one trusts the service's own.
    pub key: Option<[u8; 32]>,
}

impl DirectoryConfig {
    pub fn new(url: &str, key: Option<&str>) -> Result<Self, String> {
        let parsed = reqwest::Url::parse(url).map_err(|_| "the directory is not a URL")?;
        if !matches!(parsed.scheme(), "https" | "http") {
            return Err("the directory is an http(s) URL".into());
        }
        let key = key
            .map(|key| {
                hex::decode(key)
                    .ok()
                    .and_then(|key| key.try_into().ok())
                    .ok_or("a directory key is 32 bytes of hex")
            })
            .transpose()?;
        Ok(Self {
            url: url.to_owned(),
            key,
        })
    }
}

/// Why a discovery flow stopped: a code, a message, and whether the same
/// request may succeed later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl Refusal {
    pub fn new(code: &str, message: &str, retryable: bool) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_owned(),
            retryable,
        }
    }
    fn unavailable(message: &str) -> Self {
        Self::new("unavailable", message, true)
    }
}

/// The owner IPC of the daemon a flow runs on.
pub trait Node {
    type Error: From<Refusal>;
    fn call(
        &self,
        method: &str,
        request: Value,
    ) -> impl Future<Output = Result<Value, Self::Error>> + Send;
}

/// A service answer: its body, or its refusal.
fn served(answer: Result<Answer, String>) -> Result<Value, Refusal> {
    match answer.map_err(|error| Refusal::unavailable(&error))? {
        Answer::Ok(value) => Ok(value),
        Answer::Refused(status, code) => {
            let retryable = code == "unknown_book" || code == "rate_limited" || status >= 500;
            Err(Refusal::new(
                &code,
                "the discovery service refused",
                retryable,
            ))
        }
    }
}

/// POST a paid `body` to `path`, again while the service's node learns a
/// newly bought or granted book (`unknown_book`), for up to a minute: the
/// same request spends nothing more.
async fn post_paid(service: &Directory, path: &str, body: Value) -> Result<Value, Refusal> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let answer = service.post(path, body.clone()).await;
        if matches!(&answer, Ok(Answer::Refused(_, code)) if code == "unknown_book")
            && Instant::now() < deadline
        {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }
        return served(answer);
    }
}

/// The daemon's discovery service, checked: of this network, and signing
/// with the key the daemon names. Nothing else is sent before.
async fn service<N: Node>(node: &N) -> Result<(Directory, [u8; 32]), N::Error> {
    let answer = node.call("discover_config", json!({})).await?;
    let config = DirectoryConfig::new(
        answer["url"].as_str().unwrap_or_default(),
        answer["key"].as_str(),
    )
    .map_err(|error| Refusal::unavailable(&error))?;
    let service = Directory::new(&config.url).map_err(|error| Refusal::unavailable(&error))?;
    let policy = served(service.get("/v1/policy").await)?;
    let bytes = |field: &str| -> Option<[u8; 32]> {
        hex::decode(policy[field].as_str()?).ok()?.try_into().ok()
    };
    let (Some(domain), Some(key)) = (bytes("domain"), bytes("key")) else {
        return Err(Refusal::unavailable("the discovery service answers no policy").into());
    };
    if domain != NETWORK_DOMAIN {
        return Err(Refusal::unavailable("the discovery service is of another network").into());
    }
    if config.key.is_some_and(|named| named != key) {
        return Err(Refusal::new(
            "directory_key_mismatch",
            "the discovery service is not the one the network names",
            false,
        )
        .into());
    }
    Ok((service, key))
}

/// A login link binding the account the human signs in with of `kind` to
/// this profile: `{linkId, loginUrl, code, expiresAt}`.
pub async fn link<N: Node>(node: &N, kind: &str) -> Result<Value, N::Error> {
    consent(node, "link", kind, "/v1/links").await
}

/// Drop the binding of `kind`.
pub async fn unlink<N: Node>(node: &N, kind: &str) -> Result<Value, N::Error> {
    consent(node, "unlink", kind, "/v1/unlink").await
}

async fn consent<N: Node>(
    node: &N,
    action: &str,
    kind: &str,
    path: &str,
) -> Result<Value, N::Error> {
    let (service, _) = service(node).await?;
    let consent = node
        .call(
            "discover_consent",
            json!({"action": action, "kind": kind, "service": service.base()}),
        )
        .await?;
    Ok(served(
        service
            .post(path, json!({ "consent": consent["consent"] }))
            .await,
    )?)
}

/// A link's state: `{status, reason}`.
pub async fn status<N: Node>(node: &N, link: &str) -> Result<Value, N::Error> {
    if link.is_empty()
        || !link
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(Refusal::new("invalid_input", "not a link id", false).into());
    }
    let (service, _) = service(node).await?;
    Ok(served(service.get(&format!("/v1/links/{link}")).await)?)
}

/// The network ids of normalized `handles` (kind, handle), a coin each, a
/// hundred at a time: `{found: [{kind, handle, networkId}]}`, `networkId`
/// null when nobody bound it. Bindings are checked against the service's
/// key.
pub async fn lookup<N: Node>(node: &N, handles: &[(String, String)]) -> Result<Value, N::Error> {
    let (service, key) = service(node).await?;
    let mut found = vec![];
    for chunk in handles.chunks(100) {
        let digests: Vec<(String, [u8; 32])> = chunk
            .iter()
            .map(|(kind, handle)| (kind.clone(), Sha256::digest(handle.as_bytes()).into()))
            .collect();
        let payment = node
            .call(
                "discover_stamps",
                json!({ "handles": digests
                    .iter()
                    .map(|(kind, d)| json!({"kind": kind, "digest": hex::encode(d)}))
                    .collect::<Vec<_>>() }),
            )
            .await?;
        let answer = post_paid(&service, "/v1/lookup", lookup_request(&digests, &payment)).await?;
        let results = answer["results"].as_array().cloned().unwrap_or_default();
        for ((kind, digest), (_, handle)) in digests.iter().zip(chunk) {
            let network = results
                .iter()
                .find(|r| r["kind"] == kind.as_str() && r["digest"] == hex::encode(digest))
                .and_then(|r| r["binding"].as_str())
                .and_then(|b| hex::decode(b).ok())
                .and_then(|wire| verify_binding(&wire, NETWORK_DOMAIN, now(), &key).ok())
                .filter(|binding| binding.kind == *kind && binding.digest == *digest)
                .map(|binding| binding.network_id);
            found.push(json!({"kind": kind, "handle": handle, "networkId": network}));
        }
    }
    Ok(json!({ "found": found }))
}

/// Publish a card the node makes of `request` (its `discover_card`
/// request): `{id, expiresAt}`.
pub async fn publish<N: Node>(node: &N, request: Value) -> Result<Value, N::Error> {
    let (service, _) = service(node).await?;
    let made = node.call("discover_card", request).await?;
    Ok(post_paid(
        &service,
        "/v1/cards",
        json!({"card": made["card"], "stamps": made["stamps"], "grants": made["grants"]}),
    )
    .await?)
}

/// Take one of this profile's cards off the index.
pub async fn withdraw<N: Node>(node: &N, card: &str) -> Result<Value, N::Error> {
    let (service, _) = service(node).await?;
    let made = node
        .call("discover_withdrawal", json!({ "cardId": card }))
        .await?;
    Ok(served(
        service
            .post("/v1/withdraw", json!({ "withdrawal": made["withdrawal"] }))
            .await,
    )?)
}

/// Cards by words, tag, language or kind, the node's book shown as a pass:
/// `{cards: [...]}`, each read and checked (see [`read_card`]).
pub async fn search<N: Node>(
    node: &N,
    query: &str,
    tag: Option<&str>,
    lang: Option<&str>,
    kind: Option<&str>,
) -> Result<Value, N::Error> {
    let (service, _) = service(node).await?;
    let pass = node.call("discover_pass", json!({})).await?;
    let answer = served(
        service
            .post(
                "/v1/search",
                json!({"query": query, "tag": tag, "lang": lang, "kind": kind, "pass": pass["pass"]}),
            )
            .await,
    )?;
    let cards: Vec<Value> = answer["cards"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(read_card)
        .collect();
    Ok(json!({ "cards": cards }))
}

/// One card by its id, read and checked (see [`read_card`]).
pub async fn card<N: Node>(node: &N, id: &str) -> Result<Value, N::Error> {
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Refusal::new("invalid_input", "a card id is 32 bytes of hex", false).into());
    }
    let (service, _) = service(node).await?;
    let found = served(service.get(&format!("/v1/cards/{id}")).await)?;
    Ok(read_card(&found)
        .ok_or_else(|| Refusal::new("bad_card", "not a card of the service", false))?)
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// A card as the service answered it, its author's signature checked:
/// `{id, kind, name, about, tags, langs, owner, groupRef, expiresAt}`.
pub fn read_card(found: &Value) -> Option<Value> {
    let wire = hex::decode(found["card"].as_str()?).ok()?;
    let (author, request, _) = verify_request(&wire, NETWORK_DOMAIN, now()).ok()?;
    let Request::Card(card) = request else {
        return None;
    };
    let owner = agentic_protocol::network_id(&author);
    let (kind, group_ref) = match &card {
        Card::Group { group_id, .. } | Card::Channel { group_id, .. } => (
            if matches!(card, Card::Channel { .. }) {
                "channel"
            } else {
                "group"
            },
            Some(hex::encode(agentic_protocol::group::group_ref(
                &NETWORK_DOMAIN,
                &author,
                group_id,
            ))),
        ),
        Card::Profile { .. } => ("profile", None),
    };
    Some(json!({
        "id": found["id"],
        "kind": kind,
        "name": card.name(),
        "about": card.about(),
        "tags": card.tags(),
        "langs": card.langs(),
        "owner": owner,
        "groupRef": group_ref,
        "expiresAt": found["expiresAt"],
    }))
}
