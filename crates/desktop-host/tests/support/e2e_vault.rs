use super::*;
use agentic_desktop_host::E2eFileStore;
use sha2::{Digest, Sha256};

fn account(root: &TempDir) -> String {
    let profile = root.path().canonicalize().unwrap().join("profile.db");
    hex::encode(Sha256::digest(profile.as_os_str().as_encoded_bytes()))
}

#[tokio::test]
async fn e2e_file_secrets_survive_real_daemon_and_vault_restart() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = E2eFileStore::new(&config(&root).data_dir).unwrap();
    assert!(vault.get(&account(&root)).unwrap().is_none());
    let (host, mut process) = start(&root, &vault).await;
    let identity = host
        .request("create_identity", json!({"name":"Isolated E2E"}))
        .await
        .unwrap();
    let secret = vault.get(&account(&root)).unwrap().unwrap();
    assert_eq!(secret.len(), 64);
    assert!(secret.iter().any(|b| *b != 0));
    let path = root.path().join(".e2e-secrets");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let before = fs::read(&path).unwrap();
    drop(host);
    process.kill();
    drop(vault);
    let reopened = E2eFileStore::new(root.path()).unwrap();
    let (host, _process) = start(&root, &reopened).await;
    let snapshot = host.request("snapshot", json!({})).await.unwrap();
    assert_eq!(snapshot["identity"], identity);
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "restart must not replace the database key"
    );
    assert!(!snapshot.to_string().contains(&hex::encode(&secret[..32])));
    assert!(!snapshot.to_string().contains(&hex::encode(&secret[32..])));
}

#[tokio::test]
async fn e2e_missing_or_exposed_secret_refuses_existing_profile_without_rekeying() {
    let root = TempDir::new_in("/tmp").unwrap();
    let vault = E2eFileStore::new(&config(&root).data_dir).unwrap();
    let (host, mut process) = start(&root, &vault).await;
    let identity = host
        .request("create_identity", json!({"name":"Keep original"}))
        .await
        .unwrap();
    drop(host);
    process.kill();
    let path = root.path().join(".e2e-secrets");
    let bytes = fs::read(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(DesktopHost::connect(config(&root), &vault).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let parked = root.path().join("parked-test-key");
    fs::rename(&path, &parked).unwrap();
    assert!(DesktopHost::connect(config(&root), &vault).await.is_err());
    assert!(
        !path.exists(),
        "missing key must not silently generate a new one for an existing database"
    );
    fs::rename(&parked, &path).unwrap();
    let database = fs::read(root.path().join("profile.db")).unwrap();
    fs::write(&path, &bytes[..31]).unwrap();
    assert!(DesktopHost::connect(config(&root), &vault).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), &bytes[..31]);
    assert_eq!(fs::read(root.path().join("profile.db")).unwrap(), database);
    fs::write(&path, &bytes).unwrap();
    let (host, _process) = start(&root, &vault).await;
    assert_eq!(
        host.request("snapshot", json!({})).await.unwrap()["identity"],
        identity
    );
}

#[test]
fn e2e_fixture_seed_is_private_scoped_and_never_overwrites_an_existing_key() {
    let root = TempDir::new_in("/tmp").unwrap();
    let other = TempDir::new_in("/tmp").unwrap();
    let vault = E2eFileStore::new(&config(&root).data_dir).unwrap();
    let secret = vec![0x75; 64];
    assert!(vault.set(&account(&other), &secret).is_err());
    assert!(vault.set(&account(&root), &secret[..32]).is_err());
    assert!(!root.path().join(".e2e-secrets").exists());
    vault.set(&account(&root), &secret).unwrap();
    assert_eq!(fs::read(root.path().join(".e2e-secrets")).unwrap(), secret);
    assert!(vault.get(&account(&other)).is_err());
    assert_eq!(&*vault.get(&account(&root)).unwrap().unwrap(), &secret);
    assert!(vault.set(&account(&root), &[0x19; 64]).is_err());
    assert_eq!(&*vault.get(&account(&root)).unwrap().unwrap(), &secret);
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(E2eFileStore::new(root.path()).is_err());
}
