//! The native side of the owner's window (spec/desktop-gui-v1.md): it opens
//! the profile and its daemon like the owner CLI, and gives the webview only
//! the listed commands, each forwarding one daemon method. Secrets stay here.
use agentic_desktop_host::autostart::Autostart;
use agentic_desktop_host::network_preset::{
    self, NoNetworkOffer, PresetSource, PresetStatus, RELEASE_CHECK_EVERY, ReleaseStatus,
};
use agentic_desktop_host::update::{self, UpdateError};
use agentic_desktop_host::{
    DesktopHost, PasswordFileStore, SKILL_ROOTS, SecretStore, SecretsLocked, connect_profile,
    discover, owner_skill, platform_data_dir, restart_profile, stop_profile,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, Runtime, State, WebviewWindow};
use zeroize::Zeroizing;

/// Daemon answers that ask to try again, as for the CLI; and a stopped daemon.
const RETRYABLE: [&str; 6] = [
    "chain_pending",
    "claim_pending",
    "network_unavailable",
    "card_pending",
    "group_busy",
    "daemon_unavailable",
];
/// How long a command waits while the node looks for members' cards.
const CARD_WAIT: Duration = Duration::from_secs(75);
/// Where the app is downloaded, for a window that cannot replace itself.
const DOWNLOADS: &str = "https://kaikichat.com/#get";

/// A refusal as the webview gets it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommandError {
    code: String,
    message: String,
    retryable: bool,
}
impl CommandError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: RETRYABLE.contains(&code),
        }
    }
    fn invalid(message: &str) -> Self {
        Self::new("invalid_request", message)
    }
}
type Answer = std::result::Result<Value, CommandError>;

impl From<discover::Refusal> for CommandError {
    fn from(refusal: discover::Refusal) -> Self {
        Self {
            code: refusal.code,
            message: refusal.message,
            retryable: refusal.retryable,
        }
    }
}

/// Opens a link in the system's browser or wallet.
pub trait Opener: Send + Sync {
    fn open(&self, link: &str) -> std::result::Result<(), String>;
}
/// `open` on macOS, `xdg-open` elsewhere.
pub struct SystemOpener;
impl Opener for SystemOpener {
    fn open(&self, link: &str) -> std::result::Result<(), String> {
        let program = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(program)
            .arg(link)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
/// Refuses every link: a window made without an opener.
struct NoOpener;
impl Opener for NoOpener {
    fn open(&self, _: &str) -> std::result::Result<(), String> {
        Err("links cannot be opened here".into())
    }
}

/// Where the profile's secret is.
pub enum Secrets {
    /// The system keychain or the Secret Service (or a stand-in for it).
    Store(Arc<dyn SecretStore>),
    /// `secrets.json`, sealed with a password; without one the window asks.
    File { password: Option<Zeroizing<String>> },
}

pub struct ProfileConfig {
    pub data_dir: PathBuf,
    pub node_binary: PathBuf,
    /// Listen addresses when the profile saved none.
    pub listen: Vec<String>,
    pub secrets: Secrets,
    /// The home that skills are installed under.
    pub home: PathBuf,
    pub opener: Arc<dyn Opener>,
    /// Where the profile's network comes from; none for a network set by
    /// hand or none at all.
    pub preset: Option<PresetSource>,
    /// The app opening at login; none where it cannot (a build, E2E, a
    /// profile named by `AGENTIC_DATA_DIR`).
    pub autostart: Option<Autostart>,
}

struct Profile {
    data_dir: PathBuf,
    node_binary: PathBuf,
    listen: Vec<String>,
    file: bool,
    preset: Option<PresetSource>,
}

#[derive(Default)]
struct Link {
    /// The secret's store once known (for a file profile, once unlocked).
    vault: Option<Arc<dyn SecretStore>>,
    host: Option<Arc<DesktopHost>>,
    /// Why the daemon could not be opened.
    failure: Option<CommandError>,
}

pub struct NativeBridge {
    profile: Option<Profile>,
    link: tokio::sync::Mutex<Link>,
    opener: Arc<dyn Opener>,
    home: Option<PathBuf>,
    /// The app's own binary: in a macOS app bundle, the app replaces itself.
    app: Option<PathBuf>,
    /// Payment URIs the daemon made in this session, by book and step.
    payments: std::sync::Mutex<HashMap<String, HashMap<PaymentStep, String>>>,
    autostart: Option<Autostart>,
}

impl NativeBridge {
    /// A window on a daemon opened elsewhere; it cannot unlock or restart it.
    pub fn connected(host: Arc<DesktopHost>) -> Self {
        Self {
            profile: None,
            link: tokio::sync::Mutex::new(Link {
                host: Some(host),
                ..Link::default()
            }),
            opener: Arc::new(NoOpener),
            home: None,
            app: None,
            payments: Default::default(),
            autostart: None,
        }
    }
    pub fn unavailable(error: String) -> Self {
        Self {
            profile: None,
            link: tokio::sync::Mutex::new(Link {
                failure: Some(CommandError::new("daemon_unavailable", error)),
                ..Link::default()
            }),
            opener: Arc::new(NoOpener),
            home: None,
            app: None,
            payments: Default::default(),
            autostart: None,
        }
    }
    pub fn with_opener(mut self, opener: Arc<dyn Opener>) -> Self {
        self.opener = opener;
        self
    }
    pub fn with_home(mut self, home: PathBuf) -> Self {
        self.home = Some(home);
        self
    }
    /// The binary the app runs as, when not this process's own.
    pub fn with_app(mut self, app: PathBuf) -> Self {
        self.app = Some(app);
        self
    }

    /// The profile as the app starts: its daemon started or joined when the
    /// secret is at hand, else locked until the password is given.
    pub async fn open(config: ProfileConfig) -> Self {
        let (vault, file): (Option<Arc<dyn SecretStore>>, bool) = match config.secrets {
            Secrets::Store(store) => (Some(store), false),
            Secrets::File {
                password: Some(password),
            } => (
                Some(Arc::new(PasswordFileStore::new(&config.data_dir, password))),
                true,
            ),
            Secrets::File { password: None } => (None, true),
        };
        let bridge = Self {
            profile: Some(Profile {
                data_dir: config.data_dir,
                node_binary: config.node_binary,
                listen: config.listen,
                file,
                preset: config.preset,
            }),
            link: Default::default(),
            opener: config.opener,
            home: Some(config.home),
            app: std::env::current_exe().ok(),
            payments: Default::default(),
            autostart: config.autostart,
        };
        bridge.refresh_skills();
        // The installed app opens at login, unless the owner turned it off.
        if let Some(autostart) = &bridge.autostart
            && let Err(error) = autostart.ensure()
        {
            eprintln!("The app was not put into the login items: {error}");
        }
        if let Some(vault) = vault {
            let mut link = bridge.link.lock().await;
            bridge.start(&mut link, vault).await;
        }
        bridge
    }

    /// Opens the daemon with `vault`; a store that does not open is dropped.
    async fn start(&self, link: &mut Link, vault: Arc<dyn SecretStore>) -> Option<CommandError> {
        let profile = self.profile.as_ref()?;
        match connect_profile(
            &profile.data_dir,
            profile.node_binary.clone(),
            &profile.listen,
            vault.as_ref(),
            profile.preset.as_ref(),
        )
        .await
        {
            Ok(host) => {
                *link = Link {
                    vault: Some(vault),
                    host: Some(Arc::new(host)),
                    failure: None,
                };
                None
            }
            Err(error) if error.downcast_ref::<SecretsLocked>().is_some() => Some(
                CommandError::new("secrets_locked", "The password does not open this profile"),
            ),
            Err(error) => {
                let failure = CommandError::new("daemon_unavailable", error.to_string());
                *link = Link {
                    vault: Some(vault),
                    host: None,
                    failure: Some(failure.clone()),
                };
                Some(failure)
            }
        }
    }

    /// One daemon method; never starts a daemon.
    pub async fn request(&self, method: &str, request: Value) -> Answer {
        let host = {
            let link = self.link.lock().await;
            match (&link.host, &link.vault, &self.profile) {
                (Some(host), _, _) => host.clone(),
                (None, None, Some(profile)) if profile.file => {
                    return Err(CommandError::new(
                        "profile_locked",
                        "Give the profile's password to open it",
                    ));
                }
                _ => {
                    return Err(link.failure.clone().unwrap_or_else(|| {
                        CommandError::new("daemon_unavailable", "The daemon is not running")
                    }));
                }
            }
        };
        let answer = host.call(method, request).await.map_err(|_| {
            CommandError::new("daemon_unavailable", "The profile's daemon is not running")
        })?;
        if let Some(error) = answer.get("error") {
            let code = error["code"].as_str().unwrap_or("core_error");
            let message = error["message"].as_str().unwrap_or("core command failed");
            return Err(CommandError::new(code, message));
        }
        answer
            .get("result")
            .cloned()
            .ok_or_else(|| CommandError::new("core_error", "invalid core response"))
    }

    /// A daemon method answered once the members' cards are read.
    async fn when_cards_are_read(&self, method: &str, request: Value) -> Answer {
        let deadline = Instant::now() + CARD_WAIT;
        loop {
            match self.request(method, request.clone()).await {
                Err(error) if error.code == "card_pending" && Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                answer => return answer,
            }
        }
    }

    async fn status(&self) -> Value {
        let (connected, locked, failure, secrets) = {
            let link = self.link.lock().await;
            let file = self.profile.as_ref().is_some_and(|p| p.file);
            (
                link.host.clone(),
                file && link.vault.is_none(),
                link.failure.clone(),
                if file { "file" } else { "keychain" },
            )
        };
        if locked {
            let fresh = self
                .profile
                .as_ref()
                .is_some_and(|p| !p.data_dir.join("secrets.json").exists());
            return json!({"state":"locked","secrets":secrets,"newProfile":fresh});
        }
        let alive = match connected {
            Some(host) => host.call("desktop_revision", json!({})).await.is_ok(),
            None => false,
        };
        if alive {
            json!({"state":"connected","secrets":secrets})
        } else {
            json!({"state":"unavailable","secrets":secrets,"error":failure})
        }
    }

    async fn unlock(&self, password: Zeroizing<String>) -> Answer {
        let Some(profile) = self.profile.as_ref().filter(|p| p.file) else {
            return Err(CommandError::invalid(
                "This profile's secret is not in a password file",
            ));
        };
        if password.is_empty() {
            return Err(CommandError::invalid("The password is empty"));
        }
        let mut link = self.link.lock().await;
        if link.host.is_none() {
            let vault = Arc::new(PasswordFileStore::new(&profile.data_dir, password));
            if let Some(error) = self.start(&mut link, vault).await {
                return Err(error);
            }
        }
        drop(link);
        Ok(self.status().await)
    }

    /// Starts the daemon again (or joins one), when the owner asks.
    async fn reconnect(&self) -> Answer {
        if self.profile.is_none() {
            return Err(CommandError::invalid("This window cannot start the daemon"));
        }
        let mut link = self.link.lock().await;
        let Some(vault) = link.vault.clone() else {
            return Err(CommandError::new(
                "profile_locked",
                "Give the profile's password to open it",
            ));
        };
        if let Some(error) = self.start(&mut link, vault).await {
            return Err(error);
        }
        drop(link);
        Ok(self.status().await)
    }

    /// The profile's network as last checked.
    fn network(&self) -> Value {
        let status = match &self.profile {
            Some(profile) => network_preset::status(&profile.data_dir, profile.preset.as_ref()),
            None => PresetStatus::off(),
        };
        serde_json::to_value(status).unwrap_or(Value::Null)
    }

    /// The latest release, asked for when the last check is older than
    /// `every` (now for none), and whether this app can install it.
    async fn release(&self, every: Option<Duration>) -> Answer {
        let status = match self.profile.as_ref() {
            Some(Profile {
                data_dir,
                preset: Some(source),
                ..
            }) => network_preset::check_release(data_dir, source, every).await,
            Some(profile) => network_preset::release_status(&profile.data_dir, None),
            None => ReleaseStatus::unknown(),
        };
        self.release_answer(status)
    }

    fn release_answer(&self, status: ReleaseStatus) -> Answer {
        let installable = self.app_build().is_some();
        let mut answer = serde_json::to_value(status)
            .map_err(|error| CommandError::new("io_error", error.to_string()))?;
        answer["installable"] = installable.into();
        Ok(answer)
    }

    /// The build of the latest release this app replaces itself with: its
    /// platform's app build, for an app in a bundle.
    fn app_build(&self) -> Option<(PathBuf, String, network_preset::Build)> {
        let profile = self.profile.as_ref()?;
        let app = self
            .app
            .clone()
            .filter(|app| update::app_bundle(app).is_some())?;
        let release = network_preset::latest_release(&profile.data_dir, profile.preset.as_ref()?)?;
        let build = release
            .builds
            .get(&format!("app-{}", update::platform()?))?
            .clone();
        Some((app, release.version, build))
    }

    fn skip_release(&self, version: &str) -> Answer {
        let Some(Profile {
            data_dir,
            preset: Some(source),
            ..
        }) = self.profile.as_ref()
        else {
            return Err(CommandError::new("no_update", "No release is known"));
        };
        let status = network_preset::skip_release(data_dir, source, version)
            .map_err(|error| CommandError::invalid(&error.to_string()))?;
        self.release_answer(status)
    }

    /// Replaces the app with the latest release and stops the daemon, which
    /// the new app starts again; the caller restarts the app.
    async fn install_update(&self) -> Answer {
        let Some(profile) = self.profile.as_ref() else {
            return Err(CommandError::invalid("This window cannot update the app"));
        };
        let status = match &profile.preset {
            Some(source) => network_preset::release_status(&profile.data_dir, Some(source)),
            None => network_preset::release_status(&profile.data_dir, None),
        };
        if !status.available {
            return Err(CommandError::new("no_update", "No newer release is known"));
        }
        let Some((app, version, build)) = self.app_build() else {
            let code = if self.app.as_deref().and_then(update::app_bundle).is_some() {
                "no_build"
            } else {
                "not_updatable"
            };
            return Err(CommandError::new(
                code,
                "This app cannot replace itself; download the new version",
            ));
        };
        update::install_app(&app, &build).await.map_err(|error| {
            match error.downcast_ref::<UpdateError>() {
                Some(refusal) => CommandError::new(refusal.code, refusal.message.clone()),
                None => CommandError::new("download_failed", error.to_string()),
            }
        })?;
        let link = self.link.lock().await;
        if let Some(vault) = link.vault.as_ref()
            && let Err(error) = stop_profile(&profile.data_dir, vault.as_ref()).await
        {
            eprintln!("The daemon was not stopped for the update: {error}");
        }
        Ok(json!({ "version": version }))
    }

    /// Whether the app opens at login; null where it cannot.
    fn autostart_status(&self) -> Answer {
        match &self.autostart {
            Some(autostart) => serde_json::to_value(autostart.status())
                .map_err(|error| CommandError::new("io_error", error.to_string())),
            None => Ok(Value::Null),
        }
    }

    fn login_items(&self) -> std::result::Result<&Autostart, CommandError> {
        self.autostart.as_ref().ok_or_else(|| {
            CommandError::new(
                "autostart_unavailable",
                "Only the installed app opens at login",
            )
        })
    }

    fn set_autostart(&self, on: bool) -> Answer {
        let autostart = self.login_items()?;
        let status = if on {
            autostart.enable()
        } else {
            autostart.disable()
        }
        .map_err(|error| CommandError::new("io_error", error.to_string()))?;
        serde_json::to_value(status)
            .map_err(|error| CommandError::new("io_error", error.to_string()))
    }

    fn open_login_items(&self) -> Answer {
        self.login_items()?
            .ask()
            .map_err(|error| CommandError::new("open_failed", error.to_string()))?;
        Ok(Value::Null)
    }

    fn open_downloads(&self) -> Answer {
        self.opener
            .open(DOWNLOADS)
            .map_err(|error| CommandError::new("open_failed", error))?;
        Ok(Value::Null)
    }

    /// Starts the daemon again with a fresh look at the preset; with
    /// `switch`, on the network the preset offered.
    async fn refresh_network(&self, switch: bool) -> Answer {
        let Some(profile) = self.profile.as_ref() else {
            return Err(CommandError::invalid("This window cannot start the daemon"));
        };
        let mut link = self.link.lock().await;
        let Some(vault) = link.vault.clone() else {
            return Err(CommandError::new(
                "profile_locked",
                "Give the profile's password to open it",
            ));
        };
        match restart_profile(
            &profile.data_dir,
            profile.node_binary.clone(),
            &profile.listen,
            vault.as_ref(),
            profile.preset.as_ref(),
            switch,
        )
        .await
        {
            Ok(host) => {
                *link = Link {
                    vault: Some(vault),
                    host: Some(Arc::new(host)),
                    failure: None,
                };
            }
            Err(error) if error.downcast_ref::<NoNetworkOffer>().is_some() => {
                return Err(CommandError::new("no_network_offer", error.to_string()));
            }
            Err(error) => {
                let failure = CommandError::new("daemon_unavailable", error.to_string());
                link.host = None;
                link.failure = Some(failure.clone());
                return Err(failure);
            }
        }
        drop(link);
        Ok(self.network())
    }

    /// A book to buy; its payment calls are kept for `open_payment`, the
    /// latest answer for a book replacing an earlier one (the ETH quote moves).
    async fn buy(&self) -> Answer {
        let payment = self.request("coins_buy", json!({})).await?;
        if let Some(book) = payment["book"].as_str() {
            let steps: HashMap<PaymentStep, String> = [
                (PaymentStep::Eth, &payment["eth"]["uri"]),
                (PaymentStep::Approve, &payment["usdc"]["approve"]["uri"]),
                (PaymentStep::Buy, &payment["usdc"]["buy"]["uri"]),
            ]
            .into_iter()
            .filter_map(|(step, uri)| Some((step, uri.as_str()?.to_owned())))
            .collect();
            if let Ok(mut payments) = self.payments.lock() {
                payments.insert(book.to_owned(), steps);
            }
        }
        Ok(payment)
    }

    fn open_payment(&self, book: &str, step: PaymentStep) -> Answer {
        let uri = self
            .payments
            .lock()
            .ok()
            .and_then(|payments| payments.get(book)?.get(&step).cloned())
            .filter(|uri| uri.starts_with("ethereum:"))
            .ok_or_else(|| {
                CommandError::new(
                    "unknown_payment",
                    "No payment for this book in this session",
                )
            })?;
        self.opener
            .open(&uri)
            .map_err(|error| CommandError::new("open_failed", error))?;
        Ok(Value::Null)
    }

    /// The daemon's login link, opened at the provider the owner chose.
    async fn claim(&self, provider: Provider) -> Answer {
        let claim = self.request("coins_claim", json!({})).await?;
        let link = claim["loginUrl"].as_str().unwrap_or_default();
        if !web_link(link) {
            return Err(CommandError::new(
                "unsafe_link",
                "The identity server's login link is not a web page",
            ));
        }
        let at = match provider {
            Provider::Google => "google",
            Provider::Github => "github",
        };
        self.opener
            .open(&format!("{}/{at}", link.trim_end_matches('/')))
            .map_err(|error| CommandError::new("open_failed", error))?;
        Ok(claim)
    }

    /// Addresses and GitHub logins in pasted text or a contacts file, in
    /// their normal form.
    fn discover_handles(&self, request: TextRequest) -> Answer {
        if request.text.len() > MAX_ADDRESS_TEXT {
            return Err(CommandError::invalid("The text is too long"));
        }
        let handles: Vec<Value> = discover::handles_from(&request.text)
            .into_iter()
            .map(|(kind, handle)| json!({"kind": kind, "handle": handle}))
            .collect();
        Ok(json!({ "handles": handles }))
    }

    async fn discover_lookup(&self, request: LookupRequest) -> Answer {
        let handles: Option<Vec<(String, String)>> = request
            .handles
            .iter()
            .map(|h| {
                discover::normalize_handle(h.kind.name(), &h.handle)
                    .map(|normal| (h.kind.name().to_owned(), normal))
            })
            .collect();
        let handles = handles
            .filter(|handles| !handles.is_empty())
            .ok_or_else(|| CommandError::invalid("Not an address or a GitHub login"))?;
        discover::lookup(self, &handles).await
    }

    /// A login link of the discovery service, opened in the browser only
    /// when it is a web page.
    async fn discover_link(&self, kind: Provider) -> Answer {
        let opened = discover::link(self, kind.name()).await?;
        let link = opened["loginUrl"].as_str().unwrap_or_default();
        if !web_link(link) {
            return Err(CommandError::new(
                "unsafe_link",
                "The discovery service's login link is not a web page",
            ));
        }
        self.opener
            .open(link)
            .map_err(|error| CommandError::new("open_failed", error))?;
        Ok(opened)
    }

    /// The owner CLI `kaiki` beside the app's `agentic-node`: the app's
    /// bundle is not on PATH.
    /// It runs on this profile with the returned arguments.
    fn cli(&self) -> std::result::Result<(PathBuf, Vec<String>), CommandError> {
        let profile = self.profile.as_ref().ok_or_else(|| {
            CommandError::new("cli_unavailable", "This window does not know the owner CLI")
        })?;
        Ok((
            profile.node_binary.with_file_name("kaiki"),
            profile_args(&profile.data_dir, platform_data_dir().as_deref()),
        ))
    }

    /// How an agent runs the owner CLI on this profile.
    fn owner_cli(&self) -> Answer {
        let (command, args) = self.cli()?;
        Ok(json!({"command": command, "args": args}))
    }

    /// Points every installed `kaiki` skill at the CLI this app ships: the
    /// one an older app wrote may name a bundle that moved or is gone.
    /// Installs none the owner did not install.
    fn refresh_skills(&self) {
        let (Some(home), Ok((cli, args))) = (self.home.as_ref(), self.cli()) else {
            return;
        };
        let text = owner_skill(&cli, &args);
        for root in SKILL_ROOTS {
            let path = home.join(root).join("kaiki").join("SKILL.md");
            match std::fs::read_to_string(&path) {
                Ok(installed) if installed != text => {
                    if let Err(error) = write_file(&path, &text) {
                        eprintln!("Could not update {}: {error}", path.display());
                    }
                }
                _ => {}
            }
        }
    }

    fn install_skill(&self, skill: Skill, host: SkillHost) -> Answer {
        let home = self
            .home
            .as_ref()
            .ok_or_else(|| CommandError::invalid("No home directory"))?;
        let (name, text) = match skill {
            Skill::Kaiki => {
                let (cli, args) = self.cli()?;
                ("kaiki", owner_skill(&cli, &args))
            }
            Skill::AgenticMessaging => ("agentic-messaging", AGENT_SKILL.to_owned()),
        };
        let root = match host {
            SkillHost::Claude => SKILL_ROOTS[0],
            SkillHost::Codex => SKILL_ROOTS[1],
        };
        let path = home.join(root).join(name).join("SKILL.md");
        write_file(&path, &text)
            .map_err(|error| CommandError::new("io_error", error.to_string()))?;
        Ok(json!({"path": path}))
    }
}

const AGENT_SKILL: &str =
    include_str!("../../../../integrations/agent-skill/agentic-messaging/SKILL.md");

fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)
}

/// A link the browser may open: https, or http to this machine.
fn web_link(link: &str) -> bool {
    let Ok(url) = tauri::Url::parse(link) else {
        return false;
    };
    match url.scheme() {
        "https" => url.host().is_some(),
        "http" => matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "localhost")),
        _ => false,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Skill {
    Kaiki,
    AgenticMessaging,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum SkillHost {
    Claude,
    Codex,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SkillRequest {
    skill: Skill,
    host: SkillHost,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnlockRequest {
    password: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshNetworkRequest {
    switch: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetAutostartRequest {
    on: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkipReleaseRequest {
    version: String,
}
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Provider {
    Google,
    Github,
}
impl Provider {
    fn name(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Github => "github",
        }
    }
}
/// Pasted text or a contacts file read for addresses, at most this long.
const MAX_ADDRESS_TEXT: usize = 1 << 20;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextRequest {
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HandleRequest {
    kind: Provider,
    handle: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LookupRequest {
    handles: Vec<HandleRequest>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KindRequest {
    kind: Provider,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LinkRequest {
    link_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CardRequest {
    kind: String,
    #[serde(default)]
    group_id: Option<String>,
    about: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    langs: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WithdrawRequest {
    card_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchRequest {
    query: String,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    kind: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimRequest {
    provider: Provider,
}
/// A payment call: ETH at the quote, or USDC's allowance then purchase.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
enum PaymentStep {
    Eth,
    Approve,
    Buy,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaymentRequest {
    book: String,
    step: PaymentStep,
}
fn parse<T: serde::de::DeserializeOwned>(request: Value) -> std::result::Result<T, CommandError> {
    serde_json::from_value(request).map_err(|_| CommandError::invalid("Invalid command fields"))
}

fn owner_window<R: Runtime>(window: &WebviewWindow<R>) -> std::result::Result<(), CommandError> {
    if window.label() != "main" {
        return Err(CommandError::new(
            "not_allowed",
            "Desktop command requires the main window",
        ));
    }
    Ok(())
}

/// Commands that forward one daemon method with the webview's request.
macro_rules! forward {
    ($($name:ident),* $(,)?) => {$(
        #[tauri::command]
        async fn $name<R: Runtime>(
            window: WebviewWindow<R>,
            bridge: State<'_, NativeBridge>,
            request: Value,
        ) -> Answer {
            owner_window(&window)?;
            bridge.request(stringify!($name), request).await
        }
    )*};
}
/// Commands that forward one daemon method with no request.
macro_rules! forward_empty {
    ($($name:ident),* $(,)?) => {$(
        #[tauri::command]
        async fn $name<R: Runtime>(
            window: WebviewWindow<R>,
            bridge: State<'_, NativeBridge>,
        ) -> Answer {
            owner_window(&window)?;
            bridge.request(stringify!($name), json!({})).await
        }
    )*};
}
forward!(
    desktop_overview,
    conversation_history,
    configure_network,
    create_identity,
    create_invitation,
    add_contact,
    send_message,
    provision_runtime,
    revoke_runtime,
    accept_intro_request,
    reject_intro_request,
    set_intro_policy,
    group,
    follow_group,
    unfollow_group,
    door_requests,
    door_decide,
    channel_storage,
);
forward_empty!(
    snapshot,
    network_settings,
    list_runtimes,
    intro_requests,
    intro_policy,
    groups,
    coins_balance,
    follows,
);

/// The discovery screen's commands (spec/discovery-v1.md), over the
/// daemon's service.
#[tauri::command]
async fn discover_handles<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    bridge.discover_handles(parse(request)?)
}
#[tauri::command]
async fn discover_lookup<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    bridge.discover_lookup(parse(request)?).await
}
#[tauri::command]
async fn discover_link<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: KindRequest = parse(request)?;
    bridge.discover_link(request.kind).await
}
#[tauri::command]
async fn discover_status<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: LinkRequest = parse(request)?;
    discover::status(&*bridge, &request.link_id).await
}
#[tauri::command]
async fn discover_unlink<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: KindRequest = parse(request)?;
    discover::unlink(&*bridge, request.kind.name()).await
}
#[tauri::command]
async fn discover_publish<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: CardRequest = parse(request)?;
    discover::publish(
        &*bridge,
        json!({"kind": request.kind, "groupId": request.group_id, "about": request.about, "tags": request.tags, "langs": request.langs}),
    )
    .await
}
#[tauri::command]
async fn discover_withdraw<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: WithdrawRequest = parse(request)?;
    discover::withdraw(&*bridge, &request.card_id).await
}
#[tauri::command]
async fn discover_search<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: SearchRequest = parse(request)?;
    discover::search(
        &*bridge,
        &request.query,
        request.tag.as_deref(),
        request.lang.as_deref(),
        request.kind.as_deref(),
    )
    .await
}

#[tauri::command]
async fn request_contact<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    bridge.when_cards_are_read("request_contact", request).await
}
#[tauri::command]
async fn create_group<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    bridge.when_cards_are_read("create_group", request).await
}
#[tauri::command]
async fn change_group<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    bridge.when_cards_are_read("change_group", request).await
}
#[tauri::command]
async fn channel_subscribe<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    bridge
        .when_cards_are_read("channel_subscribe", request)
        .await
}
/// Asking at a group's door waits while its card is looked for, like a
/// member's card.
#[tauri::command]
async fn join_group<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    bridge.when_cards_are_read("join_group", request).await
}
#[tauri::command]
async fn coins_buy<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.buy().await
}
#[tauri::command]
async fn open_payment<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: PaymentRequest = parse(request)?;
    bridge.open_payment(&request.book, request.step)
}
#[tauri::command]
async fn claim_coins<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: ClaimRequest = parse(request)?;
    bridge.claim(request.provider).await
}
#[tauri::command]
async fn install_skill<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: SkillRequest = parse(request)?;
    bridge.install_skill(request.skill, request.host)
}
#[tauri::command]
async fn owner_cli<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.owner_cli()
}
#[tauri::command]
async fn profile_status<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    Ok(bridge.status().await)
}
#[tauri::command]
async fn unlock_profile<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: UnlockRequest = parse(request)?;
    bridge.unlock(Zeroizing::new(request.password)).await
}
#[tauri::command]
async fn reconnect<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.reconnect().await
}
#[tauri::command]
async fn network_preset<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    Ok(bridge.network())
}
#[tauri::command]
async fn release_status<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.release(Some(RELEASE_CHECK_EVERY)).await
}
#[tauri::command]
async fn check_release<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.release(None).await
}
#[tauri::command]
async fn skip_release<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: SkipReleaseRequest = parse(request)?;
    bridge.skip_release(&request.version)
}
/// Replaces the app, then starts the new one.
#[tauri::command]
async fn install_update<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    let answer = bridge.install_update().await?;
    window.app_handle().request_restart();
    Ok(answer)
}
#[tauri::command]
async fn open_downloads<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.open_downloads()
}
#[tauri::command]
async fn autostart_status<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.autostart_status()
}
#[tauri::command]
async fn set_autostart<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: SetAutostartRequest = parse(request)?;
    bridge.set_autostart(request.on)
}
#[tauri::command]
async fn open_login_items<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
) -> Answer {
    owner_window(&window)?;
    bridge.open_login_items()
}
#[tauri::command]
async fn refresh_network<R: Runtime>(
    window: WebviewWindow<R>,
    bridge: State<'_, NativeBridge>,
    request: Value,
) -> Answer {
    owner_window(&window)?;
    let request: RefreshNetworkRequest = parse(request)?;
    bridge.refresh_network(request.switch).await
}

pub fn configure<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        profile_status,
        unlock_profile,
        reconnect,
        network_preset,
        refresh_network,
        release_status,
        check_release,
        skip_release,
        install_update,
        open_downloads,
        autostart_status,
        set_autostart,
        open_login_items,
        snapshot,
        desktop_overview,
        conversation_history,
        network_settings,
        configure_network,
        create_identity,
        create_invitation,
        add_contact,
        send_message,
        request_contact,
        intro_requests,
        accept_intro_request,
        reject_intro_request,
        intro_policy,
        set_intro_policy,
        groups,
        group,
        create_group,
        change_group,
        door_requests,
        door_decide,
        join_group,
        channel_storage,
        channel_subscribe,
        list_runtimes,
        provision_runtime,
        revoke_runtime,
        install_skill,
        owner_cli,
        coins_balance,
        coins_buy,
        claim_coins,
        open_payment,
        follows,
        follow_group,
        unfollow_group,
        discover_handles,
        discover_lookup,
        discover_link,
        discover_status,
        discover_unlink,
        discover_publish,
        discover_withdraw,
        discover_search
    ])
}

/// The daemon, for the discovery flows.
impl discover::Node for NativeBridge {
    type Error = CommandError;
    async fn call(&self, method: &str, request: Value) -> Answer {
        self.request(method, request).await
    }
}

/// The CLI's arguments for the profile in `data_dir`: none when the CLI,
/// started without `AGENTIC_DATA_DIR`, opens that profile by itself.
fn profile_args(data_dir: &Path, default: Option<&Path>) -> Vec<String> {
    if default == Some(data_dir) {
        vec![]
    } else {
        vec!["--data-dir".into(), data_dir.to_string_lossy().into_owned()]
    }
}

/// Invalidate the main window only when the daemon's public view changes.
/// An error is a state transition too, so an idle UI notices daemon failure.
pub fn start_updates<R: Runtime>(app: tauri::AppHandle<R>) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut previous = None;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let current = app
                .state::<NativeBridge>()
                .request("desktop_revision", json!({}))
                .await;
            if previous.as_ref() != Some(&current) {
                let _ = app.emit_to("main", "core:changed", ());
                previous = Some(current);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The profile's directory goes into the agent's command only when the
    /// CLI, started without AGENTIC_DATA_DIR, would not find it by itself.
    #[test]
    fn the_agents_command_names_only_a_profile_the_cli_would_not_find() {
        let default = Path::new("/home/owner/.local/share/net.agenticinternet.desktop");
        assert_eq!(profile_args(default, Some(default)), Vec::<String>::new());
        let other = Path::new("/tmp/another profile");
        assert_eq!(
            profile_args(other, Some(default)),
            ["--data-dir", "/tmp/another profile"]
        );
    }
}
