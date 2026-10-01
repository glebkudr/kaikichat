#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(all(feature = "e2e", not(debug_assertions)))]
compile_error!("The automation driver must never be included in a release build");

#[cfg(feature = "e2e")]
use agentic_desktop::Opener;
use agentic_desktop::{NativeBridge, ProfileConfig, Secrets, configure, start_updates};
#[cfg(feature = "e2e")]
use agentic_desktop_host::E2eFileStore;
use agentic_desktop_host::network_preset::PresetSource;
#[cfg(not(feature = "e2e"))]
use agentic_desktop_host::{
    KeychainStore, SERVICE, SecretsBackend,
    autostart::{Autostart, Place, SystemApproval, window_launch},
    default_data_dir, password_from_env,
};
#[cfg(feature = "e2e")]
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

/// E2E runs record the links the app would open, one per line.
#[cfg(feature = "e2e")]
struct RecordedLinks(PathBuf);
#[cfg(feature = "e2e")]
impl Opener for RecordedLinks {
    fn open(&self, link: &str) -> Result<(), String> {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.0)
            .and_then(|mut file| writeln!(file, "{link}"))
            .map_err(|error| error.to_string())
    }
}

/// The profile as the owner CLI finds it (spec/desktop-gui-v1.md).
fn profile(app: &tauri::App) -> Result<ProfileConfig, Box<dyn std::error::Error>> {
    let node_binary = std::env::current_exe()?
        .parent()
        .ok_or("desktop binary has no parent directory")?
        .join("kaiki-agentic-node");
    #[cfg(feature = "e2e")]
    {
        let _ = app;
        let data_dir = PathBuf::from(std::env::var("AIN_E2E_DATA_DIR")?);
        let store = E2eFileStore::new(&data_dir)
            .map_err(|error| -> Box<dyn std::error::Error> { error })?;
        let opener: Arc<dyn Opener> = Arc::new(RecordedLinks(data_dir.join("opened-links.txt")));
        Ok(ProfileConfig {
            listen: vec![
                "/ip4/127.0.0.1/udp/0/quic-v1".into(),
                "/ip4/127.0.0.1/tcp/0".into(),
            ],
            secrets: Secrets::Store(Arc::new(store)),
            // Skills go into the test's own home, never the developer's.
            home: std::env::var_os("AIN_E2E_HOME")
                .map_or_else(|| data_dir.join("home"), PathBuf::from),
            data_dir,
            node_binary,
            opener,
            // Never the developer's login items.
            autostart: None,
            // E2E runs set their network by flags unless they name a preset.
            preset: PresetSource::from_values(
                Some(
                    std::env::var("AIN_E2E_NETWORK_PRESET")
                        .as_deref()
                        .unwrap_or("off"),
                ),
                std::env::var("AIN_E2E_NETWORK_PRESET_KEY").ok().as_deref(),
            )
            .map_err(|error| -> Box<dyn std::error::Error> { error })?,
        })
    }
    #[cfg(not(feature = "e2e"))]
    {
        let data_dir = match default_data_dir() {
            Some(dir) => dir,
            None => app.path().app_data_dir()?,
        };
        let secrets = match SecretsBackend::chosen() {
            SecretsBackend::Keychain => Secrets::Store(Arc::new(KeychainStore::new(SERVICE))),
            SecretsBackend::File => Secrets::File {
                password: password_from_env(),
            },
        };
        let home = app.path().home_dir()?;
        // The installed app opens at login (spec/desktop-gui-v1.md).
        let autostart = window_launch(
            &std::env::current_exe()?,
            &home,
            std::env::var_os("AGENTIC_DATA_DIR").is_some(),
        )
        .zip(Place::here())
        .map(|(launch, place)| Autostart::new(launch, place, &data_dir, Arc::new(SystemApproval)));
        Ok(ProfileConfig {
            autostart,
            data_dir,
            node_binary,
            home,
            listen: vec![
                "/ip4/0.0.0.0/udp/0/quic-v1".into(),
                "/ip4/0.0.0.0/tcp/0".into(),
            ],
            secrets,
            opener: Arc::new(agentic_desktop::SystemOpener),
            preset: PresetSource::from_env()
                .map_err(|error| -> Box<dyn std::error::Error> { error })?,
        })
    }
}

fn run() -> tauri::Result<()> {
    let builder = configure(tauri::Builder::default());
    #[cfg(feature = "e2e")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    builder.setup(|app| {
        #[cfg(all(feature = "e2e", target_os = "macos"))]
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        #[cfg(feature = "e2e")]
        app.add_capability(r#"{"identifier":"test-driver","windows":["main"],"permissions":["wdio-webdriver:default"]}"#)?;
        let bridge = tauri::async_runtime::block_on(NativeBridge::open(profile(app)?));
        app.manage(bridge);
        tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
            .title("Kaiki Chat")
            .inner_size(1200.0, 800.0)
            .min_inner_size(850.0, 650.0)
            // Hidden automation windows on macOS; on Linux they run under
            // Xvfb, where WebKitGTK snapshots only what is shown.
            .visible(!cfg!(feature = "e2e") || cfg!(target_os = "linux"))
            .focused(!cfg!(feature = "e2e"))
            .on_navigation(|url| {
                (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                    || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"))
                    || (!cfg!(feature = "custom-protocol") && url.origin().ascii_serialization() == "http://127.0.0.1:1420")
            })
            .build()?;
        start_updates(app.handle().clone());
        Ok(())
    }).run(tauri::generate_context!())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Desktop startup failed: {error}");
        std::process::exit(1);
    }
}
