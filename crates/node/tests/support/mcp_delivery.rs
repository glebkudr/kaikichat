use super::*;

#[test]
fn mcp_delivery_get_tracks_offline_send_and_receipt_after_daemon_restart_for_both_protocols() {
    for modern in [true, false] {
        let mut a = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
        let mut b = Node::start(&["/ip4/127.0.0.1/tcp/0"]);
        let group = connect(&a, &b, None);
        let key = ed25519_dalek::SigningKey::from_bytes(&[116; 32]);
        let grant = grant(&a, &key, &group, json!(["send_message"]));
        let mut mcp = Mcp::start(&credentials(&a, &key, &grant), modern);
        let tools = mcp.request("tools/list", json!({}));
        let tool = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "delivery.get")
            .expect("delivery status tool");
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["idempotentHint"], true);
        let context = runtime_context(&mut mcp);
        let discovered = context["conversations"][0]["id"].as_str().unwrap();
        b.kill();
        let sent=mcp.success("messages.send",json!({"conversationId":discovered,"operationId":"track-offline","text":"Sent while recipient offline"}));
        let request = json!({"conversationId":discovered,"operationId":"track-offline"});
        let mut expected = json!({"conversationId":group,"operationId":"track-offline","messageId":sent["id"],"delivery":{"phase":"queued","replicas":0,"target":10}});
        assert_eq!(mcp.success("delivery.get", request.clone()), expected);
        a.kill();
        a.launch();
        assert_eq!(mcp.success("delivery.get", request.clone()), expected);
        b.launch();
        let received = b.wait(|v| messages(v).len() == 1);
        assert_eq!(messages(&received)[0]["id"], sent["id"]);
        assert_eq!(
            messages(&received)[0]["text"],
            "Sent while recipient offline"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        let delivered = loop {
            let value = mcp.success("delivery.get", request.clone());
            if value["delivery"]["phase"] == "delivered" {
                break value;
            }
            assert!(
                Instant::now() < deadline,
                "receipt not reflected in delivery.get"
            );
            thread::sleep(Duration::from_millis(50));
        };
        expected["delivery"]["phase"] = json!("delivered");
        assert_eq!(delivered, expected);
        let other_key = ed25519_dalek::SigningKey::from_bytes(&[117; 32]);
        let other_grant = super::grant(&a, &other_key, &group, json!(["send_message"]));
        let mut other = Mcp::start(&credentials(&a, &other_key, &other_grant), modern);
        let denied = other.tool("delivery.get", request.clone());
        assert_eq!(denied["result"]["isError"], true);
        assert_eq!(
            denied["result"]["structuredContent"]["error"]["code"],
            "unauthorized"
        );
        assert!(!denied.to_string().contains(sent["id"].as_str().unwrap()));
        a.call("revoke_runtime", json!({"grantId":grant.grant_id}));
        let revoked = mcp.tool("delivery.get", request);
        assert_eq!(revoked["result"]["isError"], true);
        assert_eq!(
            revoked["result"]["structuredContent"]["error"]["code"],
            "unauthorized"
        );
        assert_eq!(messages(&a.snapshot()).len(), 1);
        assert_eq!(messages(&b.snapshot()).len(), 1);
        other.close_cleanly();
        mcp.close_cleanly();
    }
}
