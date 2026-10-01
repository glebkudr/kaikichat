//! The signed network preset (Docs/V1_NETWORK_PRESET_2026_09_28_RU.md): the
//! one address the app knows, and the network a starting daemon takes from
//! it — kept in the profile, updated within a network, offered across
//! networks, and never taken unsigned. It also names the app's latest
//! release, which the app offers to install.
use crate::host::{DaemonFlags, START_LOCK};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use fs2::FileExt;
use libp2p::{Multiaddr, multiaddr::Protocol};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub type Result<T> = crate::Result<T>;

/// Where the app finds its network.
pub const PRESET_URL: &str = "https://kaikichat.com/network.json";
/// The key the presets at `PRESET_URL` are signed with; its secret is kept
/// offline.
pub const PRESET_KEY: &str = "70485f722511aa48711f5ac650eb6b09f90098538c1baee6eb22ef8152f84d2c";

const DOMAIN: &[u8] = b"kaiki-network-preset/v1\n";
const FILE: &str = "network-preset.json";
/// What the profile knows of releases, beside the network it follows.
const RELEASE_FILE: &str = "release.json";
/// How often the app asks for a newer release between daemon starts.
pub const RELEASE_CHECK_EVERY: Duration = Duration::from_secs(12 * 3600);
const MAX_BYTES: usize = 64 * 1024;
const MAX_ROUTES: usize = 16;
/// The bootstrap routes a daemon takes at most.
const DAEMON_ROUTES: usize = 4;
/// A fetch holds the profile's start lock: short, so that another start
/// waiting for it does not give up.
const TIMEOUT: Duration = Duration::from_secs(5);
/// No preset below this serial is taken, even by a new profile: raised in a
/// release to retire the presets signed before it.
pub const MIN_SERIAL: u64 = 1;

/// A preset URL and the key its presets must be signed with.
#[derive(Clone, Debug)]
pub struct PresetSource {
    url: String,
    key: VerifyingKey,
    timeout: Duration,
}

impl PresetSource {
    /// `url` over https (or http on loopback), `key` as 64 hex digits.
    pub fn new(url: &str, key: &str) -> Result<Self> {
        check_url(url)?;
        let key: [u8; 32] = hex::decode(key)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or("a preset key is 64 hex digits")?;
        Ok(Self {
            url: url.to_owned(),
            key: VerifyingKey::from_bytes(&key)?,
            timeout: TIMEOUT,
        })
    }

    /// kaikichat.com's preset.
    pub fn public() -> Result<Self> {
        Self::new(PRESET_URL, PRESET_KEY)
    }

    /// The source `AGENTIC_NETWORK_PRESET` and `AGENTIC_NETWORK_PRESET_KEY`
    /// would give: none for `off`, another URL, else kaikichat.com's. Another
    /// key is taken only by debug builds (tests); a release trusts its own.
    pub fn from_values(preset: Option<&str>, key: Option<&str>) -> Result<Option<Self>> {
        match preset {
            None | Some("") => Self::public().map(Some),
            Some("off") => Ok(None),
            Some(url) => {
                let key = key
                    .filter(|key| cfg!(debug_assertions) && !key.is_empty())
                    .unwrap_or(PRESET_KEY);
                Self::new(url, key).map(Some)
            }
        }
    }

    pub fn from_env() -> Result<Option<Self>> {
        Self::from_values(
            std::env::var("AGENTIC_NETWORK_PRESET").ok().as_deref(),
            std::env::var("AGENTIC_NETWORK_PRESET_KEY").ok().as_deref(),
        )
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// How long a fetch may take in all.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// https, or http on loopback, with no user name or password.
fn check_url(url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url)?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(format!("{url} has credentials").into());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("{url} has no host"))?;
    let loopback = host == "localhost"
        || host
            .parse::<std::net::Ipv4Addr>()
            .is_ok_and(|ip| ip.is_loopback())
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::Ipv6Addr>()
            .is_ok_and(|ip| ip.is_loopback());
    match parsed.scheme() {
        "https" => Ok(()),
        "http" if loopback => Ok(()),
        _ => Err(format!("{url} is not https").into()),
    }
}

/// The network a preset gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub network: String,
    pub name: String,
    pub serial: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
    pub bootstrap: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_rpc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_shop: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant_issuer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_confirmations: Option<u64>,
    /// `OperatorPool`, where operators' nodes draw and withdraw prizes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_pool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_server: Option<String>,
    /// The discovery service and the key it signs with; a directory set by
    /// hand wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_key: Option<String>,
    /// Where a new profile starts: the network's welcome agent and lobby.
    /// Not a daemon flag: `kaiki network` shows it for the agent to use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome: Option<Welcome>,
    /// The channels and groups the network recommends to a new profile, in
    /// its order: the window offers them at the first run, `kaiki network`
    /// shows them for the agent to offer. Not daemon flags either.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended: Vec<Recommended>,
    /// The app's latest release. Not a daemon flag either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Release>,
}

/// The app's latest release: its version and its builds by name
/// (`cli-macos-arm64`, `app-macos-arm64`, …), each an archive and its SHA-256.
/// The hash is what an update installs: signed with the preset, it holds
/// even if the downloads are replaced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub builds: BTreeMap<String, Build>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    pub url: String,
    pub sha256: String,
}

impl Release {
    fn check(&self) -> Result<()> {
        version_of(&self.version)?;
        for build in self.builds.values() {
            check_url(&build.url)?;
            if build.sha256.len() != 64 || !build.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("a build's sha256 is 32 bytes of hex".into());
            }
        }
        Ok(())
    }
}

/// The network's welcome agent (a script on its nodes, deploy/node/welcome.py)
/// and its lobby, an open group where new agents meet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Welcome {
    /// Its network id, `ain1` and 64 lowercase hex digits.
    pub agent: String,
    pub name: String,
    /// The lobby's reference `G`, 64 lowercase hex digits: what
    /// `kaiki groups join --group-ref` takes.
    pub lobby: String,
    pub lobby_name: String,
}

/// 32 bytes written as 64 lowercase hex digits.
fn lower_hex(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A network id: `ain1` and 64 lowercase hex digits.
fn network_id(text: &str) -> bool {
    text.strip_prefix("ain1").is_some_and(lower_hex)
}

/// At most this many recommendations in a preset.
const MAX_RECOMMENDED: usize = 8;
/// The kinds of recommendation this version offers.
const KNOWN_KINDS: [&str; 2] = ["channel", "group"];

/// A channel or group the network recommends to a new profile: a channel is
/// followed (`kaiki groups follow --group-ref REF --owner OWNER --name
/// NAME`), a group joined (`kaiki groups join --group-ref REF`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommended {
    /// `channel` or `group`. A kind a later version adds keeps the preset
    /// valid and is left out of the status.
    pub kind: String,
    /// The group's reference `G`, 64 lowercase hex digits.
    #[serde(rename = "ref")]
    pub group_ref: String,
    /// The owner's network id.
    pub owner: String,
    pub name: String,
}

impl Recommended {
    fn check(&self) -> Result<()> {
        let kind = self.kind.len();
        if kind == 0 || kind > 16 || !self.kind.bytes().all(|b| b.is_ascii_lowercase()) {
            return Err("a recommendation's kind is 1-16 of a-z".into());
        }
        if !lower_hex(&self.group_ref) {
            return Err("a recommendation's reference is 32 bytes of lowercase hex".into());
        }
        if !network_id(&self.owner) {
            return Err("a recommendation's owner is ain1 and 64 lowercase hex digits".into());
        }
        let name = self.name.chars().count();
        if name == 0 || name > 80 {
            return Err("a recommendation's name is 1-80 characters".into());
        }
        Ok(())
    }

    /// Whether this version knows what to do with it.
    fn known(&self) -> bool {
        KNOWN_KINDS.contains(&self.kind.as_str())
    }
}

impl Welcome {
    fn check(&self) -> Result<()> {
        if !network_id(&self.agent) {
            return Err("a welcome agent is ain1 and 64 lowercase hex digits".into());
        }
        if !lower_hex(&self.lobby) {
            return Err("a lobby is 32 bytes of lowercase hex".into());
        }
        for name in [&self.name, &self.lobby_name] {
            let length = name.chars().count();
            if length == 0 || length > 80 {
                return Err("a welcome agent's and a lobby's names are 1-80 characters".into());
            }
        }
        Ok(())
    }
}

impl Preset {
    fn check(&self) -> Result<()> {
        let id = &self.network;
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err("a network id is 1-64 of a-z, 0-9 and -".into());
        }
        let name = self.name.chars().count();
        if name == 0 || name > 80 {
            return Err("a network name is 1-80 characters".into());
        }
        if let Some(version) = &self.min_version {
            version_of(version)?;
        }
        if self.serial < MIN_SERIAL {
            return Err(format!("serials start at {MIN_SERIAL}").into());
        }
        if self.bootstrap.is_empty() || self.bootstrap.len() > MAX_ROUTES {
            return Err("a preset has 1-16 bootstrap routes".into());
        }
        // One route per peer: the daemon refuses two routes of one peer.
        let mut peers = BTreeSet::new();
        for route in &self.bootstrap {
            let address: Multiaddr = route.parse()?;
            let Some(Protocol::P2p(peer)) = address.iter().last() else {
                return Err(format!("{route} names no peer").into());
            };
            if !peers.insert(peer) {
                return Err(format!("{route} names a peer given before").into());
            }
        }
        // The chain is given whole or not at all, as the daemon takes it.
        let chain = [
            self.chain_rpc.is_some(),
            self.chain_id.is_some(),
            self.book_shop.is_some(),
            self.grant_issuer.is_some(),
            self.registry.is_some(),
        ];
        if chain.contains(&true) && chain.contains(&false) {
            return Err("a chain needs its RPC, id, shop, issuer and registry".into());
        }
        match self.chain_confirmations {
            Some(0) => return Err("a chain needs at least one confirmation".into()),
            Some(_) if !chain[0] => return Err("confirmations need a chain".into()),
            _ => {}
        }
        if self.operator_pool.is_some() && !chain[0] {
            return Err("an operator pool needs a chain".into());
        }
        if let Some(key) = &self.directory_key {
            if self.directory.is_none() {
                return Err("a directory key needs its directory".into());
            }
            if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("a directory key is 32 bytes of hex".into());
            }
        }
        if let Some(welcome) = &self.welcome {
            welcome.check()?;
        }
        if self.recommended.len() > MAX_RECOMMENDED {
            return Err(format!("a preset recommends at most {MAX_RECOMMENDED}").into());
        }
        let mut recommended = BTreeSet::new();
        for recommendation in &self.recommended {
            recommendation.check()?;
            if !recommended.insert(&recommendation.group_ref) {
                return Err("a preset recommends a group once".into());
            }
        }
        if let Some(release) = &self.release {
            release.check()?;
        }
        for url in [&self.chain_rpc, &self.identity_server, &self.directory]
            .into_iter()
            .flatten()
        {
            check_url(url)?;
        }
        for address in [
            &self.book_shop,
            &self.grant_issuer,
            &self.registry,
            &self.operator_pool,
        ]
        .into_iter()
        .flatten()
        {
            let digits = address.strip_prefix("0x").unwrap_or_default();
            if digits.len() != 40 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(format!("{address} is not a contract address").into());
            }
        }
        Ok(())
    }

    /// `base` with this network: every field, and up to four bootstrap
    /// routes chosen at random, so that clients spread over the nodes.
    pub fn flags(&self, base: DaemonFlags) -> DaemonFlags {
        let mut routes = self.bootstrap.clone();
        for i in (1..routes.len()).rev() {
            let mut bytes = [0u8; 8];
            let pick = match getrandom::fill(&mut bytes) {
                Ok(()) => u64::from_le_bytes(bytes) % (i as u64 + 1),
                Err(_) => i as u64,
            };
            routes.swap(i, pick as usize);
        }
        routes.truncate(DAEMON_ROUTES);
        DaemonFlags {
            bootstrap: routes,
            chain_rpc: self.chain_rpc.clone(),
            chain_id: self.chain_id,
            book_shop: self.book_shop.clone(),
            grant_issuer: self.grant_issuer.clone(),
            registry: self.registry.clone(),
            chain_confirmations: self.chain_confirmations,
            operator_pool: self.operator_pool.clone(),
            identity_server: self.identity_server.clone(),
            directory: base.directory.clone().or_else(|| self.directory.clone()),
            directory_key: if base.directory.is_some() {
                base.directory_key.clone()
            } else {
                self.directory_key
                    .as_ref()
                    .map(|key| key.to_ascii_lowercase())
            },
            ..base
        }
    }

    /// Whether this app is too old for the preset.
    fn needs_newer_app(&self) -> bool {
        let ours = version_of(env!("CARGO_PKG_VERSION")).unwrap_or_default();
        self.min_version
            .as_deref()
            .and_then(|version| version_of(version).ok())
            .is_some_and(|wanted| wanted > ours)
    }
}

pub(crate) fn version_of(text: &str) -> Result<(u64, u64, u64)> {
    let parts: Vec<u64> = text
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| format!("{text} is not X.Y.Z"))?;
    match parts.as_slice() {
        [major, minor, patch] => Ok((*major, *minor, *patch)),
        _ => Err(format!("{text} is not X.Y.Z").into()),
    }
}

/// What the preset URL serves: the preset's text and its signature.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Envelope {
    preset: String,
    signature: String,
}

impl Envelope {
    fn open(&self, key: &VerifyingKey) -> Result<Preset> {
        let signature: [u8; 64] = hex::decode(&self.signature)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or("a signature is 128 hex digits")?;
        key.verify_strict(&message(&self.preset), &Signature::from_bytes(&signature))
            .map_err(|_| "the preset is not signed by its key")?;
        let preset: Preset = serde_json::from_str(&self.preset)?;
        preset.check()?;
        Ok(preset)
    }
}

fn message(text: &str) -> Vec<u8> {
    let mut message = DOMAIN.to_vec();
    message.extend_from_slice(text.as_bytes());
    message
}

/// The public key of `seed`, in hex.
pub fn public_key(seed: &[u8; 32]) -> String {
    hex::encode(SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

/// The file to serve for the preset `payload` (its JSON text), once the
/// preset is valid.
pub fn sign(payload: &str, seed: &[u8; 32]) -> Result<String> {
    let preset: Preset = serde_json::from_str(payload)?;
    preset.check()?;
    let signature = SigningKey::from_bytes(seed).sign(&message(payload));
    Ok(serde_json::to_string(&Envelope {
        preset: payload.to_owned(),
        signature: hex::encode(signature.to_bytes()),
    })?)
}

/// The preset in `file` if `key` signed it and it is valid.
pub fn verify(file: &[u8], key: &str) -> Result<Preset> {
    let source = PresetSource::new(PRESET_URL, key)?;
    serde_json::from_slice::<Envelope>(file)?.open(&source.key)
}

/// Where a profile's network comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Preset,
    Manual,
    Off,
}

/// The outcome of the last check (see the doc's table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Current,
    Cached,
    Unavailable,
    Switch,
    Update,
    Manual,
    Off,
}

/// Another network the profile may move to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub network: String,
    pub name: String,
    pub serial: u64,
}

/// A profile's network, as the window and `kaiki network` show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetStatus {
    pub source: Source,
    pub state: State,
    pub network: Option<String>,
    pub name: Option<String>,
    pub serial: Option<u64>,
    pub checked_at: Option<u64>,
    pub offered: Option<Offer>,
    pub required: Option<String>,
    pub error: Option<String>,
    /// The welcome agent and lobby of the network the profile is on.
    pub welcome: Option<Welcome>,
    /// What that network recommends to a new profile, of the kinds this
    /// version knows.
    pub recommended: Vec<Recommended>,
}

impl PresetStatus {
    fn bare(source: Source, state: State) -> Self {
        Self {
            source,
            state,
            network: None,
            name: None,
            serial: None,
            checked_at: None,
            offered: None,
            required: None,
            error: None,
            welcome: None,
            recommended: Vec::new(),
        }
    }
    /// A profile with no preset source.
    pub fn off() -> Self {
        Self::bare(Source::Off, State::Off)
    }
}

/// Offering another network needs the owner's word.
#[derive(Debug)]
pub struct NoNetworkOffer;

impl fmt::Display for NoNetworkOffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("no other network is offered")
    }
}

impl std::error::Error for NoNetworkOffer {}

/// `network-preset.json`: the presets as served, and the last check.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kept {
    #[serde(default)]
    accepted: Option<Envelope>,
    #[serde(default)]
    offered: Option<Envelope>,
    #[serde(default)]
    state: Option<State>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    required: Option<String>,
    #[serde(default)]
    checked_at: Option<u64>,
    /// The highest serial this profile took or was offered: nothing below it
    /// is offered again.
    #[serde(default)]
    highest: Option<u64>,
}

impl Kept {
    /// What the profile keeps, field by field: a field this version cannot
    /// read (a newer app's) counts as missing, not the whole file.
    fn load(data_dir: &Path) -> Self {
        let Some(saved) = fs::read(data_dir.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            return Self::default();
        };
        fn field<T: serde::de::DeserializeOwned>(
            saved: &serde_json::Value,
            key: &str,
        ) -> Option<T> {
            serde_json::from_value(saved.get(key)?.clone()).ok()
        }
        Self {
            accepted: field(&saved, "accepted"),
            offered: field(&saved, "offered"),
            state: field(&saved, "state"),
            error: field(&saved, "error"),
            required: field(&saved, "required"),
            checked_at: field(&saved, "checkedAt"),
            highest: field(&saved, "highest"),
        }
    }

    fn raise(&mut self, serial: u64) {
        self.highest = Some(self.highest.unwrap_or(0).max(serial));
    }

    fn save(&self, data_dir: &Path) -> Result<()> {
        write_private(data_dir, FILE, &serde_json::to_vec(self)?)
    }

    /// The accepted and offered presets that `key` signed.
    fn opened(&self, key: &VerifyingKey) -> (Option<Preset>, Option<Preset>) {
        let open = |envelope: &Option<Envelope>| envelope.as_ref()?.open(key).ok();
        (open(&self.accepted), open(&self.offered))
    }
}

/// `bytes` as `name` in the profile, private to its owner, whole or not at
/// all: written beside it and renamed over it.
fn write_private(data_dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(data_dir)?;
    let mut nonce = [0u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| "OS randomness unavailable")?;
    let temporary = data_dir.join(format!(
        "{name}.{}.{}.new",
        std::process::id(),
        hex::encode(nonce)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(temporary, data_dir.join(name))?;
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |now| now.as_secs())
}

/// The preset `source` serves now: 200, no redirects, at most 64 KiB.
async fn fetch(source: &PresetSource) -> Result<(Preset, Envelope)> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(source.timeout)
        .connect_timeout(source.timeout)
        .build()?;
    let mut response = client.get(&source.url).send().await?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(format!("{} answered {}", source.url, response.status()).into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_BYTES {
            return Err(format!("{} served more than 64 KiB", source.url).into());
        }
    }
    let envelope: Envelope = serde_json::from_slice(&body)?;
    let preset = envelope.open(&source.key)?;
    Ok((preset, envelope))
}

/// The flags this profile's daemon starts with. A profile with network
/// flags of its own, or with no source, keeps its flags; any other takes
/// its network from the preset, as the doc's table decides, and keeps the
/// outcome for `status`.
pub async fn resolve(data_dir: &Path, source: Option<&PresetSource>) -> DaemonFlags {
    let flags = DaemonFlags::load(data_dir);
    let Some(source) = source.filter(|_| !flags.has_network()) else {
        return flags;
    };
    let mut kept = Kept::load(data_dir);
    let (mut accepted, offered) = kept.opened(&source.key);
    if offered.is_none() {
        kept.offered = None;
    }
    if accepted.is_none() {
        kept.accepted = None;
    }
    kept.error = None;
    kept.required = None;
    kept.checked_at = Some(unix_now());
    for preset in [&accepted, &offered].into_iter().flatten() {
        kept.raise(preset.serial);
    }
    let fetched = fetch(source).await;
    // The daemon starts under the profile's lock: what the preset says of
    // releases is kept here too.
    let mut release = KeptRelease::load(data_dir);
    release.record(&source.key, &fetched);
    if let Err(error) = release.save(data_dir) {
        tracing::warn!("the release was not kept: {error}");
    }
    let state = match fetched {
        Err(error) => {
            kept.error = Some(error.to_string());
            if accepted.is_none() {
                State::Unavailable
            } else if kept.offered.is_some() {
                State::Switch
            } else {
                State::Cached
            }
        }
        Ok((preset, envelope)) => {
            let highest = kept.highest.unwrap_or(0);
            let same_offer = offered.as_ref().is_some_and(|offer| {
                offer.network == preset.network && offer.serial == preset.serial
            });
            let verdict = match &accepted {
                None if preset.serial >= highest => Verdict::Take,
                Some(current) if preset.network == current.network => {
                    if preset.serial >= current.serial {
                        Verdict::Take
                    } else {
                        Verdict::Old
                    }
                }
                Some(_) if same_offer => Verdict::Offered,
                Some(_) if preset.serial > highest => Verdict::Offer,
                _ => Verdict::Old,
            };
            match verdict {
                Verdict::Old => {
                    kept.error = Some(format!(
                        "the server gave {} at serial {}, older than this profile's",
                        preset.network, preset.serial
                    ));
                    if accepted.is_none() {
                        State::Unavailable
                    } else if kept.offered.is_some() {
                        State::Switch
                    } else {
                        State::Cached
                    }
                }
                Verdict::Offered => State::Switch,
                // Only a preset that would be taken or offered asks for an update.
                _ if preset.needs_newer_app() => {
                    kept.required = preset.min_version.clone();
                    State::Update
                }
                Verdict::Offer => {
                    kept.raise(preset.serial);
                    kept.offered = Some(envelope);
                    State::Switch
                }
                Verdict::Take => {
                    kept.raise(preset.serial);
                    if offered
                        .as_ref()
                        .is_some_and(|offer| offer.serial < preset.serial)
                    {
                        kept.offered = None;
                    }
                    kept.accepted = Some(envelope);
                    accepted = Some(preset);
                    if kept.offered.is_some() {
                        State::Switch
                    } else {
                        State::Current
                    }
                }
            }
        }
    };
    kept.state = Some(state);
    if let Err(error) = kept.save(data_dir) {
        tracing::warn!("the network preset was not kept: {error}");
    }
    match accepted {
        Some(preset) => preset.flags(flags),
        None => flags,
    }
}

/// What a fetched preset is to the profile.
enum Verdict {
    /// Its network, as new or newer: taken.
    Take,
    /// Another network, newer than anything seen: offered.
    Offer,
    /// The offer already made.
    Offered,
    /// Older than what the profile has seen.
    Old,
}

/// The profile's network as last checked.
pub fn status(data_dir: &Path, source: Option<&PresetSource>) -> PresetStatus {
    if DaemonFlags::load(data_dir).has_network() {
        return PresetStatus::bare(Source::Manual, State::Manual);
    }
    let Some(source) = source else {
        return PresetStatus::off();
    };
    let kept = Kept::load(data_dir);
    let (accepted, offered) = kept.opened(&source.key);
    let state = match (kept.state, &accepted, &offered) {
        (Some(State::Current | State::Cached | State::Switch), None, _) | (None, None, _) => {
            State::Unavailable
        }
        (Some(State::Switch), Some(_), None) | (None, Some(_), _) => State::Cached,
        (Some(state), _, _) => state,
    };
    PresetStatus {
        source: Source::Preset,
        state,
        network: accepted.as_ref().map(|preset| preset.network.clone()),
        name: accepted.as_ref().map(|preset| preset.name.clone()),
        serial: accepted.as_ref().map(|preset| preset.serial),
        checked_at: kept.checked_at,
        offered: offered.map(|preset| Offer {
            network: preset.network,
            name: preset.name,
            serial: preset.serial,
        }),
        required: kept.required,
        error: kept.error,
        recommended: accepted
            .as_ref()
            .map(|preset| {
                preset
                    .recommended
                    .iter()
                    .filter(|r| r.known())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
        welcome: accepted.and_then(|preset| preset.welcome),
    }
}

/// The network offered to this profile, if any.
pub fn offered(data_dir: &Path, source: &PresetSource) -> Result<Preset> {
    let (accepted, offered) = Kept::load(data_dir).opened(&source.key);
    offered
        .filter(|offer| {
            accepted
                .as_ref()
                .is_none_or(|current| offer.serial > current.serial)
        })
        .ok_or_else(|| NoNetworkOffer.into())
}

/// Takes the network offered to this profile, as it was checked when
/// offered; the daemon takes it at its next start.
pub fn accept_offer(data_dir: &Path, source: &PresetSource) -> Result<Preset> {
    let preset = offered(data_dir, source)?;
    let mut kept = Kept::load(data_dir);
    kept.raise(preset.serial);
    kept.accepted = kept.offered.take();
    kept.state = Some(State::Current);
    kept.error = None;
    kept.required = None;
    kept.save(data_dir)?;
    Ok(preset)
}

/// `release.json`: the newest preset fetched, for the release it names; the
/// last check; and the version the owner skipped.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeptRelease {
    /// The valid preset of the highest serial fetched, of any network: a
    /// release is the app's. An older one does not replace it, so a replayed
    /// preset brings no other release back; a newer one without a release
    /// withdraws it.
    #[serde(default)]
    newest: Option<Envelope>,
    #[serde(default)]
    checked_at: Option<u64>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    skipped: Option<String>,
}

impl KeptRelease {
    /// Field by field, as `Kept`: what this version cannot read counts as
    /// missing.
    fn load(data_dir: &Path) -> Self {
        let Some(saved) = fs::read(data_dir.join(RELEASE_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            return Self::default();
        };
        fn field<T: serde::de::DeserializeOwned>(
            saved: &serde_json::Value,
            key: &str,
        ) -> Option<T> {
            serde_json::from_value(saved.get(key)?.clone()).ok()
        }
        Self {
            newest: field(&saved, "newest"),
            checked_at: field(&saved, "checkedAt"),
            error: field(&saved, "error"),
            skipped: field(&saved, "skipped"),
        }
    }

    fn save(&self, data_dir: &Path) -> Result<()> {
        write_private(data_dir, RELEASE_FILE, &serde_json::to_vec(self)?)
    }

    /// The newest preset, if `key` signed it.
    fn newest(&self, key: &VerifyingKey) -> Option<Preset> {
        self.newest.as_ref()?.open(key).ok()
    }

    /// A check's outcome: a preset at least as new as the newest kept
    /// becomes the newest.
    fn record(&mut self, key: &VerifyingKey, fetched: &Result<(Preset, Envelope)>) {
        self.checked_at = Some(unix_now());
        match fetched {
            Ok((preset, envelope)) => {
                self.error = None;
                if self
                    .newest(key)
                    .is_none_or(|newest| preset.serial >= newest.serial)
                {
                    self.newest = Some(envelope.clone());
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Whether the last check is younger than `every`; a clock set back
    /// makes it old.
    fn fresh(&self, every: Duration) -> bool {
        let now = unix_now();
        self.checked_at
            .is_some_and(|at| at <= now && now - at < every.as_secs())
    }

    /// Changes the kept release under the profile's lock; `wait` for the
    /// lock, or give up when a daemon start holds it (the start keeps what
    /// its own check learned).
    fn change(data_dir: &Path, wait: bool, change: impl FnOnce(&mut Self)) -> Result<()> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(data_dir)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(data_dir.join(START_LOCK))?;
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            match lock.try_lock_exclusive() {
                Ok(()) => break,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if !wait || std::time::Instant::now() >= deadline {
                        return Err("the profile is busy starting its daemon".into());
                    }
                    std::thread::sleep(Duration::from_millis(40));
                }
                Err(error) => return Err(error.into()),
            }
        }
        let mut kept = Self::load(data_dir);
        change(&mut kept);
        kept.save(data_dir)
    }
}

/// This app's version and the latest release, as `kaiki update --check`,
/// every `kaiki` answer and the window show them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseStatus {
    pub current: String,
    pub latest: Option<String>,
    /// The latest is newer than this app.
    pub available: bool,
    /// The owner skipped the latest.
    pub skipped: bool,
    pub checked_at: Option<u64>,
    /// Why the last check failed.
    pub error: Option<String>,
}

impl ReleaseStatus {
    /// This app, with no release known.
    pub fn unknown() -> Self {
        release_status_of(&KeptRelease::default(), None)
    }
    /// Whether to tell the owner: a newer release, not skipped.
    pub fn news(&self) -> bool {
        self.available && !self.skipped
    }
}

fn release_status_of(kept: &KeptRelease, source: Option<&PresetSource>) -> ReleaseStatus {
    let current = env!("CARGO_PKG_VERSION").to_owned();
    let latest = source
        .and_then(|source| kept.newest(&source.key))
        .and_then(|preset| preset.release)
        .map(|release| release.version);
    let newer = |latest: &str| {
        version_of(latest)
            .ok()
            .zip(version_of(&current).ok())
            .is_some_and(|(latest, current)| latest > current)
    };
    ReleaseStatus {
        available: latest.as_deref().is_some_and(newer),
        skipped: latest.is_some() && kept.skipped == latest,
        latest,
        current,
        checked_at: kept.checked_at,
        error: kept.error.clone(),
    }
}

/// The latest release as the profile knows it; asks nobody.
pub fn release_status(data_dir: &Path, source: Option<&PresetSource>) -> ReleaseStatus {
    release_status_of(&KeptRelease::load(data_dir), source)
}

/// Asks `source` for the latest release when the last check is older than
/// `every`, or now for none. Only the owner's asking (none) checks a
/// profile set by hand. The network is not changed: a daemon takes it when
/// it starts.
pub async fn check_release(
    data_dir: &Path,
    source: &PresetSource,
    every: Option<Duration>,
) -> ReleaseStatus {
    if let Some(every) = every
        && (DaemonFlags::load(data_dir).has_network() || KeptRelease::load(data_dir).fresh(every))
    {
        return release_status(data_dir, Some(source));
    }
    let fetched = fetch(source).await;
    if let Err(error) = KeptRelease::change(data_dir, every.is_none(), |kept| {
        kept.record(&source.key, &fetched)
    }) {
        tracing::warn!("the release check was not kept: {error}");
    }
    release_status(data_dir, Some(source))
}

/// The latest release with its builds, as signed.
pub fn latest_release(data_dir: &Path, source: &PresetSource) -> Option<Release> {
    KeptRelease::load(data_dir).newest(&source.key)?.release
}

/// Skips `version`: the owner is not told of it again, only of a newer one.
pub fn skip_release(
    data_dir: &Path,
    source: &PresetSource,
    version: &str,
) -> Result<ReleaseStatus> {
    version_of(version)?;
    KeptRelease::change(data_dir, true, |kept| {
        kept.skipped = Some(version.to_owned())
    })?;
    Ok(release_status(data_dir, Some(source)))
}

#[cfg(test)]
#[path = "network_preset_tests.rs"]
mod tests;
