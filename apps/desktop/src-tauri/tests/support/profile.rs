//! One profile, one daemon for the window and the owner CLI
//! (spec/desktop-gui-v1.md): whichever starts first starts it with the
//! profile's saved flags, the other joins it; a password-sealed profile is
//! opened with the password given in the window.
use super::*;
use agentic_desktop::{ProfileConfig, Secrets};
use agentic_desktop_host::autostart::{Approval, Autostart, Launch, Manager, Place};
use agentic_desktop_host::network_preset::{PresetSource, public_key, sign};
use agentic_desktop_host::stop_profile;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

const PASSWORD: &str = "correct horse battery staple";

fn binaries() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// A private profile directory and a home of its own.
struct Profile {
    dir: TempDir,
}

impl Profile {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("ain-gui-")
            .tempdir_in("/tmp")
            .unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        Self { dir }
    }
    fn data(&self) -> PathBuf {
        self.dir.path().join("data")
    }
    fn home(&self) -> PathBuf {
        let home = self.dir.path().join("home");
        std::fs::create_dir_all(&home).unwrap();
        home
    }

    /// `kaiki` on this profile with the file secrets and `password`.
    fn cli_with(&self, password: &str, args: &[&str]) -> (i32, Value) {
        let output = std::process::Command::new(binaries().join("kaiki"))
            .args(args)
            .env("HOME", self.home())
            .env("AGENTIC_DATA_DIR", self.data())
            .env("AGENTIC_SECRETS", "file")
            .env("AGENTIC_PASSWORD", password)
            .env_remove("AGENTIC_PASSWORD_FILE")
            .env("AGENTIC_NETWORK_PRESET", "off")
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let value = serde_json::from_str(text.trim())
            .unwrap_or_else(|_| panic!("not one JSON envelope: {text:?}"));
        (output.status.code().unwrap_or(-1), value)
    }
    fn cli(&self, args: &[&str]) -> Value {
        let (code, value) = self.cli_with(PASSWORD, args);
        assert_eq!(code, 0, "{args:?}: {value}");
        value["result"].clone()
    }

    /// The window's view of this profile: its secrets in the file, the
    /// password given now or later in the window.
    async fn window(&self, password: Option<&str>) -> Window {
        self.window_with(Secrets::File {
            password: password.map(|p| Zeroizing::new(p.to_owned())),
        })
        .await
    }

    /// The window as the app runs it, updates included.
    async fn window_with(&self, secrets: Secrets) -> Window {
        self.window_following(secrets, None).await
    }

    /// The window with the network preset `preset`, or none.
    async fn window_following(&self, secrets: Secrets, preset: Option<PresetSource>) -> Window {
        self.window_as(secrets, preset, None).await
    }

    /// The window, as if its binary were `app` when one is named.
    async fn window_as(
        &self,
        secrets: Secrets,
        preset: Option<PresetSource>,
        app: Option<PathBuf>,
    ) -> Window {
        self.window_opening(secrets, preset, app, None).await
    }

    /// The installed app at `app`, which opens at login from this
    /// profile's home, in `items`.
    async fn window_at_login(&self, app: &str, items: Arc<LoginItems>) -> Window {
        let autostart = Autostart::new(
            Launch::Window { app: app.into() },
            Place {
                manager: Manager::here(),
                home: self.home(),
                config: self.home().join(".config"),
            },
            &self.data(),
            items,
        );
        self.window_opening(
            Secrets::File {
                password: Some(Zeroizing::new(PASSWORD.to_owned())),
            },
            None,
            Some(app.into()),
            Some(autostart),
        )
        .await
    }

    async fn window_opening(
        &self,
        secrets: Secrets,
        preset: Option<PresetSource>,
        app: Option<PathBuf>,
        autostart: Option<Autostart>,
    ) -> Window {
        let opened = Arc::new(Opened::default());
        let mut bridge = NativeBridge::open(ProfileConfig {
            data_dir: self.data(),
            node_binary: binaries().join("agentic-node"),
            listen: vec!["/ip4/127.0.0.1/tcp/0".into()],
            secrets,
            home: self.home(),
            opener: opened.clone(),
            preset,
            autostart,
        })
        .await;
        if let Some(app) = app {
            bridge = bridge.with_app(app);
        }
        let app = configure(mock_builder())
            .manage(bridge)
            .build(tauri::generate_context!())
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(
            &app,
            "main",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .build()
        .unwrap();
        let updates = start_updates(app.handle().clone());
        Window {
            window,
            updates,
            opened,
            _app: app,
        }
    }

    /// The profile's one daemon once its command line has `flag`: its pid.
    fn daemon_with(&self, flag: &str) -> u32 {
        let data = std::fs::canonicalize(self.data()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let listing = std::process::Command::new("ps")
                .args(["-wwAo", "pid=,command="])
                .output()
                .unwrap()
                .stdout;
            let daemons: Vec<(u32, String)> = String::from_utf8(listing)
                .unwrap()
                .lines()
                .filter(|line| {
                    line.contains("agentic-node serve")
                        && line.contains(&format!("{}/profile.db", data.display()))
                })
                .filter_map(|line| {
                    let (pid, command) = line.trim().split_once(' ')?;
                    Some((pid.parse().ok()?, command.to_owned()))
                })
                .collect();
            if let [(pid, command)] = daemons.as_slice()
                && command.contains(flag)
            {
                return *pid;
            }
            assert!(
                Instant::now() < deadline,
                "one daemon with {flag}: {daemons:?}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// The daemons running for this profile.
    fn daemons(&self) -> usize {
        let data = std::fs::canonicalize(self.data()).unwrap_or_else(|_| self.data());
        let listing = std::process::Command::new("ps")
            .args(["-wwAo", "command"])
            .output()
            .unwrap()
            .stdout;
        String::from_utf8(listing)
            .unwrap()
            .lines()
            .filter(|line| {
                line.contains("agentic-node serve")
                    && line.contains(&format!("{}/profile.db", data.display()))
            })
            .count()
    }
}

impl Drop for Profile {
    fn drop(&mut self) {
        let _ = self.cli_with(PASSWORD, &["daemon", "stop"]);
        // A daemon the CLI cannot open (a keychain profile, a test that
        // failed half-way) is stopped by its profile path.
        let data = std::fs::canonicalize(self.data()).unwrap_or_else(|_| self.data());
        let _ = std::process::Command::new("pkill")
            .args([
                "-f",
                &format!("agentic-node serve --profile {}/profile.db", data.display()),
            ])
            .status();
    }
}

struct Window {
    window: tauri::WebviewWindow<MockRuntime>,
    updates: tauri::async_runtime::JoinHandle<()>,
    /// Links the window asked the system to open.
    opened: Arc<Opened>,
    _app: tauri::App<MockRuntime>,
}

impl Drop for Window {
    fn drop(&mut self) {
        self.updates.abort();
    }
}

impl Window {
    fn call(&self, method: &str, body: Value) -> std::result::Result<Value, Value> {
        invoke(&self.window, "tauri://localhost", method, body)
    }
    fn ok(&self, method: &str) -> Value {
        self.call(method, json!({}))
            .unwrap_or_else(|error| panic!("{method}: {error}"))
    }
    fn refused(&self, method: &str, body: Value) -> Value {
        let error = self
            .call(method, body)
            .expect_err(&format!("{method} must be refused"));
        assert!(error["code"].is_string(), "{method}: {error}");
        assert!(error["message"].is_string(), "{method}: {error}");
        assert!(error["retryable"].is_boolean(), "{method}: {error}");
        error
    }
    fn unlock(&self, password: &str) -> std::result::Result<Value, Value> {
        self.call("unlock_profile", json!({"request":{"password":password}}))
    }
}

/// A closed port: a node told to use it keeps asking (claim_pending), while
/// one without the flag answers identity_not_configured.
fn closed_identity_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}", listener.local_addr().unwrap())
}

fn wait_until(what: &str, check: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !check() {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn claim_is_pending(window: &Window) {
    let refused = window.refused("claim_coins", json!({"request":{"provider":"google"}}));
    assert_eq!(
        (refused["code"].as_str(), refused["retryable"].as_bool()),
        (Some("claim_pending"), Some(true)),
        "the daemon runs with the profile's saved identity server: {refused}"
    );
}

fn private_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o077 == 0)
        .unwrap_or(false)
}

#[tokio::test]
async fn the_window_joins_the_daemon_the_cli_started_once_its_password_is_given() {
    let p = Profile::new();
    let server = closed_identity_server();
    p.cli(&[
        "daemon",
        "start",
        "--listen",
        "/ip4/127.0.0.1/tcp/0",
        "--identity-server",
        &server,
    ]);
    let identity = p.cli(&["init", "--name", "Alice"]);
    let w = p.window(None).await;
    let status = w.ok("profile_status");
    assert_eq!(
        (
            status["state"].as_str(),
            status["secrets"].as_str(),
            status["newProfile"].as_bool()
        ),
        (Some("locked"), Some("file"), Some(false)),
        "{status}"
    );
    let refused = w.refused("snapshot", json!({}));
    assert_eq!(refused["code"], "profile_locked", "{refused}");
    let refused = w.unlock("not the password").expect_err("a wrong password");
    assert_eq!(refused["code"], "secrets_locked", "{refused}");
    assert_eq!(w.ok("profile_status")["state"], "locked");
    w.unlock(PASSWORD).unwrap();
    let status = w.ok("profile_status");
    assert_eq!(status["state"], "connected", "{status}");
    assert!(!status.to_string().contains(PASSWORD));
    // The same identity through the same daemon.
    assert_eq!(
        w.ok("snapshot")["identity"]["networkId"],
        identity["networkId"]
    );
    assert_eq!(p.daemons(), 1);
    claim_is_pending(&w);
}

#[tokio::test]
async fn a_daemon_stopped_from_the_cli_waits_for_the_window_to_start_it_with_the_saved_flags() {
    let p = Profile::new();
    let server = closed_identity_server();
    p.cli(&[
        "daemon",
        "start",
        "--listen",
        "/ip4/127.0.0.1/tcp/0",
        "--identity-server",
        &server,
    ]);
    let identity = p.cli(&["init", "--name", "Alice"]);
    let w = p.window(Some(PASSWORD)).await;
    assert_eq!(w.ok("profile_status")["state"], "connected");
    p.cli(&["daemon", "stop"]);
    let refused = w.refused("snapshot", json!({}));
    assert_eq!(
        (refused["code"].as_str(), refused["retryable"].as_bool()),
        (Some("daemon_unavailable"), Some(true)),
        "{refused}"
    );
    // The window does not start a daemon its owner stopped on its own.
    std::thread::sleep(Duration::from_millis(800));
    assert_eq!(p.cli(&["daemon", "status"])["running"], false);
    assert_eq!(p.daemons(), 0);
    assert_eq!(w.ok("profile_status")["state"], "unavailable");
    w.ok("reconnect");
    assert_eq!(w.ok("profile_status")["state"], "connected");
    // The CLI now uses the daemon the window started.
    let status = p.cli(&["daemon", "status"]);
    assert_eq!(status["running"], true);
    assert_eq!(status["networkId"], identity["networkId"]);
    wait_until("one daemon for the profile", || p.daemons() == 1);
    claim_is_pending(&w);
    // After a restart of the machine: the app opens on a stopped profile and
    // starts its daemon with the saved flags.
    drop(w);
    p.cli(&["daemon", "stop"]);
    let reopened = p.window(Some(PASSWORD)).await;
    assert_eq!(reopened.ok("profile_status")["state"], "connected");
    assert_eq!(reopened.ok("snapshot")["identity"], identity);
    claim_is_pending(&reopened);
    wait_until("one daemon for the profile", || p.daemons() == 1);
}

#[tokio::test]
async fn a_new_profile_opened_in_the_window_is_sealed_with_the_password_given_there() {
    let p = Profile::new();
    let w = p.window(None).await;
    let status = w.ok("profile_status");
    assert_eq!(
        (status["state"].as_str(), status["newProfile"].as_bool()),
        (Some("locked"), Some(true)),
        "{status}"
    );
    w.unlock(PASSWORD).unwrap();
    let identity = w
        .call("create_identity", json!({"request":{"name":"Bob"}}))
        .unwrap();
    let secrets = p.data().join("secrets.json");
    assert!(private_file(&secrets), "the sealed secret is private");
    // The CLI opens the same profile with the same password.
    let status = p.cli(&["daemon", "status"]);
    assert_eq!(status["networkId"], identity["networkId"]);
    // A window given the password up front (AGENTIC_PASSWORD) opens at once.
    drop(w);
    let again = p.window(Some(PASSWORD)).await;
    assert_eq!(again.ok("profile_status")["state"], "connected");
    assert_eq!(again.ok("snapshot")["identity"], identity);
    assert_eq!(p.daemons(), 1);
}

/// The default on macOS: the secret is in the keychain (here a stand-in), so
/// the window opens the profile at once and never asks for a password.
#[tokio::test]
async fn a_keychain_profile_opens_without_asking_for_a_password() {
    let p = Profile::new();
    let w = p
        .window_with(Secrets::Store(Arc::new(Vault::default())))
        .await;
    let status = w.ok("profile_status");
    assert_eq!(
        (status["state"].as_str(), status["secrets"].as_str()),
        (Some("connected"), Some("keychain")),
        "{status}"
    );
    let identity = w
        .call("create_identity", json!({"request":{"name":"Carol"}}))
        .unwrap();
    assert_eq!(w.ok("snapshot")["identity"], identity);
    // A password never seals a second secret for a keychain profile.
    let refused = w.refused("unlock_profile", json!({"request":{"password":PASSWORD}}));
    assert_eq!(refused["code"], "invalid_request", "{refused}");
    assert!(!p.data().join("secrets.json").exists());
}

/// The owner's answer to macOS's keychain dialog.
#[derive(Clone, Copy, PartialEq)]
enum DialogAnswer {
    Deny,
    /// Gives the key this once: the next read asks again.
    Allow,
    /// Trusts this app with the item from now on.
    AlwaysAllow,
}

/// The macOS keychain, as far as the window sees it: a program reads the
/// items it saved itself at once; an item another program saved (the CLI,
/// an older build) makes macOS show its dialog first, answered with
/// `answer`.
struct Keychain {
    /// Each account's secret and whether this app may read it unasked.
    items: Mutex<HashMap<String, (Vec<u8>, bool)>>,
    answer: Mutex<DialogAnswer>,
    dialogs: AtomicUsize,
}

impl Keychain {
    fn new() -> Self {
        Self {
            items: Mutex::default(),
            answer: Mutex::new(DialogAnswer::Deny),
            dialogs: AtomicUsize::new(0),
        }
    }
    /// Every item as if another program had saved it.
    fn saved_elsewhere(&self) {
        for item in self.items.lock().unwrap().values_mut() {
            item.1 = false;
        }
    }
    fn answer(&self, answer: DialogAnswer) {
        *self.answer.lock().unwrap() = answer;
    }
    fn dialogs(&self) -> usize {
        self.dialogs.load(SeqCst)
    }
}

impl SecretStore for Keychain {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let mut items = self.items.lock().unwrap();
        let Some((secret, trusted)) = items.get_mut(account) else {
            return Ok(None);
        };
        if !*trusted {
            self.dialogs.fetch_add(1, SeqCst);
            match *self.answer.lock().unwrap() {
                DialogAnswer::Deny => {
                    return Err("the owner denied access to the keychain item".into());
                }
                DialogAnswer::Allow => {}
                DialogAnswer::AlwaysAllow => *trusted = true,
            }
        }
        Ok(Some(Zeroizing::new(secret.clone())))
    }
    fn set(&self, account: &str, secret: &[u8]) -> Result<()> {
        self.items
            .lock()
            .unwrap()
            .insert(account.into(), (secret.to_vec(), true));
        Ok(())
    }
    fn asks_first(&self, account: &str) -> Result<bool> {
        let items = self.items.lock().unwrap();
        Ok(items.get(account).is_some_and(|(_, trusted)| !trusted))
    }
}

/// A key another program saved (the CLI made the profile, or an older
/// build): macOS asks before this app reads it. The window explains that
/// first and touches the key only when the owner goes on, each time with
/// one dialog: a denial keeps the explanation, "Allow" opens the profile
/// for this launch, "Always Allow" for good.
#[tokio::test]
async fn a_key_another_program_saved_is_read_only_after_the_window_explains_the_dialog() {
    let p = Profile::new();
    let keychain = Arc::new(Keychain::new());
    let w = p.window_with(Secrets::Store(keychain.clone())).await;
    assert_eq!(w.ok("profile_status")["state"], "connected");
    let identity = w
        .call("create_identity", json!({"request":{"name":"Dana"}}))
        .unwrap();
    drop(w);
    stop_profile(&p.data(), keychain.as_ref()).await.unwrap();
    wait_until("the daemon stopped", || p.daemons() == 0);
    assert_eq!(keychain.dialogs(), 0, "the app's own new key never asks");
    keychain.saved_elsewhere();

    let w = p.window_with(Secrets::Store(keychain.clone())).await;
    let status = w.ok("profile_status");
    assert_eq!(
        (status["state"].as_str(), status["secrets"].as_str()),
        (Some("keychain"), Some("keychain")),
        "{status}"
    );
    for method in ["snapshot", "reconnect"] {
        let refused = w.refused(method, json!({}));
        assert_eq!(refused["code"], "keychain_consent", "{method}: {refused}");
    }
    assert_eq!(
        keychain.dialogs(),
        0,
        "macOS asked before the owner read why"
    );
    assert_eq!(p.daemons(), 0, "nothing starts before the key is read");

    let refused = w.refused("open_keychain", json!({}));
    assert_eq!(refused["code"], "keychain_denied", "{refused}");
    assert_eq!(keychain.dialogs(), 1);
    assert_eq!(w.ok("profile_status")["state"], "keychain");
    assert_eq!(p.daemons(), 0);

    keychain.answer(DialogAnswer::Allow);
    let opened = w.call("open_keychain", json!({})).unwrap();
    assert_eq!(opened["state"], "connected", "{opened}");
    assert_eq!(
        keychain.dialogs(),
        2,
        "one dialog per time the owner goes on"
    );
    assert_eq!(w.ok("snapshot")["identity"], identity);
    drop(w);

    // Allowed once only: the next launch explains again, joins the daemon
    // after "Always Allow", and the launch after it opens at once.
    keychain.answer(DialogAnswer::AlwaysAllow);
    let w = p.window_with(Secrets::Store(keychain.clone())).await;
    assert_eq!(w.ok("profile_status")["state"], "keychain");
    assert_eq!(keychain.dialogs(), 2);
    let opened = w.call("open_keychain", json!({})).unwrap();
    assert_eq!(opened["state"], "connected", "{opened}");
    assert_eq!(keychain.dialogs(), 3);
    drop(w);
    let w = p.window_with(Secrets::Store(keychain.clone())).await;
    assert_eq!(w.ok("profile_status")["state"], "connected");
    assert_eq!(w.ok("snapshot")["identity"], identity);
    assert_eq!(
        keychain.dialogs(),
        3,
        "an always allowed app is not asked again"
    );
    assert_eq!(p.daemons(), 1);
}

/// The window tells an agent how to reach this profile: the owner CLI beside
/// the app, with this profile's directory since it is not the default one.
/// An agent that runs exactly that drives the window's own identity, and the
/// skill the window installs names the same CLI.
#[tokio::test]
async fn an_agent_following_the_windows_instructions_drives_this_profile() {
    let p = Profile::new();
    let w = p.window(Some(PASSWORD)).await;
    let identity = w
        .call("create_identity", json!({"request":{"name":"Alice"}}))
        .unwrap();
    let cli = w.ok("owner_cli");
    let command = cli["command"].as_str().unwrap().to_owned();
    assert_eq!(Path::new(&command), binaries().join("kaiki"), "{cli}");
    let args: Vec<String> = serde_json::from_value(cli["args"].clone()).unwrap();
    assert_eq!(args.len(), 2, "{cli}");
    assert_eq!(args[0], "--data-dir");
    assert_eq!(
        std::fs::canonicalize(&args[1]).unwrap(),
        std::fs::canonicalize(p.data()).unwrap()
    );
    // The agent's shell has no AGENTIC_DATA_DIR; the secrets file and its
    // password stand in for the keychain, as in every test here.
    let output = std::process::Command::new(&command)
        .args(&args)
        .args(["daemon", "status"])
        .env("AGENTIC_NETWORK_PRESET", "off")
        .env("HOME", p.home())
        .env("AGENTIC_SECRETS", "file")
        .env("AGENTIC_PASSWORD", PASSWORD)
        .env_remove("AGENTIC_DATA_DIR")
        .env_remove("AGENTIC_PASSWORD_FILE")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .unwrap();
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        status["result"]["networkId"], identity["networkId"],
        "{status}"
    );
    let installed = w
        .call(
            "install_skill",
            json!({"request":{"skill":"kaiki","host":"claude"}}),
        )
        .unwrap();
    let text = std::fs::read_to_string(installed["path"].as_str().unwrap()).unwrap();
    assert!(
        text.contains(&format!("`'{command}' --data-dir '{}'`", args[1])),
        "{text}"
    );
}

/// A skill an older app installed may name a CLI that is gone: the app was
/// moved, renamed or first run from a disk image. Opening the window points
/// every installed `kaiki` skill at the CLI it ships, before the profile is
/// even unlocked, and installs none the owner did not install.
#[tokio::test]
async fn opening_the_window_points_installed_skills_at_its_own_cli() {
    let p = Profile::new();
    let installed = p.home().join(".codex/skills/kaiki/SKILL.md");
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(
        &installed,
        "---\nname: kaiki\n---\n# Kaiki Chat\nOn this computer the CLI is `'/Volumes/Kaiki Chat/Kaiki Chat.app/Contents/MacOS/kaiki'`.\n",
    )
    .unwrap();
    let w = p.window(None).await;
    assert_eq!(w.ok("profile_status")["state"], "locked");
    let cli = w.ok("owner_cli");
    let text = std::fs::read_to_string(&installed).unwrap();
    assert!(!text.contains("/Volumes/Kaiki Chat/"), "{text}");
    assert!(
        text.contains(&format!(
            "`'{}' --data-dir '{}'`",
            cli["command"].as_str().unwrap(),
            cli["args"][1].as_str().unwrap()
        )),
        "{text}"
    );
    assert!(
        text.contains("kaiki contacts list"),
        "the whole skill: {text}"
    );
    assert!(!p.home().join(".claude/skills/kaiki").exists());
}

const PRESET_SEED: [u8; 32] = [3; 32];

/// A network preset signed with `PRESET_SEED`: one route to a peer that is
/// not there and the identity server `identity`.
fn network_preset(network: &str, serial: u64, identity: &str) -> Value {
    let preset = json!({
        "network": network,
        "name": format!("Network {network}"),
        "serial": serial,
        "bootstrap": ["/ip4/127.0.0.1/tcp/9/p2p/12D3KooW9tHTtS3inCZiYykw4u5G4frbjVFqhkmJX12gSNCVeH3e"],
        "identityServer": identity,
    });
    serde_json::from_str(&sign(&preset.to_string(), &PRESET_SEED).unwrap()).unwrap()
}

/// A new profile opened in the window follows the signed preset: the window
/// shows its network, and another network is only offered until the owner
/// takes it.
#[tokio::test]
async fn the_window_follows_the_signed_preset_and_changes_network_when_asked() {
    let p = Profile::new();
    let (first, second) = (closed_identity_server(), closed_identity_server());
    let served = Arc::new(Mutex::new(network_preset("net-a", 1, &first)));
    let answer = served.clone();
    let url = format!(
        "{}/network.json",
        crate::owner::stub(move |_, _| answer.lock().unwrap().clone())
    );
    let source = PresetSource::new(&url, &public_key(&PRESET_SEED)).unwrap();
    let w = p
        .window_following(
            Secrets::File {
                password: Some(Zeroizing::new(PASSWORD.to_owned())),
            },
            Some(source),
        )
        .await;
    assert_eq!(w.ok("profile_status")["state"], "connected");
    let network = w.ok("network_preset");
    assert_eq!(
        (
            &network["source"],
            &network["state"],
            &network["network"],
            &network["name"]
        ),
        (
            &json!("preset"),
            &json!("current"),
            &json!("net-a"),
            &json!("Network net-a")
        ),
        "{network}"
    );
    let started = p.daemon_with(&format!("--identity-server {first}"));
    let refused = w.refused("refresh_network", json!({"request":{"switch":true}}));
    assert_eq!(
        (refused["code"].as_str(), refused["retryable"].as_bool()),
        (Some("no_network_offer"), Some(false)),
        "{refused}"
    );

    *served.lock().unwrap() = network_preset("net-b", 2, &second);
    let network = w
        .call("refresh_network", json!({"request":{"switch":false}}))
        .unwrap();
    assert_eq!(
        (
            &network["state"],
            &network["network"],
            &network["offered"]["network"]
        ),
        (&json!("switch"), &json!("net-a"), &json!("net-b")),
        "{network}"
    );
    assert_ne!(
        p.daemon_with(&format!("--identity-server {first}")),
        started,
        "refresh starts the daemon again"
    );
    w.call("create_identity", json!({"request":{"name":"Alice"}}))
        .unwrap();
    let identity = w.ok("snapshot")["identity"].clone();

    let network = w
        .call("refresh_network", json!({"request":{"switch":true}}))
        .unwrap();
    assert_eq!(
        (&network["state"], &network["network"], &network["offered"]),
        (&json!("current"), &json!("net-b"), &Value::Null),
        "{network}"
    );
    p.daemon_with(&format!("--identity-server {second}"));
    assert_eq!(w.ok("snapshot")["identity"], identity);
    assert_eq!(p.daemons(), 1);
}

/// The network preset `network_preset` gives, naming the release `version`
/// with app builds for both platforms.
fn preset_with_release(serial: u64, identity: &str, version: &str) -> Value {
    let mut preset: Value = serde_json::from_str(
        network_preset("net-a", serial, identity)["preset"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let build = json!({"url": "https://kaikichat.com/downloads/kaiki-chat.tar.gz", "sha256": "ab".repeat(32)});
    preset["release"] = json!({
        "version": version,
        "builds": {"app-macos-arm64": build, "app-linux-x86_64": build},
    });
    serde_json::from_str(&sign(&preset.to_string(), &PRESET_SEED).unwrap()).unwrap()
}

/// The window names a newer release from the preset and says whether it
/// can replace itself: only as a binary in an app bundle. Elsewhere it
/// refuses the update and opens the downloads page; a skipped release stays
/// known but is marked.
#[tokio::test]
async fn the_window_names_a_newer_release_and_what_this_app_can_do_with_it() {
    let p = Profile::new();
    let identity = closed_identity_server();
    let served = Arc::new(Mutex::new(preset_with_release(1, &identity, "99.0.0")));
    let hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (answer, count) = (served.clone(), hits.clone());
    let url = format!(
        "{}/network.json",
        crate::owner::stub(move |_, _| {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            answer.lock().unwrap().clone()
        })
    );
    let source = PresetSource::new(&url, &public_key(&PRESET_SEED)).unwrap();
    let secrets = || Secrets::File {
        password: Some(Zeroizing::new(PASSWORD.to_owned())),
    };
    let w = p.window_following(secrets(), Some(source.clone())).await;
    let release = w.ok("release_status");
    assert_eq!(
        (
            &release["current"],
            &release["latest"],
            &release["available"],
            &release["skipped"],
            &release["installable"]
        ),
        (
            &json!(env!("CARGO_PKG_VERSION")),
            &json!("99.0.0"),
            &json!(true),
            &json!(false),
            &json!(false)
        ),
        "{release}"
    );
    // Learned when the daemon started: no second question within 12 hours.
    let asked = hits.load(std::sync::atomic::Ordering::SeqCst);
    w.ok("release_status");
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), asked);

    let refused = w.refused("install_update", json!({}));
    assert_eq!(refused["code"], "not_updatable", "{refused}");
    w.ok("open_downloads");
    assert_eq!(
        w.opened.0.lock().unwrap().clone(),
        ["https://kaikichat.com/#get"]
    );

    let skipped = w
        .call("skip_release", json!({"request": {"version": "99.0.0"}}))
        .unwrap();
    assert_eq!(
        (&skipped["latest"], &skipped["skipped"]),
        (&json!("99.0.0"), &json!(true))
    );
    // Checking now asks, and a newer release is news again.
    *served.lock().unwrap() = preset_with_release(2, &identity, "99.1.0");
    let checked = w.ok("check_release");
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), asked + 1);
    assert_eq!(
        (&checked["latest"], &checked["skipped"]),
        (&json!("99.1.0"), &json!(false))
    );
    drop(w);

    // The same app in a bundle can replace itself where there is its build.
    let bundled = p
        .window_as(
            secrets(),
            Some(source),
            Some(PathBuf::from(
                "/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop",
            )),
        )
        .await;
    assert_eq!(
        bundled.ok("release_status")["installable"],
        json!(agentic_desktop_host::update::platform().is_some())
    );
}

/// The system's login items: whether they block Kaiki Chat until the owner
/// allows it, and how often the owner was asked to.
#[derive(Default)]
struct LoginItems {
    blocked: std::sync::atomic::AtomicBool,
    asked: std::sync::atomic::AtomicUsize,
}

impl Approval for LoginItems {
    fn blocked(&self, _job: &Path) -> bool {
        self.blocked.load(std::sync::atomic::Ordering::SeqCst)
    }
    fn ask(&self) -> std::io::Result<()> {
        self.asked.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

/// The app opens at login (spec/desktop-gui-v1.md, "Start at login"): the
/// installed app puts itself into the owner's login items when it opens;
/// the setting takes it out, and opening the app again leaves it out, until
/// the owner turns it back on. When the system blocks it, the window says so
/// and opens the system's login items for the owner to allow it. A window
/// that cannot open at login (a build, E2E) has no such setting.
#[tokio::test]
async fn the_app_opens_at_login_until_the_owner_turns_it_off() {
    use std::sync::atomic::Ordering::SeqCst;
    let p = Profile::new();
    let items = Arc::new(LoginItems::default());
    let app = "/Applications/Kaiki Chat.app/Contents/MacOS/agentic-desktop";

    let w = p.window_at_login(app, items.clone()).await;
    let on = w.ok("autostart_status");
    assert_eq!(on["state"], "on", "{on}");
    let job = PathBuf::from(on["path"].as_str().unwrap());
    let home = std::fs::canonicalize(p.home()).unwrap();
    assert!(
        job.starts_with(p.home()) || job.starts_with(&home),
        "{}",
        job.display()
    );
    assert!(std::fs::read_to_string(&job).unwrap().contains(app));

    let off = w
        .call("set_autostart", json!({"request": {"on": false}}))
        .unwrap();
    assert_eq!(off["state"], "off", "{off}");
    assert!(!job.exists());
    drop(w);
    let w = p.window_at_login(app, items.clone()).await;
    assert_eq!(w.ok("autostart_status")["state"], "off");
    assert!(!job.exists(), "opening the app again leaves it off");

    let on = w
        .call("set_autostart", json!({"request": {"on": true}}))
        .unwrap();
    assert_eq!(on["state"], "on", "{on}");
    assert!(job.is_file());
    items.blocked.store(true, SeqCst);
    let asked = items.asked.load(SeqCst);
    assert_eq!(w.ok("autostart_status")["state"], "blocked");
    assert_eq!(items.asked.load(SeqCst), asked, "a look asks nobody");
    w.ok("open_login_items");
    assert_eq!(items.asked.load(SeqCst), asked + 1);
    drop(w);

    let build = p.window(Some(PASSWORD)).await;
    assert_eq!(build.ok("autostart_status"), Value::Null);
    let refused = build.refused("set_autostart", json!({"request": {"on": true}}));
    assert_eq!(refused["code"], "autostart_unavailable", "{refused}");
}
