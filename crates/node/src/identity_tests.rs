//! The claim client against a local stub of the identity server's API
//! (`services/identity-server/src/lib.rs`: `POST /v1/claims`,
//! `GET /v1/claims/{id}`).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use agentic_grant_book::{GrantRevocation, GrantTerms, SecpKey};
use agentic_mailbox_swarm::stamp::{BookKey, Stamp};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A server answering each `(method, path)` from `answer` with a status and
/// a JSON body, keeping every request line and body.
struct Stub {
    url: String,
    seen: Arc<Mutex<Vec<(String, Value)>>>,
}

type Answer = dyn Fn(&str, &Value) -> (u16, Value) + Send + Sync;

async fn stub(answer: impl Fn(&str, &Value) -> (u16, Value) + Send + Sync + 'static) -> Stub {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = seen.clone();
    let answer: Arc<Answer> = Arc::new(answer);
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let (kept, answer) = (kept.clone(), answer.clone());
            tokio::spawn(async move {
                let mut buffer = Vec::new();
                loop {
                    let Some((line, body)) = read_request(&mut socket, &mut buffer).await else {
                        return;
                    };
                    let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                    kept.lock().unwrap().push((line.clone(), body.clone()));
                    let (status, reply) = answer(&line, &body);
                    let reply = serde_json::to_vec(&reply).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
                        reply.len()
                    );
                    if socket.write_all(head.as_bytes()).await.is_err()
                        || socket.write_all(&reply).await.is_err()
                    {
                        return;
                    }
                }
            });
        }
    });
    Stub { url, seen }
}

/// The request line (`METHOD /path`) and body of one HTTP/1.1 request.
async fn read_request(
    socket: &mut tokio::net::TcpStream,
    buffer: &mut Vec<u8>,
) -> Option<(String, Vec<u8>)> {
    loop {
        if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buffer[..end]).to_string();
            let line = head
                .lines()
                .next()
                .unwrap_or("")
                .rsplit_once(' ')
                .map_or(String::new(), |(line, _)| line.to_owned());
            let length: usize = head
                .to_ascii_lowercase()
                .lines()
                .find_map(|l| l.strip_prefix("content-length:").map(str::to_owned))
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0);
            if buffer.len() >= end + 4 + length {
                let body = buffer[end + 4..end + 4 + length].to_vec();
                buffer.drain(..end + 4 + length);
                return Some((line, body));
            }
        }
        let mut chunk = [0; 4096];
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
}

fn request() -> ClaimRequest {
    ClaimRequest::sign(
        [0xd0; 32],
        [7; 16],
        1_800_000_000,
        &SecpKey::from_secret(&[0x41; 32]).unwrap(),
    )
}

fn grant() -> GrantBook {
    GrantBook::issue(
        GrantTerms {
            domain: [0xd0; 32],
            book: SecpKey::from_secret(&[0x41; 32]).unwrap().account(),
            day: 20_833,
            serial: 3,
            count: 50,
            expiry: 1_802_592_000,
        },
        &SecpKey::from_secret(&[0x31; 32]).unwrap(),
    )
}

#[tokio::test]
async fn a_claim_is_posted_and_read_until_it_is_decided() {
    let polls = Arc::new(Mutex::new(0));
    let counted = polls.clone();
    let server = stub(move |line, _body| match line {
        // A repeated request gets the same claim with 200 instead of 201.
        "POST /v1/claims" => (
            if counted.lock().unwrap().eq(&0) { 201 } else { 200 },
            json!({"claimId": "c1", "loginUrl": "https://id.test/v1/claims/c1/login", "expiresAt": 1_800_000_900u64}),
        ),
        "GET /v1/claims/c1" => {
            let mut polls = counted.lock().unwrap();
            *polls += 1;
            match *polls {
                1 => (200, json!({"status": "pending"})),
                2 => (200, json!({"status": "granted", "grant": grant()})),
                _ => (200, json!({"status": "denied", "reason": "already_claimed"})),
            }
        }
        other => panic!("unexpected {other}"),
    })
    .await;
    // A trailing slash in the configured URL makes no `//v1` path.
    let client = IdentityClient::new(&format!("{}/", server.url)).unwrap();
    assert_eq!(
        client.create(&request()).await,
        Ok(ClaimOpened {
            claim_id: "c1".into(),
            login_url: "https://id.test/v1/claims/c1/login".into(),
            expires_at: 1_800_000_900,
        })
    );
    // The body is the server's `CreateClaim`: the signed request.
    let seen = server.seen.lock().unwrap().clone();
    assert_eq!(seen[0].1, json!({"request": request()}));
    assert_eq!(client.status("c1").await, Ok(ClaimStatus::Pending));
    assert_eq!(client.status("c1").await, Ok(ClaimStatus::Granted(grant())));
    assert_eq!(
        client.status("c1").await,
        Ok(ClaimStatus::Denied("already_claimed".into()))
    );
    // Asked again after the first answer, the same claim.
    assert_eq!(client.create(&request()).await.unwrap().claim_id, "c1");
}

#[tokio::test]
async fn a_refusal_carries_the_servers_code_and_a_failure_is_transport() {
    let server = stub(|line, _body| match line {
        "POST /v1/claims" => (400, json!({"error": "stale_request"})),
        _ => (404, json!({"error": "unknown_claim"})),
    })
    .await;
    let client = IdentityClient::new(&server.url).unwrap();
    assert_eq!(
        client.create(&request()).await,
        Err(IdentityError::Refused("stale_request".into()))
    );
    assert_eq!(
        client.status("gone").await,
        Err(IdentityError::Refused("unknown_claim".into()))
    );
    // A server or proxy failure is not the server's decision: a claim the
    // human already signed in for must not be dropped over it.
    for (status, body) in [
        (500, json!({"error": "internal_error"})),
        (502, json!("<html>Bad Gateway</html>")),
    ] {
        let failing = stub(move |_line, _body| (status, body.clone())).await;
        let client = IdentityClient::new(&failing.url).unwrap();
        assert!(
            matches!(client.status("c1").await, Err(IdentityError::Transport(_))),
            "{status}"
        );
        assert!(
            matches!(
                client.create(&request()).await,
                Err(IdentityError::Transport(_))
            ),
            "{status}"
        );
    }
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let client = IdentityClient::new(&format!("http://{closed}")).unwrap();
    assert!(matches!(
        client.status("c1").await,
        Err(IdentityError::Transport(_))
    ));
}

fn spent(operation: u8) -> Stamp {
    Stamp::sign(
        &[0xd0; 32],
        grant().id(),
        9,
        [operation; 32],
        &BookKey::from_bytes(&[0x41; 32]).unwrap(),
    )
}

#[tokio::test]
async fn a_double_spend_is_reported_and_revocations_are_read_after_a_cursor() {
    let revocation = GrantRevocation::issue(
        &grant(),
        1_800_000_000,
        &SecpKey::from_secret(&[0x31; 32]).unwrap(),
    );
    let listed = revocation.clone();
    let server = stub(move |line, _body| match line {
        "POST /v1/reports" => (200, json!({"banned": true, "revoked": 2})),
        "GET /v1/revocations?after=0" => (200, json!({"revocations": [listed], "last": 4})),
        other => panic!("unexpected {other}"),
    })
    .await;
    let client = IdentityClient::new(&server.url).unwrap();
    assert_eq!(
        client.report(&grant(), &spent(1), &spent(2)).await,
        Ok(Reported {
            banned: true,
            revoked: 2
        })
    );
    // The body is the server's report: the grant and both stamps as the
    // directory's HTTP API shows stamps.
    let seen = server.seen.lock().unwrap().clone();
    let shown = |stamp: Stamp| {
        json!({"book": hex::encode(stamp.book), "index": 9,
            "operation": hex::encode(stamp.operation), "signature": hex::encode(stamp.signature)})
    };
    assert_eq!(
        seen[0].1,
        json!({"grant": grant(), "first": shown(spent(1)), "second": shown(spent(2))})
    );
    assert_eq!(
        client.revocations(0).await,
        Ok(RevocationPage {
            revocations: vec![revocation],
            last: 4
        })
    );
    // A report the server finds no proof in is its final word.
    let refusing = stub(|_line, _body| (400, json!({"error": "no_proof"}))).await;
    let client = IdentityClient::new(&refusing.url).unwrap();
    assert_eq!(
        client.report(&grant(), &spent(1), &spent(1)).await,
        Err(IdentityError::Refused("no_proof".into()))
    );
}
