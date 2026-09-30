//! Service-owned callback lifetime, independent of other connections to the same peer.
use super::*;

struct Callback {
    peer: PeerId,
    deadline: Option<Instant>,
}
pub(in crate::runtime) struct CallbackLeases {
    lifetime: Duration,
    connections: HashMap<ConnectionId, Callback>,
}
impl CallbackLeases {
    pub(in crate::runtime) fn new(lifetime: Duration) -> Self {
        Self {
            lifetime,
            connections: HashMap::new(),
        }
    }
    pub(in crate::runtime) fn dial_started(&mut self, peer: PeerId, id: ConnectionId) {
        self.connections.insert(
            id,
            Callback {
                peer,
                deadline: None,
            },
        );
    }
    pub(in crate::runtime) fn established(&mut self, id: ConnectionId, now: Instant) {
        if let Some(callback) = self.connections.get_mut(&id) {
            callback.deadline = Some(now + self.lifetime);
        }
    }
    pub(in crate::runtime) fn closed(&mut self, id: ConnectionId) {
        self.connections.remove(&id);
    }
    pub(in crate::runtime) fn expired(&mut self, now: Instant) -> Vec<(PeerId, ConnectionId)> {
        let mut expired = Vec::new();
        self.connections.retain(|id, callback| {
            if callback.deadline.is_some_and(|deadline| now >= deadline) {
                expired.push((callback.peer, *id));
                false
            } else {
                true
            }
        });
        expired
    }
}
