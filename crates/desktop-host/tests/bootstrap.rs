#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use agentic_desktop_host::{DesktopHost, HostConfig, Result, SecretStore};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    process::Child,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use tempfile::TempDir;
use zeroize::Zeroizing;
#[derive(Default)]
struct Vault {
    entries: Mutex<BTreeMap<String, Vec<u8>>>,
    writes: AtomicUsize,
    fail_get: AtomicBool,
    fail_set: AtomicBool,
    before_first_save: Mutex<Option<std::path::PathBuf>>,
}
impl SecretStore for Vault {
    fn get(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        if self.fail_get.load(Ordering::SeqCst) {
            return Err("keychain locked".into());
        }
        Ok(self
            .entries
            .lock()
            .unwrap()
            .get(account)
            .cloned()
            .map(Zeroizing::new))
    }
    fn set(&self, account: &str, secret: &[u8]) -> Result<()> {
        if self.fail_set.load(Ordering::SeqCst) {
            return Err("keychain write denied".into());
        }
        if let Some(root) = self.before_first_save.lock().unwrap().as_ref() {
            assert!(
                !root.join("profile.db").exists(),
                "profile created before durable secret save"
            );
            assert!(!root.join("node.sock").exists());
        }
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.entries
            .lock()
            .unwrap()
            .insert(account.into(), secret.to_vec());
        Ok(())
    }
}
struct Process(Option<Child>);
impl Drop for Process {
    fn drop(&mut self) {
        self.kill();
    }
}
impl Process {
    fn kill(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn config(root: &TempDir) -> HostConfig {
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    HostConfig {
        data_dir: root.path().into(),
        node_binary: std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("kaiki-agentic-node"),
        listen: vec![
            "/ip4/127.0.0.1/tcp/0".into(),
            "/ip4/127.0.0.1/udp/0/quic-v1".into(),
        ],
        serve_args: vec![],
    }
}
async fn start(root: &TempDir, vault: &dyn SecretStore) -> (DesktopHost, Process) {
    let mut host = DesktopHost::connect(config(root), vault).await.unwrap();
    let child = host.take_process();
    assert!(child.is_some());
    (host, Process(child))
}

fn ready(info: &serde_json::Value) -> (u16, u16) {
    let peer = info["peerId"].as_str().unwrap();
    assert!(!peer.is_empty());
    let listeners = info["listeners"].as_array().unwrap();
    assert_eq!(
        listeners.len(),
        2,
        "host returned before both listeners were ready"
    );
    let mut tcp = None;
    let mut udp = None;
    for address in listeners {
        let parts = address.as_str().unwrap().split('/').collect::<Vec<_>>();
        assert_eq!(&parts[..3], ["", "ip4", "127.0.0.1"]);
        assert_eq!(parts.last(), Some(&peer));
        let port = parts[4].parse::<u16>().unwrap();
        assert_ne!(port, 0);
        match parts[3] {
            "tcp" => {
                assert_eq!(parts.len(), 7);
                assert_eq!(parts[5], "p2p");
                assert!(tcp.replace(port).is_none());
            }
            "udp" => {
                assert_eq!(parts.len(), 8);
                assert_eq!(&parts[5..7], ["quic-v1", "p2p"]);
                assert!(udp.replace(port).is_none());
            }
            _ => panic!("unexpected listener transport"),
        }
    }
    (tcp.unwrap(), udp.unwrap())
}
#[tokio::test]
async fn first_launch_persists_secrets_before_start_and_reopening_ui_reuses_running_daemon() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    *vault.before_first_save.lock().unwrap() = Some(root.path().into());
    let (host, _process) = start(&root, &vault).await;
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
    let secrets = vault
        .entries
        .lock()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    assert_eq!(secrets.len(), 64);
    assert_ne!(&secrets[..32], &secrets[32..]);
    assert!(secrets.iter().any(|b| *b != 0));
    let identity = host
        .request("create_identity", json!({"name":"Alice"}))
        .await
        .unwrap();
    let info = host.request("node_info", json!({})).await.unwrap();
    ready(&info);
    let listeners = info["listeners"].clone();
    drop(host);
    let mut reopened = DesktopHost::connect(config(&root), &vault).await.unwrap();
    assert!(
        reopened.take_process().is_none(),
        "UI reopen spawned another daemon"
    );
    assert_eq!(
        reopened.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
    assert_eq!(
        reopened.request("node_info", json!({})).await.unwrap()["listeners"],
        listeners
    );
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
    // Only encrypted DB and public route metadata may be written to disk.
    for file in fs::read_dir(root.path()).unwrap() {
        let path = file.unwrap().path();
        if path.is_file() {
            let bytes = fs::read(path).unwrap();
            for secret in [&secrets[..32], &secrets[32..]] {
                assert!(!bytes.windows(32).any(|w| w == secret));
                assert!(
                    !bytes
                        .windows(64)
                        .any(|w| w == hex::encode(secret).as_bytes())
                );
            }
        }
    }
}
#[tokio::test]
async fn locked_or_missing_keychain_never_rekeys_existing_profile() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    let (host, mut process) = start(&root, &vault).await;
    let identity = host
        .request("create_identity", json!({"name":"Alice"}))
        .await
        .unwrap();
    drop(host);
    process.kill();
    let before = fs::read(root.path().join("profile.db")).unwrap();
    vault.fail_get.store(true, Ordering::SeqCst);
    assert!(DesktopHost::connect(config(&root), &vault).await.is_err());
    assert_eq!(fs::read(root.path().join("profile.db")).unwrap(), before);
    vault.fail_get.store(false, Ordering::SeqCst);
    let saved = std::mem::take(&mut *vault.entries.lock().unwrap());
    assert!(DesktopHost::connect(config(&root), &vault).await.is_err());
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
    assert_eq!(fs::read(root.path().join("profile.db")).unwrap(), before);
    *vault.entries.lock().unwrap() = saved;
    let (restored, _process) = start(&root, &vault).await;
    assert_eq!(
        restored.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
}
#[tokio::test]
async fn keychain_write_failure_cannot_create_an_unrecoverable_new_database() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    vault.fail_set.store(true, Ordering::SeqCst);
    assert!(DesktopHost::connect(config(&root), &vault).await.is_err());
    assert!(!root.path().join("profile.db").exists());
    assert!(!root.path().join("node.sock").exists());
    assert!(vault.entries.lock().unwrap().is_empty());
    vault.fail_set.store(false, Ordering::SeqCst);
    let (host, _process) = start(&root, &vault).await;
    assert!(host.request("snapshot", json!({})).await.unwrap()["identity"].is_null());
}
#[tokio::test]
async fn daemon_crash_restarts_with_same_identity_and_advertised_ports() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    let (host, mut process) = start(&root, &vault).await;
    let identity = host
        .request("create_identity", json!({"name":"Alice"}))
        .await
        .unwrap();
    let info = host.request("node_info", json!({})).await.unwrap();
    process.kill();
    assert!(host.request("snapshot", json!({})).await.is_err());
    drop(host);
    let (restored, _process) = start(&root, &vault).await;
    let after = restored.request("node_info", json!({})).await.unwrap();
    ready(&info);
    ready(&after);
    assert_eq!(after["peerId"], info["peerId"]);
    assert_eq!(after["listeners"], info["listeners"]);
    assert_eq!(
        restored.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn independent_profiles_receive_distinct_keys_and_remote_errors_are_not_success() {
    let ar = TempDir::new_in("/tmp").unwrap();
    let br = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    let (a, _ap) = start(&ar, &vault).await;
    let (b, _bp) = start(&br, &vault).await;
    let ai = a
        .request("create_identity", json!({"name":"Alice"}))
        .await
        .unwrap();
    let bi = b
        .request("create_identity", json!({"name":"Bob"}))
        .await
        .unwrap();
    assert_ne!(ai["networkId"], bi["networkId"]);
    let entries = vault
        .entries
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert_ne!(entries[0], entries[1]);
    assert!(
        a.request(
            "send_message",
            json!({"conversationId":"unknown","text":"hello","operationId":"op"})
        )
        .await
        .is_err()
    );
    assert!(a.request("sign_arbitrary", json!({})).await.is_err());
    assert!(
        a.request("snapshot", json!({})).await.unwrap()["conversations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "Interactive system Keychain test; run explicitly when access prompts are acceptable"]
fn system_keychain_roundtrip_uses_unique_test_account_and_removes_it() {
    use agentic_desktop_host::KeychainStore;
    let root = TempDir::new_in("/tmp").unwrap();
    let account = root.path().to_string_lossy().to_string();
    let store = KeychainStore::new("net.agenticinternet.desktop.tests");
    struct Cleanup<'a>(&'a KeychainStore, String);
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            let _ = self.0.remove(&self.1);
        }
    }
    let _cleanup = Cleanup(&store, account.clone());
    assert!(store.get(&account).unwrap().is_none());
    let secret = vec![0x37; 64];
    store.set(&account, &secret).unwrap();
    // Reopening a keychain wrapper reads the same actual OS entry, not an in-process map.
    let reopened = KeychainStore::new("net.agenticinternet.desktop.tests");
    assert_eq!(&*reopened.get(&account).unwrap().unwrap(), &secret);
    reopened.remove(&account).unwrap();
    assert!(reopened.get(&account).unwrap().is_none());
}

#[tokio::test]
async fn concurrent_first_windows_share_one_secret_and_exactly_one_daemon() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    let first = config(&root);
    let second = config(&root);
    let (one, two) = tokio::join!(
        DesktopHost::connect(first, &vault),
        DesktopHost::connect(second, &vault)
    );
    let mut one = one.unwrap();
    let mut two = two.unwrap();
    let p1 = one.take_process();
    let p2 = two.take_process();
    assert_ne!(
        p1.is_some(),
        p2.is_some(),
        "exactly one launch must own the child"
    );
    let _p1 = Process(p1);
    let _p2 = Process(p2);
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
    assert_eq!(vault.entries.lock().unwrap().len(), 1);
    let identity = one
        .request("create_identity", json!({"name":"Alice"}))
        .await
        .unwrap();
    assert_eq!(
        two.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
    let info1 = one.request("node_info", json!({})).await.unwrap();
    let info2 = two.request("node_info", json!({})).await.unwrap();
    ready(&info1);
    ready(&info2);
    assert_eq!(info1["listeners"], info2["listeners"]);
    assert_eq!(info1["peerId"], info2["peerId"]);
}
#[tokio::test]
async fn occupied_saved_port_fails_without_fallback_or_orphan_then_recovers() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    let (host, mut process) = start(&root, &vault).await;
    let identity = host
        .request("create_identity", json!({"name":"Alice"}))
        .await
        .unwrap();
    let info = host.request("node_info", json!({})).await.unwrap();
    let (tcp, udp) = ready(&info);
    process.kill();
    drop(host);
    let occupied = std::net::TcpListener::bind(("127.0.0.1", tcp)).unwrap();
    assert!(
        DesktopHost::connect(config(&root), &vault).await.is_err(),
        "busy saved port must not be replaced silently"
    );
    let free_other_endpoint = std::net::UdpSocket::bind(("127.0.0.1", udp)).unwrap();
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
    drop(occupied);
    drop(free_other_endpoint);
    let (restored, _process) = start(&root, &vault).await;
    let after = restored.request("node_info", json!({})).await.unwrap();
    ready(&after);
    assert_eq!(after["listeners"], info["listeners"]);
    assert_eq!(after["peerId"], info["peerId"]);
    assert_eq!(
        restored.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
}
#[tokio::test]
async fn malformed_keychain_entry_is_not_replaced_and_canonical_paths_reuse_account() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    let (host, mut process) = start(&root, &vault).await;
    let identity = host
        .request("create_identity", json!({"name":"Alice"}))
        .await
        .unwrap();
    let mut alternate = config(&root);
    alternate.data_dir = alternate.data_dir.join(".");
    let mut same = DesktopHost::connect(alternate, &vault).await.unwrap();
    assert!(same.take_process().is_none());
    assert_eq!(
        same.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
    drop(same);
    drop(host);
    process.kill();
    let original = vault.entries.lock().unwrap().clone();
    for value in vault.entries.lock().unwrap().values_mut() {
        *value = vec![0x66; 17];
    }
    assert!(DesktopHost::connect(config(&root), &vault).await.is_err());
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
    *vault.entries.lock().unwrap() = original;
    let (restored, _process) = start(&root, &vault).await;
    assert_eq!(
        restored.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
}
#[tokio::test]
async fn creates_missing_private_directory_and_rejects_shared_existing_directory() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = Vault::default();
    let mut cfg = config(&root);
    cfg.data_dir = root.path().join("new-profile");
    let mut host = DesktopHost::connect(cfg, &vault).await.unwrap();
    let _process = Process(host.take_process());
    assert_eq!(
        fs::metadata(root.path().join("new-profile"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let other = TempDir::new_in("/tmp").unwrap();
    let cfg = config(&other);
    fs::set_permissions(other.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(DesktopHost::connect(cfg, &vault).await.is_err());
    assert!(!other.path().join("profile.db").exists());
    assert_eq!(
        fs::metadata(other.path()).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(vault.writes.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "e2e")]
#[path = "support/e2e_vault.rs"]
mod e2e_vault;
