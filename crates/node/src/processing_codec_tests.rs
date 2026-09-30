//! Actual libp2p handler lifetime: a decoded request still owns resources while
//! the application withholds its response. No synthetic codec futures or peers.
use super::super::processing as processing_io;
use super::*;
use request_response::{Event, Message, ResponseChannel};

type Server = Swarm<processing_io::Behaviour<cbor::codec::Codec<Exchange, Exchange>>>;
type Client = Swarm<cbor::Behaviour<Exchange, Exchange>>;

enum Observed {
    Decoded(PeerId, Vec<u8>, ResponseChannel<Exchange>),
    Response(Vec<u8>),
    Refused(request_response::OutboundRequestId),
    Cancelled,
}

async fn next(server: &mut Server, client: &mut Client) -> Observed {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            tokio::select! {
                event = server.select_next_some() => match event {
                    SwarmEvent::Behaviour(Event::Message { peer, message: Message::Request { request, channel, .. }, .. }) =>
                        return Observed::Decoded(peer, request.node_record, channel),
                    SwarmEvent::Behaviour(Event::InboundFailure { .. }) => return Observed::Cancelled,
                    _ => {}
                },
                event = client.select_next_some() => match event {
                    SwarmEvent::Behaviour(Event::Message { message: Message::Response { response, .. }, .. }) =>
                        return Observed::Response(response.node_record),
                    SwarmEvent::Behaviour(Event::OutboundFailure { request_id, .. }) => return Observed::Refused(request_id),
                    _ => {}
                }
            }
        }
    }).await.unwrap_or_else(|_| panic!("actual request/response event before server timeout"))
}

#[tokio::test]
async fn decoded_request_retains_shared_charge_until_response_cancellation_or_swarm_drop() {
    let budget = Limits::new(2).processing();
    let mut server = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|_| bootstrap_support::behaviour(budget.clone()))
        .unwrap()
        .build();
    server
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let address = loop {
        if let SwarmEvent::NewListenAddr { address, .. } = server.select_next_some().await {
            break address;
        }
    };
    let server_peer = *server.local_peer_id();
    let mut client = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_behaviour(|_| {
            cbor::Behaviour::<Exchange, Exchange>::new(
                [(
                    StreamProtocol::new("/agentic-internet/bootstrap/1"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default()
                    .with_request_timeout(Duration::from_secs(5))
                    .with_max_concurrent_streams(8),
            )
        })
        .unwrap()
        .build();
    let client_peer = *client.local_peer_id();
    client
        .dial(address.with(Protocol::P2p(server_peer)))
        .unwrap();
    for n in 0..4 {
        client.behaviour_mut().send_request(
            &server_peer,
            Exchange {
                node_record: vec![n; 16],
            },
        );
    }
    let mut held = Vec::new();
    for _ in 0..4 {
        let Observed::Decoded(peer, value, channel) = next(&mut server, &mut client).await else {
            panic!("expected decoded request");
        };
        assert_eq!(peer, client_peer);
        assert_eq!(value.len(), 16);
        held.push((value, channel));
    }
    assert_eq!(
        held.iter()
            .map(|(value, _)| value[0])
            .collect::<HashSet<_>>(),
        HashSet::from([0, 1, 2, 3])
    );
    // All read_request futures have completed. ResponseChannels are still held.
    assert_eq!(budget.info()["ordinaryActive"], 4);
    assert_eq!(budget.info()["ordinaryBytes"], 4 * 8192);
    assert_eq!(budget.info()["peers"][0]["peerId"], client_peer.to_string());
    let refused = client.behaviour_mut().send_request(
        &server_peer,
        Exchange {
            node_record: vec![4; 16],
        },
    );
    loop {
        match next(&mut server, &mut client).await {
            Observed::Refused(id) => {
                assert_eq!(id, refused);
                break;
            }
            Observed::Cancelled => {}
            _ => panic!("fifth decoded request bypassed shared peer capacity"),
        }
    }
    assert_eq!(budget.info()["ordinaryActive"], 4);
    let (value, channel) = held.pop().unwrap();
    server
        .behaviour_mut()
        .send_response(
            channel,
            Exchange {
                node_record: value.clone(),
            },
        )
        .unwrap();
    loop {
        match next(&mut server, &mut client).await {
            Observed::Response(response) => {
                assert_eq!(response, value);
                break;
            }
            Observed::Cancelled => {}
            _ => panic!("original decoded request failed before response"),
        }
    }
    assert_eq!(budget.info()["ordinaryActive"], 3);
    client.behaviour_mut().send_request(
        &server_peer,
        Exchange {
            node_record: vec![5; 16],
        },
    );
    let Observed::Decoded(peer, value, channel) = next(&mut server, &mut client).await else {
        panic!("released capacity was not reusable");
    };
    assert_eq!(peer, client_peer);
    assert_eq!(value, vec![5; 16]);
    assert_eq!(budget.info()["ordinaryActive"], 4);
    // Cancel a completed decode instead of responding. Three old requests stay held.
    drop(channel);
    while budget.info()["ordinaryActive"] != 3 {
        match next(&mut server, &mut client).await {
            Observed::Cancelled | Observed::Refused(_) => {}
            _ => panic!("unexpected event during response cancellation"),
        }
    }
    assert_eq!(budget.info()["ordinaryBytes"], 3 * 8192);
    assert_eq!(budget.info()["peakOrdinaryActive"], 4);
    // Replacing a swarm drops its workers even if application channels remain held.
    drop(server);
    tokio::time::timeout(Duration::from_secs(2), async {
        while budget.info()["active"] != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(budget.info()["ordinaryBytes"], 0);
    assert_eq!(budget.info()["peers"], json!([]));
    drop((held, client));
}
