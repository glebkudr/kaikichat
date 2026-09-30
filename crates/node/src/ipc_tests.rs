#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use agentic_core::AppCore;
use agentic_store::ProfileStore;

#[tokio::test]
async fn disconnected_ipc_marks_queued_command_cancelled_before_real_core_execution() {
    let root = tempfile::Builder::new()
        .prefix("ain-ipc-cancel-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().join("node.sock");
    let (listener, _guard) = bind(&path).unwrap();
    let (sent, mut commands) = mpsc::channel(2);
    let server = tokio::spawn(serve(listener, Arc::new(Zeroizing::new([22; 32])), sent));
    let mut core = AppCore::new(
        ProfileStore::open(root.path().join("profile.db"), &[11; 32]).unwrap(),
        crate::NETWORK_DOMAIN,
    )
    .unwrap();
    let cancelled_path = path.clone();
    let client = tokio::spawn(async move {
        call(
            &cancelled_path,
            &[22; 32],
            "create_identity",
            json!({"name":"Abandoned"}),
        )
        .await
    });
    let mut queued = tokio::time::timeout(Duration::from_secs(2), commands.recv())
        .await
        .unwrap()
        .unwrap();
    client.abort();
    assert!(client.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(1), queued.reply.closed())
        .await
        .expect("IPC must notice disconnect before its 5s request deadline");
    queued.execute(|_| json!(core.create_profile("Must never be created").unwrap()));
    assert!(core.snapshot().unwrap().identity.is_none());
    let active_path = path.clone();
    let client = tokio::spawn(async move {
        call(
            &active_path,
            &[22; 32],
            "create_identity",
            json!({"name":"Actual owner"}),
        )
        .await
    });
    let queued = tokio::time::timeout(Duration::from_secs(2), commands.recv())
        .await
        .unwrap()
        .unwrap();
    queued.execute(|kind| match kind {
        CommandKind::Owner { method, request } => {
            assert_eq!(method, "create_identity");
            json!({"result":core.create_profile(request["name"].as_str().unwrap()).unwrap()})
        }
        CommandKind::Agent { .. } => panic!("owner request changed authority branch"),
    });
    let response = client.await.unwrap().unwrap();
    assert_eq!(response["result"]["name"], "Actual owner");
    assert_eq!(
        core.snapshot().unwrap().identity.unwrap().name,
        "Actual owner"
    );
    server.abort();
    let _ = server.await;
}
