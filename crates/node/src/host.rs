//! Starting or reusing the profile's daemon with its secret, and the owner
//! client; shared by the desktop app and the owner CLI (spec/owner-cli-v1.md).
use crate::ipc;
use crate::network_preset::{self, NoNetworkOffer, PresetSource};
use fs2::FileExt;
use libp2p::{Multiaddr, multiaddr::Protocol};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
pub type Result<T> = crate::Result<T>;
/// Implementations must persist atomically before returning success from set.
pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn set(&self, account: &str, secret: &[u8]) -> Result<()>;
    /// Whether reading `account` makes the system ask the owner first (on
    /// macOS: another program saved the keychain item, and this one is not
    /// yet trusted with it). Shows nothing itself; a store that never asks
    /// says no.
    fn asks_first(&self, _account: &str) -> Result<bool> {
        Ok(false)
    }
}
pub struct KeychainStore {
    service: String,
}
impl KeychainStore {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.into(),
        }
    }
    pub fn remove(&self, account: &str) -> Result<()> {
        keyring::Entry::new(&self.service, account)?.delete_credential()?;
        Ok(())
    }
}
impl SecretStore for KeychainStore {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        match keyring::Entry::new(&self.service, account)?.get_secret() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
    fn set(&self, account: &str, secret: &[u8]) -> Result<()> {
        keyring::Entry::new(&self.service, account)?.set_secret(secret)?;
        Ok(())
    }
    /// Reads the item with the keychain's dialogs off: a refusal the dialog
    /// would lift means macOS asks when the item is read for real.
    #[cfg(target_os = "macos")]
    fn asks_first(&self, account: &str) -> Result<bool> {
        let entry = keyring::Entry::new(&self.service, account)?;
        let quiet =
            security_framework::os::macos::keychain::SecKeychain::disable_user_interaction()?;
        let read = entry.get_secret().map(Zeroizing::new);
        drop(quiet);
        match read {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(false),
            Err(error) if needs_dialog(&error) => Ok(true),
            Err(error) => Err(error.into()),
        }
    }
}

/// Whether the keychain refused a read with its dialogs off where the
/// dialog would have let it through: the item does not trust this program
/// (errSecAuthFailed) or the keychain waits to be unlocked
/// (errSecInteractionNotAllowed).
#[cfg(target_os = "macos")]
fn needs_dialog(error: &keyring::Error) -> bool {
    const AUTH_FAILED: i32 = -25293;
    const INTERACTION_NOT_ALLOWED: i32 = -25308;
    match error {
        keyring::Error::PlatformFailure(inner) | keyring::Error::NoStorageAccess(inner) => inner
            .downcast_ref::<security_framework::base::Error>()
            .is_some_and(|error| matches!(error.code(), AUTH_FAILED | INTERACTION_NOT_ALLOWED)),
        _ => false,
    }
}

/// The keychain account of the profile in `data_dir` (canonical): the
/// SHA-256 of its database's path.
fn profile_account(data_dir: &Path) -> String {
    let profile = data_dir.join("profile.db");
    hex::encode(Sha256::digest(profile.as_os_str().as_encoded_bytes()))
}

/// Whether opening the profile in `data_dir` makes the system ask the owner
/// before `vault` gives its secret: the window explains that first, the CLI
/// says so. A profile not made yet has no secret to read; a store that
/// cannot tell is read as before.
pub fn asks_before_opening(data_dir: &Path, vault: &dyn SecretStore) -> bool {
    fs::canonicalize(data_dir)
        .is_ok_and(|dir| vault.asks_first(&profile_account(&dir)).unwrap_or(false))
}
#[derive(Clone)]
pub struct HostConfig {
    pub data_dir: PathBuf,
    pub node_binary: PathBuf,
    pub listen: Vec<String>,
    /// More `kaiki-agentic-node serve` flags (bootstrap, chain, identity server).
    pub serve_args: Vec<String>,
}
pub struct DesktopHost {
    socket: PathBuf,
    token: Zeroizing<[u8; 32]>,
    process: Option<Child>,
}
struct Starting(Option<Child>);
impl Drop for Starting {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListenConfig {
    version: u8,
    addresses: Vec<String>,
}
/// The profile's lock while a daemon starts, and while its network or
/// release files change.
pub(crate) const START_LOCK: &str = ".bootstrap.lock";
/// Where agent hosts keep skills, under the home directory: Claude Code's,
/// then Codex's.
pub const SKILL_ROOTS: [&str; 2] = [".claude/skills", ".codex/skills"];

impl DesktopHost {
    #[cfg(unix)]
    pub async fn connect(config: HostConfig, vault: &dyn SecretStore) -> Result<Self> {
        Self::connect_with(config, vault, None).await
    }
    /// `connect`, and when the daemon has to be started, with the network
    /// of `preset` (asked for under the profile's start lock, never when a
    /// daemon already runs).
    #[cfg(unix)]
    pub async fn connect_with(
        mut config: HostConfig,
        vault: &dyn SecretStore,
        preset: Option<&PresetSource>,
    ) -> Result<Self> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&config.data_dir)?;
        config.data_dir = fs::canonicalize(&config.data_dir)?;
        if fs::metadata(&config.data_dir)?.permissions().mode() & 0o077 != 0 {
            return Err("profile directory must be private (0700)".into());
        }
        let socket = config.data_dir.join("node.sock");
        if socket.as_os_str().as_encoded_bytes().len() > 100 {
            return Err("profile path is too long for local IPC".into());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(config.data_dir.join(START_LOCK))?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match lock.try_lock_exclusive() {
                Ok(()) => break,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("another desktop launch is still starting".into());
                    }
                    tokio::time::sleep(Duration::from_millis(40)).await;
                }
                Err(error) => return Err(error.into()),
            }
        }
        let profile = config.data_dir.join("profile.db");
        let account = profile_account(&config.data_dir);
        let secret =
            match vault.get(&account)? {
                Some(value) if value.len() == 64 => value,
                Some(_) => return Err("invalid profile secret in system keychain".into()),
                None if profile.exists() => return Err(
                    "profile key is missing from system keychain; restore the key before opening"
                        .into(),
                ),
                None => {
                    let mut secret = Zeroizing::new(vec![0; 64]);
                    getrandom::fill(&mut secret).map_err(|_| "OS randomness unavailable")?;
                    vault.set(&account, &secret)?;
                    secret
                }
            };
        let token = Zeroizing::new(secret[32..].try_into().map_err(|_| "invalid owner key")?);
        let routes_file = config.data_dir.join("listen-addresses.json");
        let listen = match fs::read(&routes_file) {
            Ok(bytes) => {
                if bytes.len() > 4096 {
                    return Err("saved listener configuration exceeds limit".into());
                }
                let saved: ListenConfig = serde_json::from_slice(&bytes)?;
                if saved.version != 1 {
                    return Err("unsupported saved listener configuration".into());
                }
                saved.addresses
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => config.listen,
            Err(error) => return Err(error.into()),
        };
        if listen.is_empty() || listen.len() > 8 {
            return Err("provide 1..8 listen addresses".into());
        }
        let expected = listen
            .iter()
            .map(|address| address.parse::<Multiaddr>())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if let Ok(response) = ipc::call(&socket, &token, "node_info", json!({})).await {
            let info = result(response)?;
            if assigned_listeners(&info, &expected)?.is_none() {
                return Err("running daemon listeners are not ready".into());
            }
            return Ok(Self {
                socket,
                token,
                process: None,
            });
        }
        if preset.is_some() {
            config.serve_args = network_preset::resolve(&config.data_dir, preset)
                .await
                .serve_args();
        }
        let log = OpenOptions::new()
            .append(true)
            .create(true)
            .mode(0o600)
            .open(config.data_dir.join("node.log"))?;
        let mut command = Command::new(&config.node_binary);
        command
            .arg("serve")
            .arg("--profile")
            .arg(&profile)
            .arg("--ipc")
            .arg(&socket)
            .arg("--secrets-stdin");
        for address in &listen {
            command.arg("--listen").arg(address);
        }
        command.args(&config.serve_args);
        // The secret goes over stdin; a password for the secrets file stays
        // with the process that opened it.
        command
            .env_remove("AGENTIC_PASSWORD")
            .env_remove("AGENTIC_PASSWORD_FILE");
        let child = command
            .stdin(Stdio::piped())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?;
        let mut starting = Starting(Some(child));
        {
            let child = starting.0.as_mut().ok_or("node process unavailable")?;
            let mut input = child
                .stdin
                .take()
                .ok_or("node bootstrap pipe unavailable")?;
            let master = Zeroizing::new(hex::encode(&secret[..32]));
            let owner = Zeroizing::new(hex::encode(token.as_slice()));
            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct Input<'a> {
                master_key: &'a str,
                owner_token: &'a str,
            }
            let bytes = Zeroizing::new(serde_json::to_vec(&Input {
                master_key: &master,
                owner_token: &owner,
            })?);
            input.write_all(&bytes)?;
            input.write_all(b"\n")?;
        }
        drop(secret);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if starting
                .0
                .as_mut()
                .ok_or("node process unavailable")?
                .try_wait()?
                .is_some()
            {
                return Err("node exited during startup; inspect node.log".into());
            }
            if let Ok(response) = ipc::call(&socket, &token, "node_info", json!({})).await {
                let info = result(response)?;
                if let Some(addresses) = assigned_listeners(&info, &expected)? {
                    save_listeners(&routes_file, addresses)?;
                    return Ok(Self {
                        socket,
                        token,
                        process: starting.0.take(),
                    });
                }
            }
            if Instant::now() >= deadline {
                return Err("node did not bind its configured listeners".into());
            }
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    }
    /// The profile's running daemon, if any: never starts one and never
    /// creates a secret.
    #[cfg(unix)]
    pub async fn attach(data_dir: &Path, vault: &dyn SecretStore) -> Result<Option<Self>> {
        let Ok(data_dir) = fs::canonicalize(data_dir) else {
            return Ok(None);
        };
        let socket = data_dir.join("node.sock");
        let Some(secret) = vault.get(&profile_account(&data_dir))? else {
            return Ok(None);
        };
        if secret.len() != 64 {
            return Err("invalid profile secret".into());
        }
        let token = Zeroizing::new(secret[32..].try_into().map_err(|_| "invalid owner key")?);
        if ipc::call(&socket, &token, "node_info", json!({}))
            .await
            .is_err()
        {
            return Ok(None);
        }
        Ok(Some(Self {
            socket,
            token,
            process: None,
        }))
    }
    /// Transfers monitoring/reaping responsibility. Dropping a client or Child does not stop delivery.
    pub fn take_process(&mut self) -> Option<Child> {
        self.process.take()
    }
    pub async fn request(&self, method: &str, request: Value) -> Result<Value> {
        result(ipc::call(&self.socket, &self.token, method, request).await?)
    }
    /// The daemon's whole answer, `{"result"}` or `{"error": {"code", …}}`.
    pub async fn call(&self, method: &str, request: Value) -> Result<Value> {
        ipc::call(&self.socket, &self.token, method, request).await
    }
}
fn result(mut response: Value) -> Result<Value> {
    if let Some(error) = response.get("error") {
        return Err(error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("core command failed")
            .to_string()
            .into());
    }
    response
        .get_mut("result")
        .map(Value::take)
        .ok_or_else(|| "invalid core response".into())
}
/// Resolve ephemeral ports while retaining wildcard bindings for other local interfaces.
fn assigned_listeners(info: &Value, expected: &[Multiaddr]) -> Result<Option<Vec<String>>> {
    let peer = info["peerId"].as_str().ok_or("node omitted PeerID")?;
    let peer = peer.parse::<libp2p::PeerId>()?;
    let listeners = info["listeners"]
        .as_array()
        .ok_or("node omitted listeners")?;
    let mut actual = vec![];
    for value in listeners {
        let mut address: Multiaddr = value.as_str().ok_or("invalid listener")?.parse()?;
        if address.pop() != Some(Protocol::P2p(peer)) {
            return Err("listener has wrong PeerID".into());
        }
        actual.push(address);
    }
    if actual.len() < expected.len() {
        return Ok(None);
    }
    let mut output = vec![];
    for wanted in expected {
        let mut matched = None;
        for candidate in &actual {
            let wanted_parts = wanted.iter().collect::<Vec<_>>();
            let actual_parts = candidate.iter().collect::<Vec<_>>();
            if wanted_parts.len() != actual_parts.len() {
                continue;
            }
            let mut address = Multiaddr::empty();
            let mut matches = true;
            for (wanted, actual) in wanted_parts.into_iter().zip(actual_parts) {
                let part = match (&wanted, &actual) {
                    (Protocol::Ip4(ip), Protocol::Ip4(_)) if ip.is_unspecified() => wanted,
                    (Protocol::Ip6(ip), Protocol::Ip6(_)) if ip.is_unspecified() => wanted,
                    (Protocol::Tcp(0), Protocol::Tcp(port)) if *port != 0 => actual,
                    (Protocol::Udp(0), Protocol::Udp(port)) if *port != 0 => actual,
                    _ if wanted == actual => wanted,
                    _ => {
                        matches = false;
                        break;
                    }
                };
                address.push(part);
            }
            if matches {
                matched = Some(address.to_string());
                break;
            }
        }
        let Some(address) = matched else {
            return Ok(None);
        };
        output.push(address);
    }
    Ok(Some(output))
}
#[cfg(unix)]
fn save_listeners(path: &Path, addresses: Vec<String>) -> Result<()> {
    let bytes = serde_json::to_vec(&ListenConfig {
        version: 1,
        addresses,
    })?;
    let temporary = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)?;
    File::open(path.parent().ok_or("configuration requires a directory")?)?.sync_all()?;
    Ok(())
}

/// The desktop's keychain service and data directory name.
pub const SERVICE: &str = "net.agenticinternet.desktop";

/// `daemon start` flags, saved in `daemon.json`: every later start of the
/// profile's daemon uses them, whether the CLI or the window starts it.
#[derive(clap::Args, Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DaemonFlags {
    #[arg(long)]
    pub listen: Vec<String>,
    #[arg(long)]
    pub bootstrap: Vec<String>,
    #[arg(long)]
    pub chain_rpc: Option<String>,
    #[arg(long)]
    pub chain_id: Option<u64>,
    #[arg(long)]
    pub book_shop: Option<String>,
    #[arg(long)]
    pub grant_issuer: Option<String>,
    #[arg(long)]
    pub registry: Option<String>,
    #[arg(long)]
    pub chain_confirmations: Option<u64>,
    /// `OperatorPool`: an operator's node draws and withdraws its prizes.
    #[arg(long)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_pool: Option<String>,
    #[arg(long)]
    pub identity_server: Option<String>,
    /// The discovery service (spec/discovery-v1.md): the node tells the
    /// CLI and the window where it is (`discover_config`).
    #[arg(long)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// The key the discovery service signs bindings with, as the network
    /// names it: a service answering with another key is refused.
    #[arg(long, requires = "directory", value_parser = directory_key)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_key: Option<String>,
}

/// A discovery service's key: 32 bytes of hex.
fn directory_key(text: &str) -> std::result::Result<String, String> {
    let key = text.trim().to_ascii_lowercase();
    (key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()))
        .then_some(key)
        .ok_or_else(|| "a directory key is 32 bytes of hex".to_owned())
}

impl DaemonFlags {
    /// Whether any flag names a network (routes, chain or identity server):
    /// such a profile is set by hand and follows no preset.
    pub fn has_network(&self) -> bool {
        !self.bootstrap.is_empty()
            || self.chain_rpc.is_some()
            || self.chain_id.is_some()
            || self.book_shop.is_some()
            || self.grant_issuer.is_some()
            || self.registry.is_some()
            || self.chain_confirmations.is_some()
            || self.operator_pool.is_some()
            || self.identity_server.is_some()
    }

    /// The profile's saved flags; none when nothing was saved.
    pub fn load(data_dir: &Path) -> Self {
        fs::read(data_dir.join("daemon.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Saves the flags; a new profile directory is made private, as the
    /// daemon requires.
    #[cfg(unix)]
    pub fn save(&self, data_dir: &Path) -> std::io::Result<()> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(data_dir)?;
        fs::write(
            data_dir.join("daemon.json"),
            serde_json::to_vec(self).map_err(std::io::Error::other)?,
        )
    }

    /// The `kaiki-agentic-node serve` flags besides the listen addresses.
    pub fn serve_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        for bootstrap in &self.bootstrap {
            args.extend(["--bootstrap".to_owned(), bootstrap.clone()]);
        }
        let options = [
            ("--chain-rpc", self.chain_rpc.clone()),
            ("--chain-id", self.chain_id.map(|id| id.to_string())),
            ("--book-shop", self.book_shop.clone()),
            ("--grant-issuer", self.grant_issuer.clone()),
            ("--registry", self.registry.clone()),
            (
                "--chain-confirmations",
                self.chain_confirmations.map(|n| n.to_string()),
            ),
            ("--operator-pool", self.operator_pool.clone()),
            ("--identity-server", self.identity_server.clone()),
            ("--directory", self.directory.clone()),
            ("--directory-key", self.directory_key.clone()),
        ];
        for (flag, value) in options {
            if let Some(value) = value {
                args.extend([flag.to_owned(), value]);
            }
        }
        args
    }
}

/// Where a profile's secret lives (spec/owner-cli-v1.md, "Data and secrets").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretsBackend {
    Keychain,
    File,
}

impl SecretsBackend {
    /// `AGENTIC_SECRETS`, else the keychain on macOS, and on Linux the
    /// Secret Service when one answers and the password file otherwise.
    pub fn chosen() -> Self {
        match std::env::var("AGENTIC_SECRETS").ok().as_deref() {
            Some("file") => Self::File,
            Some("keychain") => Self::Keychain,
            _ => Self::default_here(),
        }
    }

    pub fn default_here() -> Self {
        if cfg!(target_os = "macos") || KeychainStore::new(SERVICE).get("probe").is_ok() {
            Self::Keychain
        } else {
            Self::File
        }
    }
}

/// The profile password from `AGENTIC_PASSWORD`, or the first line of the
/// file `AGENTIC_PASSWORD_FILE`.
pub fn password_from_env() -> Option<Zeroizing<String>> {
    if let Ok(password) = std::env::var("AGENTIC_PASSWORD") {
        return Some(Zeroizing::new(password));
    }
    let file = std::env::var_os("AGENTIC_PASSWORD_FILE")?;
    let text = Zeroizing::new(fs::read_to_string(file).ok()?);
    Some(Zeroizing::new(text.lines().next()?.to_owned()))
}

/// `AGENTIC_DATA_DIR`, else the desktop app's data directory.
pub fn default_data_dir() -> Option<PathBuf> {
    std::env::var_os("AGENTIC_DATA_DIR")
        .map(PathBuf::from)
        .or_else(platform_data_dir)
}

/// The desktop app's data directory: the profile the CLI opens when neither
/// `--data-dir` nor `AGENTIC_DATA_DIR` names another.
pub fn platform_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(SERVICE))
}

/// The owner's skill as written (integrations/agent-skill/kaiki).
pub const OWNER_SKILL: &str = include_str!("../../../integrations/agent-skill/kaiki/SKILL.md");

/// One word for a POSIX shell: a path, or anything but plain characters,
/// in single quotes.
fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.,:=@%+-".contains(c));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// The owner's skill for this computer: one line after its title names the
/// command, the CLI `cli` with the profile's `args`, since the app's bundle
/// is not on PATH.
pub fn owner_skill(cli: &Path, args: &[String]) -> String {
    let command = std::iter::once(shell_word(&cli.to_string_lossy()))
        .chain(args.iter().map(|arg| shell_word(arg)))
        .collect::<Vec<_>>()
        .join(" ");
    let line = format!(
        "On this computer the CLI is `{command}`: run it wherever this skill says `kaiki`. If that file is gone, ask the owner to open Kaiki Chat once: the app writes its new place here."
    );
    let mut text = String::with_capacity(OWNER_SKILL.len() + line.len() + 2);
    let mut placed = false;
    for piece in OWNER_SKILL.split_inclusive('\n') {
        text.push_str(piece);
        if !placed && piece.starts_with("# ") {
            text.push('\n');
            text.push_str(&line);
            text.push('\n');
            placed = true;
        }
    }
    text
}

/// Use the profile's running daemon, or start it with its saved flags and
/// the network of `preset` (asked for only then): the one way the CLI and the
/// window open a profile. `listen` is used when no listen addresses were
/// saved.
#[cfg(unix)]
pub async fn connect_profile(
    data_dir: &Path,
    node_binary: PathBuf,
    listen: &[String],
    vault: &dyn SecretStore,
    preset: Option<&PresetSource>,
) -> Result<DesktopHost> {
    let flags = DaemonFlags::load(data_dir);
    DesktopHost::connect_with(
        HostConfig {
            data_dir: data_dir.into(),
            node_binary,
            listen: if flags.listen.is_empty() {
                listen.to_vec()
            } else {
                flags.listen.clone()
            },
            serve_args: flags.serve_args(),
        },
        vault,
        preset,
    )
    .await
}

/// Stops the profile's running daemon, if any, and waits until it released
/// the profile: then a new start cannot race the old one.
#[cfg(unix)]
pub async fn stop_profile(data_dir: &Path, vault: &dyn SecretStore) -> Result<()> {
    let Some(host) = DesktopHost::attach(data_dir, vault).await? else {
        return Ok(());
    };
    host.request("shutdown", json!({})).await?;
    let socket = data_dir.join("node.sock");
    let deadline = Instant::now() + Duration::from_secs(20);
    while socket.exists() {
        if Instant::now() >= deadline {
            return Err("the daemon did not stop".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

/// Starts the profile's daemon again with a fresh look at the preset; with
/// `switch`, first takes the network offered to it (`NoNetworkOffer` when
/// none is, and the daemon keeps running).
#[cfg(unix)]
pub async fn restart_profile(
    data_dir: &Path,
    node_binary: PathBuf,
    listen: &[String],
    vault: &dyn SecretStore,
    preset: Option<&PresetSource>,
    switch: bool,
) -> Result<DesktopHost> {
    // Without an offer nothing stops; the offer is taken only once the old
    // daemon is gone, so the profile never names a network no daemon runs.
    let offer = match (switch, preset) {
        (false, _) => None,
        (true, Some(preset)) => {
            network_preset::offered(data_dir, preset)?;
            Some(preset)
        }
        (true, None) => return Err(NoNetworkOffer.into()),
    };
    stop_profile(data_dir, vault).await?;
    if let Some(preset) = offer
        && let Err(error) = network_preset::accept_offer(data_dir, preset)
    {
        tracing::warn!("the offered network was not taken: {error}");
    }
    connect_profile(data_dir, node_binary, listen, vault, preset).await
}

/// How far a node reaches the network, from its `node_info`: what
/// `kaiki daemon status` shows as `network`.
pub fn network_status(info: &Value) -> Value {
    let swarm = &info["mailboxSwarm"];
    json!({
        "bootstrap": info["bootstrap"]["state"],
        "connectedPeers": info["peerConnections"].as_array().map_or(0, Vec::len),
        "holders": swarm["holders"],
        "letIn": swarm["access"]["accepted"],
        "cardStored": !swarm["intro"]["published"].is_null(),
        "failures": swarm["failureKinds"],
        "lastFailure": swarm["lastFailure"],
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod skill_tests {
    use super::*;

    /// The words a shell reads from the first `…` span of `line`.
    fn shell_words(line: &str) -> Vec<String> {
        let span = line.split('`').nth(1).expect("a code span");
        let printed = Command::new("sh")
            .arg("-c")
            .arg(format!("printf '%s\\n' {span}"))
            .output()
            .unwrap();
        String::from_utf8(printed.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// The app's bundle path has spaces and may have a quote, a profile
    /// directory too: the skill adds one line naming the command as shell
    /// words, and is otherwise the skill as written.
    #[test]
    fn the_owner_skill_names_the_cli_as_shell_words() {
        let cli = "/Applications/Kaiki Chat.app/Contents/MacOS/it's/kaiki";
        let profile = ["--data-dir".to_owned(), "/Users/o/Test profile".to_owned()];
        let line = |text: &str| {
            let source: Vec<&str> = OWNER_SKILL.lines().collect();
            let added: Vec<String> = text
                .lines()
                .filter(|line| !line.is_empty() && !source.contains(line))
                .map(str::to_owned)
                .collect();
            assert_eq!(added.len(), 1, "{text}");
            added[0].clone()
        };
        let other = owner_skill(Path::new(cli), &profile);
        assert_eq!(
            shell_words(&line(&other)),
            [cli, "--data-dir", "/Users/o/Test profile"]
        );
        let text = owner_skill(Path::new(cli), &[]);
        assert!(text.starts_with("---\nname: kaiki\n"), "{text}");
        let source: Vec<&str> = OWNER_SKILL.lines().filter(|l| !l.is_empty()).collect();
        let added: Vec<&str> = text
            .lines()
            .filter(|line| !line.is_empty() && !source.contains(line))
            .collect();
        assert_eq!(added.len(), 1, "{text}");
        let rest: Vec<&str> = text
            .lines()
            .filter(|line| !line.is_empty() && *line != added[0])
            .collect();
        assert_eq!(rest, source);
        // The first `…` span of the added line, run by a shell, is the path.
        assert_eq!(shell_words(&line(&text)), [cli]);
    }
}

#[cfg(test)]
mod network_status_tests {
    use super::*;

    /// A fresh client on the testnet, 2026-09-29: the holders' records listed
    /// only docker bridges, so it reached the four holders its preset named.
    #[test]
    fn the_status_shows_a_client_that_reaches_four_of_ten_holders() {
        let info = json!({
            "peerId": "12D3KooWClient",
            "bootstrap": {"state": "connected", "failedAttempts": 3, "verifiedPeers": [{}, {}, {}, {}]},
            "peerConnections": [{}, {}, {}, {}, {}],
            "routing": {"knownPeers": ["a", "b", "c", "d"]},
            "mailboxSwarm": {
                "holders": 10,
                "access": {"accepted": 4, "refused": {"unknown_book": 4}, "waiting": 6, "credential": "book"},
                "failureKinds": {"access": 870, "dial": 900},
                "lastFailure": "dd63efae: access: Failed to dial the requested peer",
                "intro": {"published": null, "swarm": {"joined": 0}},
                "stored": 0,
            },
        });
        assert_eq!(
            network_status(&info),
            json!({
                "bootstrap": "connected",
                "connectedPeers": 5,
                "holders": 10,
                "letIn": 4,
                "cardStored": false,
                "failures": {"access": 870, "dial": 900},
                "lastFailure": "dd63efae: access: Failed to dial the requested peer",
            })
        );
    }

    /// Once every holder lets it in, its card is stored at a quorum.
    #[test]
    fn the_status_shows_a_stored_card() {
        let info = json!({
            "bootstrap": {"state": "connected"},
            "peerConnections": [],
            "mailboxSwarm": {
                "holders": 10,
                "access": {"accepted": 10},
                "failureKinds": {},
                "lastFailure": null,
                "intro": {"published": "card:1ee7:20725"},
            },
        });
        let status = network_status(&info);
        assert_eq!(
            (status["letIn"].as_u64(), status["cardStored"].as_bool()),
            (Some(10), Some(true)),
            "{status}"
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
#[allow(clippy::unwrap_used)]
mod keychain_dialog_tests {
    use super::needs_dialog;
    use apple_native_keyring_store::keychain::decode_error;
    use security_framework::base::Error;

    /// What the keyring crate hands over when the keychain answers with its
    /// dialogs off: a refusal the dialog would lift (this program is not
    /// trusted by the item, errSecAuthFailed; or the keychain waits to be
    /// unlocked, errSecInteractionNotAllowed) needs the dialog; no other
    /// answer does.
    #[test]
    fn only_refusals_the_dialog_would_lift_need_it() {
        for code in [-25293, -25308] {
            assert!(
                needs_dialog(&decode_error(Error::from_code(code))),
                "{code}"
            );
        }
        // No item yet (a new profile), no keychain at all, and another
        // platform failure the keyring crate passes on the same way as the
        // refusals (a cancelled dialog): nothing the dialog would lift.
        for code in [-25300, -25294, -128] {
            assert!(
                !needs_dialog(&decode_error(Error::from_code(code))),
                "{code}"
            );
        }
    }

    /// The quiet read gives the keychain its dialogs back: else the owner's
    /// "Continue" would never bring up macOS's dialog. Reads the login
    /// keychain (an account nobody has; nothing is written or shown), so it
    /// runs only when asked.
    #[test]
    #[ignore = "reads the login keychain; run explicitly"]
    fn a_quiet_read_gives_the_keychain_its_dialogs_back() {
        use super::{KeychainStore, SecretStore};
        use security_framework::os::macos::keychain::SecKeychain;
        let store = KeychainStore::new("net.agenticinternet.desktop.tests");
        assert!(!store.asks_first("no-such-account").unwrap());
        assert!(SecKeychain::user_interaction_allowed().unwrap());
    }
}
