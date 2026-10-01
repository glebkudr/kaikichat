//! Native bootstrap and owner client for the desktop app: the shared host of
//! `agentic_node::host`, and the isolated E2E secret store.
#[cfg(all(feature = "e2e", not(debug_assertions)))]
compile_error!("The E2E file secret store must never be included in a release build");
#[cfg(all(feature = "e2e", unix))]
mod e2e_secrets;
pub use agentic_node::host::{
    DaemonFlags, DesktopHost, HostConfig, KeychainStore, Result, SERVICE, SKILL_ROOTS, SecretStore,
    SecretsBackend, asks_before_opening, connect_profile, default_data_dir, owner_skill,
    password_from_env, platform_data_dir, restart_profile, stop_profile,
};
pub use agentic_node::secrets_file::{PasswordFileStore, SecretsLocked};
pub use agentic_node::{NETWORK_DOMAIN, discover};
pub use agentic_node::{autostart, network_preset, relocate, update};
#[cfg(all(feature = "e2e", unix))]
pub use e2e_secrets::E2eFileStore;
