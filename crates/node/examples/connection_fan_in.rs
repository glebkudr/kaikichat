//! Test-only independent ordinary peers. No profile, selected key, or authority API.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use futures::{
    StreamExt,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
};
use libp2p::{
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder, identity, noise,
    request_response::{self, ProtocolSupport, cbor},
    swarm::{NetworkBehaviour, SwarmEvent},
    yamux,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::{BTreeSet, HashMap},
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{sync::watch, task::JoinSet};

#[derive(Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Control {
    connections: usize,
    disabled: BTreeSet<usize>,
    retry: u64,
    #[serde(default)]
    load: Option<Load>,
}
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Load {
    generation: u64,
    mode: String,
    frame: serde_json::Value,
}
#[derive(Default)]
struct Counters {
    started: AtomicU64,
    partial_writes: AtomicU64,
    complete_writes: AtomicU64,
    responses: AtomicU64,
    unverified: AtomicU64,
    enqueued: AtomicU64,
    failures: AtomicU64,
    early_failures: AtomicU64,
}
impl Counters {
    fn snapshot(&self, index: usize) -> serde_json::Value {
        json!({"index":index,"started":self.started.load(Ordering::Relaxed),
            "partialWrites":self.partial_writes.load(Ordering::Relaxed),
            "completeWrites":self.complete_writes.load(Ordering::Relaxed),
            "responses":self.responses.load(Ordering::Relaxed),
            "unverified":self.unverified.load(Ordering::Relaxed),
            "enqueued":self.enqueued.load(Ordering::Relaxed),
            "failures":self.failures.load(Ordering::Relaxed),
            "earlyFailures":self.early_failures.load(Ordering::Relaxed)})
    }
}
#[derive(Debug)]
struct RawRequest {
    value: serde_json::Value,
    generation: u64,
    hold: bool,
}
#[derive(Clone)]
struct RawCodec {
    controls: watch::Receiver<Control>,
    counters: Arc<Counters>,
}
#[async_trait::async_trait]
impl request_response::Codec for RawCodec {
    type Protocol = StreamProtocol;
    type Request = RawRequest;
    type Response = serde_json::Value;
    async fn read_request<T: AsyncRead + Unpin + Send>(
        &mut self,
        _: &StreamProtocol,
        _: &mut T,
    ) -> io::Result<RawRequest> {
        Err(io::ErrorKind::PermissionDenied.into())
    }
    async fn read_response<T: AsyncRead + Unpin + Send>(
        &mut self,
        p: &StreamProtocol,
        io: &mut T,
    ) -> io::Result<serde_json::Value> {
        let mut codec = cbor::codec::Codec::<serde_json::Value, serde_json::Value>::default()
            .set_response_size_maximum(8192);
        codec.read_response(p, io).await
    }
    async fn write_request<T: AsyncWrite + Unpin + Send>(
        &mut self,
        p: &StreamProtocol,
        io: &mut T,
        request: RawRequest,
    ) -> io::Result<()> {
        let mut codec = cbor::codec::Codec::<serde_json::Value, serde_json::Value>::default();
        let mut encoded = futures::io::Cursor::new(Vec::new());
        codec.write_request(p, &mut encoded, request.value).await?;
        let bytes = encoded.into_inner();
        assert!(bytes.len() > 1024 && bytes.len() <= 34816);
        let mut sent = 0;
        if request.hold {
            // A real incomplete CBOR stream: periodic writes notice a remote reset,
            // while the final byte and EOF remain withheld until control changes.
            io.write_all(&bytes[..1]).await?;
            io.flush().await?;
            sent = 1;
            self.counters.partial_writes.fetch_add(1, Ordering::Relaxed);
            loop {
                let holding = self
                    .controls
                    .borrow()
                    .load
                    .as_ref()
                    .is_some_and(|v| v.mode == "hold" && v.generation == request.generation);
                if !holding {
                    break;
                }
                tokio::select! {
                    changed = self.controls.changed() => { if changed.is_err() { return Err(io::ErrorKind::Interrupted.into()); } }
                    _ = tokio::time::sleep(Duration::from_millis(20)) => {
                        if sent < bytes.len() - 1 {
                            io.write_all(&bytes[sent..sent+1]).await?;
                            io.flush().await?;
                            sent += 1;
                        }
                    }
                }
            }
        }
        io.write_all(&bytes[sent..]).await?;
        self.counters
            .complete_writes
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    async fn write_response<T: AsyncWrite + Unpin + Send>(
        &mut self,
        _: &StreamProtocol,
        _: &mut T,
        _: serde_json::Value,
    ) -> io::Result<()> {
        Err(io::ErrorKind::PermissionDenied.into())
    }
}
#[derive(NetworkBehaviour)]
struct Network {
    frames: request_response::Behaviour<RawCodec>,
    bootstrap: request_response::Behaviour<RawCodec>,
}
fn traffic(codec: RawCodec, protocol: &'static str) -> request_response::Behaviour<RawCodec> {
    request_response::Behaviour::with_codec(
        codec,
        [(StreamProtocol::new(protocol), ProtocolSupport::Outbound)],
        request_response::Config::default()
            .with_request_timeout(Duration::from_secs(7))
            .with_max_concurrent_streams(2),
    )
}

async fn client(
    index: usize,
    target: Multiaddr,
    mut controls: watch::Receiver<Control>,
    counters: Arc<Counters>,
) {
    let key = identity::Keypair::generate_ed25519();
    let peer = key.public().to_peer_id();
    let target_peer: PeerId = match target.iter().last() {
        Some(libp2p::multiaddr::Protocol::P2p(peer)) => peer,
        _ => panic!("fixture target needs an authenticated peer ID"),
    };
    let mut swarm = SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_behaviour(|_| {
            let codec = RawCodec {
                controls: controls.clone(),
                counters: counters.clone(),
            };
            Network {
                frames: traffic(codec.clone(), "/agentic-internet/mailbox/1"),
                bootstrap: traffic(codec, "/agentic-internet/bootstrap/1"),
            }
        })
        .unwrap()
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
        .with_connection_timeout(Duration::from_secs(6))
        .build();
    println!("{}", json!({"created":index,"peerId":peer.to_string()}));
    swarm.dial(target.clone()).unwrap();
    let mut retry = controls.borrow().retry;
    let mut requests = HashMap::new();
    let mut load_tick = tokio::time::interval(Duration::from_millis(200));
    load_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut fast_load = false;
    loop {
        tokio::select! {
            changed=controls.changed()=>{
                if changed.is_err(){break;}
                let control=controls.borrow().clone();
                let fast = control.load.as_ref().is_some_and(|load| load.mode == "flood");
                if fast != fast_load {
                    // Completed traffic must offer enough work to exercise the
                    // rate ceiling as well as the simultaneous-reader ceiling.
                    // Stagger peers instead of repeatedly synchronizing64 bursts.
                    let period = if fast { 20 } else { 200 };
                    load_tick = tokio::time::interval_at(
                        tokio::time::Instant::now() + Duration::from_micros(index as u64 * period * 1000 / 65),
                        Duration::from_millis(period),
                    );
                    load_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    fast_load = fast;
                }
                if control.disabled.contains(&index) {let _=swarm.disconnect_peer_id(target_peer);}
                else if control.retry!=retry && !swarm.is_connected(&target_peer) {let _=swarm.dial(target.clone());}
                retry=control.retry;
            }
            _=load_tick.tick()=> {
                let control = controls.borrow().clone();
                if requests.is_empty() && !control.disabled.contains(&index) && swarm.is_connected(&target_peer)
                    && let Some(load) = control.load {
                    let bootstrap = load.mode == "hold" && index % 2 == 1;
                    let value = if bootstrap { json!({"nodeRecord":vec![0u8; 2048]}) } else { load.frame };
                    let request = RawRequest { value, generation:load.generation, hold:load.mode == "hold" };
                    let id = if bootstrap { swarm.behaviour_mut().bootstrap.send_request(&target_peer, request) }
                        else { swarm.behaviour_mut().frames.send_request(&target_peer, request) };
                    requests.insert((bootstrap,id), std::time::Instant::now());
                    counters.started.fetch_add(1, Ordering::Relaxed);
                }
            }
            event=swarm.select_next_some()=>{
                let event = match event {
                    SwarmEvent::Behaviour(NetworkEvent::Frames(event)) => { received(false, event, &mut requests, &counters); continue; }
                    SwarmEvent::Behaviour(NetworkEvent::Bootstrap(event)) => { received(true, event, &mut requests, &counters); continue; }
                    other => other,
                };
                let kind=match event {
                    SwarmEvent::ConnectionEstablished{..}=>Some("connected"),
                    SwarmEvent::ConnectionClosed{..}=>Some("closed"),
                    SwarmEvent::OutgoingConnectionError{..}=>Some("dialFailed"),
                    _=>None,
                };
                if let Some(kind)=kind {println!("{}",json!({"index":index,"peerId":peer.to_string(),"event":kind}));}
            }
        }
    }
}

fn received(
    bootstrap: bool,
    event: request_response::Event<RawRequest, serde_json::Value>,
    requests: &mut HashMap<(bool, request_response::OutboundRequestId), std::time::Instant>,
    counters: &Counters,
) {
    match event {
        request_response::Event::Message {
            message:
                request_response::Message::Response {
                    request_id,
                    response,
                },
            ..
        } => {
            assert!(requests.remove(&(bootstrap, request_id)).is_some());
            counters.responses.fetch_add(1, Ordering::Relaxed);
            if response.get("status").and_then(serde_json::Value::as_str) == Some("enqueued") {
                counters.enqueued.fetch_add(1, Ordering::Relaxed);
            }
            if response.get("status").and_then(serde_json::Value::as_str) == Some("unverified_peer")
            {
                counters.unverified.fetch_add(1, Ordering::Relaxed);
            }
        }
        request_response::Event::OutboundFailure { request_id, .. } => {
            let started = requests.remove(&(bootstrap, request_id)).unwrap();
            if started.elapsed() < Duration::from_secs(3) {
                counters.early_failures.fetch_add(1, Ordering::Relaxed);
            }
            counters.failures.fetch_add(1, Ordering::Relaxed);
        }
        _ => {}
    }
}

#[tokio::main(worker_threads = 2)]
async fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "target and control path");
    let target: Multiaddr = args[1].parse().unwrap();
    let path = PathBuf::from(&args[2]);
    let (sender, receiver) = watch::channel(Control::default());
    let mut jobs = JoinSet::new();
    let mut created = 0;
    let mut counters = Vec::new();
    let mut stats_due = tokio::time::Instant::now();
    let mut timer = tokio::time::interval(Duration::from_millis(50));
    #[cfg(unix)]
    let mut terminate =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    let shutdown = async {
        #[cfg(unix)]
        tokio::select! {_=terminate.recv()=>{},_=tokio::signal::ctrl_c()=>{}}
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    };
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _=&mut shutdown=>break,
            result=jobs.join_next(),if !jobs.is_empty()=>{result.unwrap().unwrap();panic!("ordinary peer task stopped unexpectedly");}
            _=timer.tick()=>{
                let control: Control=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                assert!((created..=65).contains(&control.connections));
                assert!(control.disabled.iter().all(|i|*i<control.connections));
                if let Some(load) = &control.load {
                    assert!(load.generation > 0 && ["hold", "flood"].contains(&load.mode.as_str()));
                    assert!(load.frame.is_object());
                }
                if *sender.borrow()!=control {sender.send_replace(control.clone());}
                while created<control.connections {
                    let count = Arc::new(Counters::default());
                    jobs.spawn(client(created,target.clone(),receiver.clone(),count.clone()));
                    counters.push(count);created+=1;
                }
                if control.load.is_some() && tokio::time::Instant::now() >= stats_due {
                    stats_due = tokio::time::Instant::now() + Duration::from_millis(500);
                    println!("{}",json!({"loadStats": counters.iter().enumerate().map(|(i,c)|c.snapshot(i)).collect::<Vec<_>>()}));
                }
            }
        }
    }
    drop(sender);
    drop(receiver);
    // Closing the watch channel stops every client and drops every real socket.
    while let Some(result) = jobs.join_next().await {
        result.unwrap();
    }
}
