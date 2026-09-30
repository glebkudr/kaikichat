use super::*;

#[tokio::test]
async fn owner_history_pages_real_received_messages_and_rejects_other_windows_and_origins() {
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
    let mut expected = Vec::new();
    for i in 0..55 {
        let text = format!("Native history {i:02} 🧭");
        let sent = a.call("send_message", json!({"request":{"conversationId":group,"text":text,"operationId":format!("native-history-{i}")}})).unwrap();
        expected.push((sent["id"].clone(), text));
    }
    b.wait(|v| v["conversations"][0]["messages"].as_array().unwrap().len() == 55)
        .await;
    let overview = b.call("desktop_overview", json!({"request":{}})).unwrap();
    assert_eq!(overview["conversations"][0]["unread"], 55);
    assert_eq!(
        overview["conversations"][0]["messages"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        overview["conversations"][0]["messages"][0]["id"],
        expected[54].0
    );
    let request = json!({"request":{"conversationId":group,"before":null}});
    let first = b.call("conversation_history", request.clone()).unwrap();
    assert_eq!(first["conversationId"], group);
    assert_eq!(first["messages"].as_array().unwrap().len(), 50);
    let second = b
        .call(
            "conversation_history",
            json!({"request":{"conversationId":group,"before":first["nextBefore"]}}),
        )
        .unwrap();
    assert!(second["nextBefore"].is_null());
    let all = second["messages"]
        .as_array()
        .unwrap()
        .iter()
        .chain(first["messages"].as_array().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(all.len(), 55);
    let author = a.snapshot()["identity"]["networkId"].clone();
    for (message, (id, text)) in all.into_iter().zip(expected) {
        assert_eq!(message["id"], id);
        assert_eq!(message["text"], text);
        assert_eq!(message["author"], author);
        assert_eq!(message["own"], false);
    }
    let other = tauri::WebviewWindowBuilder::new(
        &b.app,
        "untrusted-history",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .build()
    .unwrap();
    for (method, body) in [
        ("desktop_overview", json!({"request":{}})),
        ("conversation_history", request),
    ] {
        assert!(invoke(&other, "tauri://localhost", method, body.clone()).is_err());
        assert!(invoke(&b.window, "https://untrusted.example", method, body).is_err());
    }
    assert!(
        b.call(
            "conversation_history",
            json!({"request":{"conversationId":group,"before":"malformed"}})
        )
        .is_err()
    );
    assert!(
        b.call(
            "conversation_history",
            json!({"request":{"conversationId":group,"limit":100000}})
        )
        .is_err()
    );
    assert_eq!(
        b.call("desktop_overview", json!({"request":{}})).unwrap(),
        overview
    );
}
