//! Independent libp2p Kad responders. No production discovery scheduler or owner lookup API.
use super::*;
use futures::{StreamExt, future::select_all};
use libp2p::{
    StreamProtocol, SwarmBuilder, identify, kad, noise,
    request_response::{self, cbor},
    swarm::{NetworkBehaviour, SwarmEvent},
    yamux,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Exchange {
    #[serde(with = "serde_bytes")]
    node_record: Vec<u8>,
}
#[derive(NetworkBehaviour)]
struct Protocols {
    kad: kad::Behaviour<kad::store::MemoryStore>,
    bootstrap: cbor::Behaviour<Exchange, Exchange>,
    // Nodes admit a Kad server to routing only once Identify lists the protocol.
    identify: identify::Behaviour,
}
pub struct Farm {
    addresses: Vec<String>,
    peers: Vec<String>,
    bad: Arc<AtomicBool>,
    target_calls: Arc<AtomicUsize>,
    finds: Arc<AtomicUsize>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Drop for Farm {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
impl Farm {
    pub fn seed(&self) -> String {
        self.addresses[0].clone()
    }
    pub fn seed_peer(&self) -> String {
        self.peers[0].clone()
    }
    pub fn target(&self) -> String {
        self.peers.last().unwrap().clone()
    }
    pub fn invalid(&self, v: bool) {
        self.bad.store(v, Ordering::SeqCst);
    }
    pub fn target_bootstrap_requests(&self) -> usize {
        self.target_calls.load(Ordering::SeqCst)
    }
    pub fn find_requests(&self) -> usize {
        self.finds.load(Ordering::SeqCst)
    }
    pub fn new(count: usize) -> Self {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let (ready, receive) = std::sync::mpsc::channel();
        let bad = Arc::new(AtomicBool::new(false));
        let target_calls = Arc::new(AtomicUsize::new(0));
        let finds = Arc::new(AtomicUsize::new(0));
        let (bad2, calls2, finds2) = (bad.clone(), target_calls.clone(), finds.clone());
        let thread = thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async move {
            let target=identity::Keypair::generate_ed25519();
            let target_key=kad::KBucketKey::from(target.public().to_peer_id());
            let mut keys:Vec<_>=(1..count).map(|_|identity::Keypair::generate_ed25519()).collect();
            keys.sort_by_key(|k|std::cmp::Reverse(target_key.distance(&kad::KBucketKey::from(k.public().to_peer_id())))); keys.push(target);
            let peers:Vec<_>=keys.iter().map(|k|k.public().to_peer_id()).collect();
            let root=TempDir::new().unwrap(); let mut cores=vec![]; let mut swarms=vec![]; let mut addresses=vec![];
            for (i,key) in keys.into_iter().enumerate() {
                cores.push(agentic_core::AppCore::new(agentic_store::ProfileStore::open(root.path().join(format!("{i}.db")),&[81;32]).unwrap(),agentic_node::NETWORK_DOMAIN).unwrap());
                let mut config=kad::Config::new(StreamProtocol::new("/agentic-internet/kad/1"));
                config.set_kbucket_inserts(kad::BucketInserts::Manual).set_periodic_bootstrap_interval(None);
                let mut kad=kad::Behaviour::with_config(peers[i],kad::store::MemoryStore::new(peers[i]),config); kad.set_mode(Some(kad::Mode::Server));
                let identify=identify::Behaviour::new(identify::Config::new("/agentic-internet/1".into(),key.public()));
                let mut swarm=SwarmBuilder::with_existing_identity(key).with_tokio().with_tcp(Default::default(),noise::Config::new,yamux::Config::default).unwrap()
                    .with_behaviour(|_|Protocols {kad,bootstrap:cbor::Behaviour::new([(StreamProtocol::new("/agentic-internet/bootstrap/1"),request_response::ProtocolSupport::Full)],request_response::Config::default()),identify}).unwrap()
                    .with_swarm_config(|c|c.with_idle_connection_timeout(Duration::from_secs(60))).build();
                swarm.listen_on(TCP.parse().unwrap()).unwrap();
                loop { if let SwarmEvent::NewListenAddr {address,..}=swarm.select_next_some().await { addresses.push(format!("{address}/p2p/{}",peers[i])); break; } }
                swarms.push(swarm);
            }
            for i in 0..count-1 { swarms[i].behaviour_mut().kad.add_address(&peers[i+1],addresses[i+1].parse().unwrap()); }
            let mut ready=Some(ready); let started=Instant::now();
            let mut tick=tokio::time::interval(Duration::from_millis(25)); tokio::pin!(stopped);
            loop {
                tokio::select! {
                    _=&mut stopped=>break,
                    _=tick.tick()=>{
                        if started.elapsed()>Duration::from_secs(2) && swarms.iter().all(|s|s.behaviour().kad.iter_queries().next().is_none()) && let Some(sender)=ready.take() {
                            finds2.store(0,Ordering::SeqCst);
                            sender.send((addresses.clone(),peers.iter().map(ToString::to_string).collect::<Vec<_>>())).unwrap();
                        }
                    },
                    (event,index,_)=select_all(swarms.iter_mut().map(|s|Box::pin(s.select_next_some())))=>{
                        match event {
                            SwarmEvent::Behaviour(ProtocolsEvent::Kad(kad::Event::InboundRequest {request:kad::InboundRequest::FindNode{..}}))=>{finds2.fetch_add(1,Ordering::SeqCst);},
                            SwarmEvent::Behaviour(ProtocolsEvent::Bootstrap(request_response::Event::Message {message:request_response::Message::Request {channel,..},..}))=>{
                                if index==count-1 {calls2.fetch_add(1,Ordering::SeqCst);}
                                let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                let mut wire=cores[index].create_node_record(&peers[index].to_string(),vec![addresses[index].clone()],now).unwrap();
                                if index==count-1 && bad2.load(Ordering::SeqCst) {*wire.last_mut().unwrap()^=1;}
                                swarms[index].behaviour_mut().bootstrap.send_response(channel,Exchange {node_record:wire}).unwrap();
                            },
                            _=>{}
                        }
                    }
                }
            }
        })
        });
        let (addresses, peers) = receive.recv_timeout(Duration::from_secs(20)).unwrap();
        Self {
            addresses,
            peers,
            bad,
            target_calls,
            finds,
            stop: Some(stop),
            thread: Some(thread),
        }
    }
}
