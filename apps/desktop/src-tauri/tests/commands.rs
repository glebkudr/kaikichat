#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use agentic_desktop::{NativeBridge, configure, start_updates};
use agentic_desktop_host::{DesktopHost, HostConfig, Result, SecretStore};
use serde_json::{Value, json};
use std::{
    process::Child,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    Listener,
    ipc::{CallbackFn, InvokeBody},
    test::{MockRuntime, get_ipc_response, mock_builder},
    webview::InvokeRequest,
};
use tempfile::TempDir;
use zeroize::Zeroizing;

#[path = "support/history.rs"]
mod history;
#[path = "support/owner.rs"]
mod owner;
#[path = "support/profile.rs"]
mod profile;

#[tokio::test]
async fn owner_window_provisions_lists_and_revokes_runtime_without_exposing_scoped_seed() {
    let a = Fixture::new().await;
    let b = Fixture::new().await;
    a.call("create_identity", json!({"request":{"name":"Alice"}}))
        .unwrap();
    b.call("create_identity", json!({"request":{"name":"Bob"}}))
        .unwrap();
    let invitation = b.call("create_invitation", json!({"request":{}})).unwrap();
    let conversation = a
        .call(
            "add_contact",
            json!({"request":{"name":"Bob","invitation":invitation}}),
        )
        .unwrap();
    b.wait(|v| v["conversations"].as_array().unwrap().len() == 1)
        .await;
    let other = tauri::WebviewWindowBuilder::new(
        &a.app,
        "untrusted",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .build()
    .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let request = json!({"request":{"operationId":"native-provision","name":"Native assistant","agentId":vec![81;32],"serviceId":vec![82;32],"conversationIds":[conversation["id"]],"actions":["read_inbox"],"expiresAt":now+3600,"maxDataBytes":4096}});
    for (window, origin) in [
        (&other, "tauri://localhost"),
        (&a.window, "https://untrusted.example"),
    ] {
        assert!(invoke(window, origin, "provision_runtime", request.clone()).is_err());
        assert!(invoke(window, origin, "list_runtimes", json!({})).is_err());
    }
    assert_eq!(a.call("list_runtimes", json!({})).unwrap(), json!([]));
    let setup = a.call("provision_runtime", request.clone()).unwrap();
    assert_eq!(
        setup["runtime"]["conversationIds"],
        json!([conversation["id"]])
    );
    assert_eq!(setup["runtime"]["actions"], json!(["read_inbox"]));
    assert_eq!(setup["runtime"]["status"], "active");
    for field in ["expiresAt", "maxDataBytes", "agentId", "serviceId"] {
        assert_eq!(setup["runtime"][field], request["request"][field]);
    }
    assert_eq!(a.call("provision_runtime", request.clone()).unwrap(), setup);
    let listed = a.call("list_runtimes", json!({})).unwrap();
    assert_eq!(listed, json!([setup["runtime"]]));
    let path = std::path::PathBuf::from(setup["credentialsPath"].as_str().unwrap());
    assert!(path.starts_with(std::fs::canonicalize(a._root.path()).unwrap()));
    let credentials: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let seed = credentials["signingSeed"].as_str().unwrap();
    assert_eq!(seed.len(), 64);
    for public in [&setup, &listed, &a.snapshot()] {
        assert!(!public.to_string().contains(seed));
        assert!(!public.to_string().contains("ownerToken"));
        assert!(!public.to_string().contains("masterKey"));
    }
    let revoke = json!({"request":{"grantId":setup["runtime"]["grantId"]}});
    for (window, origin) in [
        (&other, "tauri://localhost"),
        (&a.window, "https://untrusted.example"),
    ] {
        assert!(invoke(window, origin, "revoke_runtime", revoke.clone()).is_err());
        assert_eq!(a.call("list_runtimes", json!({})).unwrap(), listed);
    }
    a.call("revoke_runtime", revoke).unwrap();
    assert_eq!(
        a.call("list_runtimes", json!({})).unwrap()[0]["status"],
        "revoked"
    );
    assert_eq!(a.snapshot()["conversations"][0]["messages"], json!([]));
    let before = a.call("list_runtimes", json!({})).unwrap();
    let mut unrestricted = request["request"].clone();
    unrestricted.as_object_mut().unwrap().remove("operationId");
    // A real, now revoked principal is available for a fresh grant to different logical IDs.
    unrestricted["principal"] = setup["runtime"]["principal"].clone();
    unrestricted["agentId"] = json!(vec![83; 32]);
    unrestricted["serviceId"] = json!(vec![84; 32]);
    let rejected = a
        .call("grant_runtime", json!({"request":unrestricted}))
        .unwrap_err();
    assert!(
        rejected
            .as_str()
            .unwrap()
            .starts_with("grant_runtime not allowed.")
    );
    assert_eq!(a.call("list_runtimes", json!({})).unwrap(), before);
}
#[derive(Default)]
struct Vault(Mutex<Option<Vec<u8>>>);
impl SecretStore for Vault {
    fn get(&self, _: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        Ok(self.0.lock().unwrap().clone().map(Zeroizing::new))
    }
    fn set(&self, _: &str, value: &[u8]) -> Result<()> {
        *self.0.lock().unwrap() = Some(value.to_vec());
        Ok(())
    }
}

#[tokio::test]
async fn main_window_receives_remote_changes_and_one_daemon_failure_transition() {
    let a = Fixture::new().await;
    let mut b = Fixture::new().await;
    a.call("create_identity", json!({"request":{"name":"Alice"}}))
        .unwrap();
    b.call("create_identity", json!({"request":{"name":"Bob"}}))
        .unwrap();
    let invitation = b.call("create_invitation", json!({"request":{}})).unwrap();
    let conversation = a
        .call(
            "add_contact",
            json!({"request":{"name":"Bob","invitation":invitation}}),
        )
        .unwrap();
    b.wait(|v| v["conversations"].as_array().unwrap().len() == 1)
        .await;
    let (tx, mut events) = tokio::sync::mpsc::unbounded_channel();
    b.window.listen("core:changed", move |event| {
        // Event is only an invalidation signal; profile/keys must not be broadcast.
        tx.send(event.payload().to_owned()).unwrap();
    });
    struct Updates(tauri::async_runtime::JoinHandle<()>);
    impl Drop for Updates {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _updates = Updates(start_updates(b.app.handle().clone()));
    let initial = tokio::time::timeout(Duration::from_secs(5), events.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(initial, "null");
    a.call("send_message", json!({"request":{"conversationId":conversation["id"],"text":"Arrived while Bob is idle","operationId":"event-incoming"}})).unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            assert_eq!(events.recv().await.unwrap(), "null");
            let state = b.snapshot();
            if state["conversations"][0]["messages"]
                .as_array()
                .unwrap()
                .len()
                == 1
            {
                assert_eq!(
                    state["conversations"][0]["messages"][0]["text"],
                    "Arrived while Bob is idle"
                );
                break;
            }
        }
    })
    .await
    .unwrap();
    // A stable snapshot must not trigger periodic WebView reloads.
    tokio::time::sleep(Duration::from_millis(700)).await;
    while events.try_recv().is_ok() {}
    assert!(
        tokio::time::timeout(Duration::from_millis(700), events.recv())
            .await
            .is_err()
    );
    let child = b._process.0.as_mut().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .unwrap()
            .unwrap(),
        "null"
    );
    assert!(b.call("snapshot", json!({})).is_err());
    assert!(
        tokio::time::timeout(Duration::from_millis(700), events.recv())
            .await
            .is_err(),
        "unchanged failure must not flood the UI"
    );
}
struct Fixture {
    window: tauri::WebviewWindow<MockRuntime>,
    app: tauri::App<MockRuntime>,
    _process: Process,
    _root: TempDir,
    /// Links the window asked the system to open.
    opened: Arc<Opened>,
    /// The home the window installs skills under.
    home: std::path::PathBuf,
}
/// Records links instead of opening a browser or a wallet.
#[derive(Default)]
struct Opened(Mutex<Vec<String>>);
impl agentic_desktop::Opener for Opened {
    fn open(&self, link: &str) -> std::result::Result<(), String> {
        self.0.lock().unwrap().push(link.to_owned());
        Ok(())
    }
}
struct Process(Option<Child>);
impl Drop for Process {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
impl Fixture {
    async fn new() -> Self {
        Self::with_args(vec![]).await
    }
    /// A window on a daemon started with more `serve` flags.
    async fn with_args(serve_args: Vec<String>) -> Self {
        let root = TempDir::new_in("/tmp").unwrap();
        let config = HostConfig {
            data_dir: root.path().join("profile"),
            node_binary: std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("agentic-node"),
            listen: vec!["/ip4/127.0.0.1/tcp/0".into()],
            serve_args,
        };
        let mut host = DesktopHost::connect(config, &Vault::default())
            .await
            .unwrap();
        let process = host.take_process();
        assert!(process.is_some());
        let process = Process(process);
        let opened = Arc::new(Opened::default());
        let home = root.path().join("home");
        let app = configure(mock_builder())
            .manage(
                NativeBridge::connected(Arc::new(host))
                    .with_opener(opened.clone())
                    .with_home(home.clone()),
            )
            .build(tauri::generate_context!())
            .unwrap();
        let window = tauri::WebviewWindowBuilder::new(
            &app,
            "main",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .build()
        .unwrap();
        Self {
            window,
            app,
            _process: process,
            _root: root,
            opened,
            home,
        }
    }
    fn call(&self, method: &str, body: Value) -> std::result::Result<Value, Value> {
        invoke(&self.window, "tauri://localhost", method, body)
    }
    fn snapshot(&self) -> Value {
        self.call("snapshot", json!({})).unwrap()
    }
    /// A command that must be refused by the daemon or the bridge: its
    /// `{code, message, retryable}`.
    fn refusal(&self, method: &str, body: Value) -> Value {
        let error = self
            .call(method, body)
            .expect_err(&format!("{method} must be refused"));
        assert!(error["code"].is_string(), "{method}: {error}");
        assert!(error["message"].is_string(), "{method}: {error}");
        assert!(error["retryable"].is_boolean(), "{method}: {error}");
        error
    }
    fn identity(&self, name: &str) -> Value {
        self.call("create_identity", json!({"request":{"name":name}}))
            .unwrap()
    }
    fn opened(&self) -> Vec<String> {
        self.opened.0.lock().unwrap().clone()
    }
    async fn wait(&self, predicate: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let value = self.snapshot();
            if predicate(&value) {
                return value;
            }
            assert!(
                Instant::now() < deadline,
                "native snapshot did not converge: {value}"
            );
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    }
}
fn invoke(
    window: &tauri::WebviewWindow<MockRuntime>,
    url: &str,
    method: &str,
    body: Value,
) -> std::result::Result<Value, Value> {
    get_ipc_response(
        window,
        InvokeRequest {
            cmd: method.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map(|value| value.deserialize().unwrap())
}
#[tokio::test]
async fn native_commands_return_real_profile_and_invitation_without_secret_fields() {
    let f = Fixture::new().await;
    assert!(f.snapshot()["identity"].is_null());
    assert!(
        f.call("create_identity", json!({"request":{"name":""}}))
            .is_err()
    );
    assert!(f.snapshot()["identity"].is_null());
    let identity = f
        .call("create_identity", json!({"request":{"name":"Alice"}}))
        .unwrap();
    assert_eq!(identity["name"], "Alice");
    assert!(identity["networkId"].as_str().unwrap().starts_with("ain1"));
    let invitation = f.call("create_invitation", json!({"request":{}})).unwrap();
    assert!(invitation.as_str().unwrap().starts_with("ain-invite1:"));
    assert_eq!(f.snapshot()["identity"], identity);
    for name in [
        "masterKey",
        "ownerToken",
        "owner_seed",
        "mls",
        "transport/identity",
    ] {
        assert!(!f.snapshot().to_string().contains(name));
    }
    assert!(
        f.call("node_info", json!({})).is_err(),
        "internal owner API is not automatically a WebView command"
    );
    assert!(f.call("sign_arbitrary", json!({})).is_err());
}
#[tokio::test]
async fn native_ipc_commands_drive_two_real_daemons_through_invite_mls_and_receipt() {
    let a = Fixture::new().await;
    let b = Fixture::new().await;
    a.call("create_identity", json!({"request":{"name":"Alice"}}))
        .unwrap();
    b.call("create_identity", json!({"request":{"name":"Bob"}}))
        .unwrap();
    let invitation = b.call("create_invitation", json!({"request":{}})).unwrap();
    let conversation = a
        .call(
            "add_contact",
            json!({"request":{"name":"Bob","invitation":invitation}}),
        )
        .unwrap();
    let group = conversation["id"].as_str().unwrap();
    b.wait(|v| v["conversations"].as_array().unwrap().len() == 1)
        .await;
    let sent=a.call("send_message",json!({"request":{"conversationId":group,"text":"From native command to MLS","operationId":"native-op"}})).unwrap();
    let received = b
        .wait(|v| v["conversations"][0]["messages"].as_array().unwrap().len() == 1)
        .await;
    assert_eq!(
        received["conversations"][0]["messages"][0]["text"],
        "From native command to MLS"
    );
    assert_eq!(
        received["conversations"][0]["messages"][0]["id"],
        sent["id"]
    );
    a.wait(|v| v["conversations"][0]["messages"][0]["delivery"]["phase"] == "delivered")
        .await;
    let retry=a.call("send_message",json!({"request":{"conversationId":group,"text":"From native command to MLS","operationId":"native-op"}})).unwrap();
    assert_eq!(retry["id"], sent["id"]);
    assert!(a.call("send_message",json!({"request":{"conversationId":group,"text":"Changed retry","operationId":"native-op"}})).is_err());
    assert_eq!(
        a.snapshot()["conversations"][0]["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[tokio::test]
async fn secondary_window_and_foreign_origin_cannot_mutate_owner_profile() {
    let f = Fixture::new().await;
    let other = tauri::WebviewWindowBuilder::new(
        &f.app,
        "untrusted",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .build()
    .unwrap();
    let request = json!({"request":{"name":"Intruder"}});
    assert!(
        invoke(
            &other,
            "tauri://localhost",
            "create_identity",
            request.clone()
        )
        .is_err()
    );
    assert!(f.snapshot()["identity"].is_null());
    assert!(
        invoke(
            &f.window,
            "https://untrusted.example",
            "create_identity",
            request
        )
        .is_err()
    );
    assert!(f.snapshot()["identity"].is_null());
    f.call(
        "create_identity",
        json!({"request":{"name":"Actual owner"}}),
    )
    .unwrap();
    assert_eq!(f.snapshot()["identity"]["name"], "Actual owner");
    assert!(invoke(&other, "tauri://localhost", "snapshot", json!({})).is_err());
    assert!(
        invoke(
            &f.window,
            "https://untrusted.example",
            "snapshot",
            json!({})
        )
        .is_err()
    );
    assert_eq!(f.snapshot()["identity"]["name"], "Actual owner");
}

#[tokio::test]
async fn only_local_owner_can_inspect_and_change_live_network_preferences() {
    let a = Fixture::new().await;
    let peer = Fixture::new().await;
    let identity = a
        .call("create_identity", json!({"request":{"name":"Alice"}}))
        .unwrap();
    let initial = a.call("network_settings", json!({})).unwrap();
    assert_eq!(initial["revision"], 0);
    assert_eq!(
        initial["preferences"],
        json!({"relays":[],"relayOnly":false,"autoNatPeers":[],"bootstrapPeers":[],"lanDiscovery":false,"dhtServer":false})
    );
    assert_eq!(initial["status"]["routing"]["mode"], "client");
    let other = tauri::WebviewWindowBuilder::new(
        &a.app,
        "untrusted",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .build()
    .unwrap();
    // A real independent peer provides a syntactically valid endpoint. It has not opted into
    // relay/AutoNAT service, so saving it must not claim a confirmed reservation or public IP.
    let endpoint =
        peer.call("network_settings", json!({})).unwrap()["status"]["listeners"][0].clone();
    let preferences = json!({"relays":[endpoint],"relayOnly":true,"autoNatPeers":[endpoint],"bootstrapPeers":[endpoint],"lanDiscovery":true,"dhtServer":true});
    let request = json!({"request":{"expectedRevision":0,"preferences":preferences}});
    for (window, origin) in [
        (&other, "tauri://localhost"),
        (&a.window, "https://untrusted.example"),
    ] {
        let denied = invoke(window, origin, "network_settings", json!({})).unwrap_err();
        let denied = denied.as_str().unwrap();
        assert!(
            denied.starts_with("network_settings not allowed on window ")
                || denied == "Command network_settings not allowed by ACL",
            "{denied}"
        );
        let denied = invoke(window, origin, "configure_network", request.clone()).unwrap_err();
        let denied = denied.as_str().unwrap();
        assert!(
            denied.starts_with("configure_network not allowed on window ")
                || denied == "Command configure_network not allowed by ACL",
            "{denied}"
        );
        let unchanged = a.call("network_settings", json!({})).unwrap();
        assert_eq!(unchanged["revision"], initial["revision"]);
        assert_eq!(unchanged["preferences"], initial["preferences"]);
    }
    let saved = a.call("configure_network", request.clone()).unwrap();
    assert_eq!(saved["revision"], 1);
    assert_eq!(saved["preferences"], preferences);
    assert_eq!(saved["status"]["routing"]["mode"], "disabled");
    assert_eq!(saved["status"]["routing"]["blockedByPolicy"], true);
    assert_eq!(saved["status"]["peerId"], initial["status"]["peerId"]);
    assert_eq!(saved["status"]["relayRoutes"], json!([]));
    assert!(saved["status"]["autoNat"]["publicAddress"].is_null());
    let retried = a.call("configure_network", request).unwrap();
    assert_eq!(retried["revision"], 1);
    assert_eq!(retried["preferences"], preferences);
    let malformed = json!({"request":{"expectedRevision":1,"preferences":{"relays":[],"relayOnly":true,"autoNatPeers":[]}}});
    assert!(a.call("configure_network", malformed).is_err());
    let stale = json!({"request":{"expectedRevision":0,"preferences":initial["preferences"]}});
    assert!(a.call("configure_network", stale).is_err());
    let unchanged = a.call("network_settings", json!({})).unwrap();
    assert_eq!(unchanged["revision"], 1);
    assert_eq!(unchanged["preferences"], preferences);
    assert_eq!(a.snapshot()["identity"], identity);
    for view in [&saved, &unchanged] {
        for secret in [
            "masterKey",
            "ownerToken",
            "signingSeed",
            "transport/identity",
        ] {
            assert!(!view.to_string().contains(secret));
        }
    }
    a.call(
        "configure_network",
        json!({"request":{"expectedRevision":1,"preferences":initial["preferences"]}}),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let restored = a.call("network_settings", json!({})).unwrap();
        assert_eq!(restored["revision"], 2);
        assert_eq!(restored["preferences"], initial["preferences"]);
        if restored["status"]["listeners"] == initial["status"]["listeners"] {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "listen ports were not preserved: {restored}"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert!(a.call("node_info", json!({})).is_err());
}
