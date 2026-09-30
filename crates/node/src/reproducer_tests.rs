//! Managed-time one-process reproducer (V1-C05): several logical nodes share a
//! process, each a real Runtime on a real SQLCipher store, driven by one
//! installed clock/RNG controller. The driver pumps production maintenance,
//! drains real swarm events in a fixed node order, and advances the shared
//! clock only to the nearest declared `next_due`. Every step is appended to an
//! event trace whose sha256 digest must be identical across runs.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::runtime::test_support;
use futures::FutureExt;
use tempfile::TempDir;

#[path = "reproducer_mailbox_swarm_tests.rs"]
mod mailbox_swarm;

struct Rig {
    clock: clock::Virtual,
    _rng: random::Deterministic,
    nodes: Vec<Runtime>,
    _dirs: Vec<TempDir>,
    trace: Vec<String>,
    /// The chain every node reads books and grant rules from.
    chain: test_support::FakeChain,
    /// Treat a due deadline that a pump serviced without progress like the
    /// production loop does — serviced, time moves on to the next deadline —
    /// instead of stopping. Scenarios that end on a terminal condition, not
    /// on exhaustion, opt in.
    pass_serviced: bool,
    /// Nodes switched off: neither pumped nor polled, their deadlines and
    /// requests left alone, like a stopped daemon.
    paused: std::collections::BTreeSet<usize>,
}

fn kind(event: &SwarmEvent<NetworkEvent>) -> &'static str {
    match event {
        SwarmEvent::ConnectionEstablished { .. } => "connection_established",
        SwarmEvent::ConnectionClosed { .. } => "connection_closed",
        SwarmEvent::NewListenAddr { .. } => "new_listen_addr",
        SwarmEvent::ExpiredListenAddr { .. } => "expired_listen_addr",
        SwarmEvent::ListenerClosed { .. } => "listener_closed",
        SwarmEvent::ListenerError { .. } => "listener_error",
        SwarmEvent::Dialing { .. } => "dialing",
        SwarmEvent::OutgoingConnectionError { .. } => "outgoing_connection_error",
        SwarmEvent::IncomingConnection { .. } => "incoming_connection",
        SwarmEvent::IncomingConnectionError { .. } => "incoming_connection_error",
        SwarmEvent::Behaviour(_) => "behaviour",
        _ => "other",
    }
}

impl Rig {
    fn new(count: usize, wall: u64) -> Self {
        let clock = clock::Virtual::install(wall);
        let rng = random::Deterministic::install([7; 32]);
        let mut nodes = Vec::new();
        let mut dirs = Vec::new();
        let chain = test_support::FakeChain::default();
        for node in 0..count {
            let dir = TempDir::new().unwrap();
            let mut runtime = test_support::runtime(dir.path());
            runtime.set_chain(chain.for_node(node));
            nodes.push(runtime);
            dirs.push(dir);
        }
        Self {
            clock,
            _rng: rng,
            nodes,
            _dirs: dirs,
            trace: vec![format!("install:wall={wall}")],
            chain,
            pass_serviced: false,
            paused: std::collections::BTreeSet::new(),
        }
    }

    async fn connect(&mut self, local: usize, remote: usize) {
        assert!(local != remote);
        let (local_node, remote_node) = if local < remote {
            let (left, right) = self.nodes.split_at_mut(remote);
            (&mut left[local], &mut right[0])
        } else {
            let (left, right) = self.nodes.split_at_mut(local);
            (&mut right[0], &mut left[remote])
        };
        test_support::connect(local_node, remote_node, false).await;
        self.trace.push(format!("connect:{local}->{remote}"));
    }

    fn drain(&mut self) -> bool {
        let mut progressed = false;
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if self.paused.contains(&i) {
                continue;
            }
            loop {
                let event = node.swarm.select_next_some().now_or_never();
                let Some(event) = event else { break };
                progressed = true;
                self.trace.push(format!("n{i}:{}", kind(&event)));
                node.event(event);
            }
        }
        progressed
    }

    /// Pump production maintenance and drain real swarm events in a fixed node
    /// order. A refused dead-leg dial surfaces as an async IO error, so a quiet
    /// pass gets one short real-time grace before the clock may advance.
    async fn step(&mut self) -> bool {
        let mut progressed = false;
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if !self.paused.contains(&i) {
                node.pump();
            }
        }
        progressed |= self.drain();
        if !progressed {
            tokio::time::sleep(Duration::from_millis(20)).await;
            progressed |= self.drain();
        }
        progressed
    }

    fn next_due(&self) -> Option<Instant> {
        self.running().map(|(_, n)| n.next_due()).min()
    }

    fn running(&self) -> impl Iterator<Item = (usize, &Runtime)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.paused.contains(i))
    }

    /// Sent requests, opening dials and chain reads: the driver waits for the
    /// real response or the real IO failure instead of letting virtual time
    /// pass them by.
    fn in_flight(&self) -> bool {
        self.running().any(|(i, n)| {
            n.mailbox_client.in_flight() > 0
                // Reads a scenario holds on purpose would stall every step.
                || (n.chain_in_flight() > 0 && !self.chain.is_held(i))
        })
    }

    /// Advance the shared clock to the earliest declared deadline, then pump —
    /// but only when nothing is in flight. Real responses and real IO failures
    /// are awaited in real time, so the number of trace events depends on the
    /// scenario, not on wall timing. Bounded steps make a runaway driver a test
    /// failure, not a hang; `done` is the scenario's terminal condition.
    async fn run_until(&mut self, max_steps: usize, done: impl Fn(&Self) -> bool) {
        // In-flight work gets exactly one real-time grace per stall: a live
        // peer answers within milliseconds, while a dead leg falls back to its
        // declared virtual dial/request deadline once the grace is spent.
        let mut grace_used = false;
        for _ in 0..max_steps {
            if done(self) {
                self.trace.push("done".into());
                return;
            }
            if self.step().await {
                grace_used = false;
                continue;
            }
            if self.in_flight() && !grace_used {
                let deadline = Instant::now() + Duration::from_secs(1);
                while self.in_flight() && Instant::now() < deadline {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    if self.drain() {
                        break;
                    }
                }
                grace_used = true;
                continue;
            }
            // A chain answer that came in after the pump is applied by the
            // next one, before any time passes.
            if self.running().any(|(_, n)| n.chain_answers_waiting()) {
                continue;
            }
            let Some(mut due) = self.next_due() else {
                break;
            };
            if due <= self.clock.instant() && self.pass_serviced {
                let now = self.clock.instant();
                let Some(next) = self
                    .running()
                    .filter_map(|(_, n)| n.next_deadline_after(now))
                    .min()
                else {
                    break;
                };
                due = next;
            }
            if due <= self.clock.instant() {
                // A due item in the past produced no progress: the work itself
                // is genuinely exhausted or stalled — record and stop.
                self.trace.push("quiesce".into());
                break;
            }
            self.trace.push(format!(
                "advance:{}",
                due.duration_since(self.clock.instant()).as_millis()
            ));
            self.clock.advance_to(due);
        }
    }
}
