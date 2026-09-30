//! This host's interfaces, as they rank the routes its signed record lists.
use super::lan_support;
use if_watch::{IfEvent, IpNet, Ipv4Net, tokio::IfWatcher};
use libp2p::{Multiaddr, multiaddr::Protocol};
use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, UdpSocket},
    task::Poll,
};

#[derive(Default)]
pub(super) struct Interfaces {
    /// This host's IPv4 subnets, one per interface address.
    networks: Vec<Ipv4Net>,
    /// The address this host's default route leaves from, when it has one.
    pub(super) primary: Option<Ipv4Addr>,
}
impl Interfaces {
    /// Whether this host's IPv4 subnets changed.
    pub(super) fn change(&mut self, event: &IfEvent) -> bool {
        match event {
            IfEvent::Up(IpNet::V4(network)) if !self.networks.contains(network) => {
                self.networks.push(*network);
                true
            }
            IfEvent::Down(IpNet::V4(network)) if self.networks.contains(network) => {
                self.networks.retain(|n| n != network);
                true
            }
            _ => false,
        }
    }
    /// Listener routes, the likeliest to connect first; see [`Rank`]. A host
    /// with many bridges keeps its public and LAN addresses among the eight
    /// a record lists.
    pub(super) fn by_reach(&self, listeners: &BTreeSet<String>) -> Vec<String> {
        let mut routes: Vec<_> = listeners
            .iter()
            .filter_map(|route| Some((self.rank(route)?, route.clone())))
            .collect();
        routes.sort_by_key(|(rank, _)| *rank);
        routes.into_iter().map(|(_, route)| route).collect()
    }
    /// `None` for a subnet's network or broadcast address: it is no host's,
    /// and a connection to it is refused (EADDRNOTAVAIL on this host, another
    /// host's network elsewhere). OrbStack numbers its bridges so.
    fn rank(&self, route: &str) -> Option<Rank> {
        let Ok(address) = route.parse::<Multiaddr>() else {
            return Some(Rank::default());
        };
        let reach = reach(&address);
        let Some(Protocol::Ip4(ip)) = address.iter().next() else {
            return Some(Rank {
                reach,
                ..Rank::default()
            });
        };
        if lan_support::reach(&self.networks, &address) == lan_support::Reach::Never {
            return None;
        }
        Some(Rank {
            reach,
            secondary: self.primary != Some(ip),
            // Docker's default pools and OrbStack number their bridges from
            // 172.16.0.0/12.
            bridge: ip.octets()[0] == 172 && ip.octets()[1] & 0xf0 == 16,
        })
    }
}

/// Where a route stands among a record's, smallest first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    /// How far the route is reached from: 0 anywhere, 1 on a private
    /// network, 2 on this host or its links only.
    reach: u8,
    /// Not the address the default route leaves from, which the other
    /// hosts of a LAN reach.
    secondary: bool,
    /// Likely a container bridge, reached from this host only.
    bridge: bool,
}
impl Default for Rank {
    fn default() -> Self {
        Self {
            reach: 2,
            secondary: true,
            bridge: false,
        }
    }
}

/// How far a route is reached from; see [`Rank::reach`].
fn reach(address: &Multiaddr) -> u8 {
    match address.iter().next() {
        Some(Protocol::Ip4(ip)) if ip.is_loopback() || ip.is_link_local() => 2,
        // RFC 1918, and carrier-grade NAT's 100.64.0.0/10.
        Some(Protocol::Ip4(ip))
            if ip.is_private() || (ip.octets()[0] == 100 && ip.octets()[1] & 0xc0 == 64) =>
        {
            1
        }
        Some(Protocol::Ip4(_)) => 0,
        Some(Protocol::Ip6(ip)) if ip.is_loopback() || ip.is_unicast_link_local() => 2,
        Some(Protocol::Ip6(ip)) if ip.is_unique_local() => 1,
        Some(Protocol::Ip6(_)) => 0,
        _ => 2,
    }
}

/// The address this host's default route leaves from. `connect` on a UDP
/// socket only picks the route: no packet leaves.
pub(super) fn default_route() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    // TEST-NET-1: an address no host has, reached through the default route.
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_unspecified() => Some(ip),
        _ => None,
    }
}

/// Watches this host's interfaces until the watcher fails.
pub(super) struct Watcher(Option<IfWatcher>);
impl Watcher {
    pub(super) fn new() -> Self {
        Self(IfWatcher::new().ok())
    }
    /// The next change; none once the watcher failed.
    pub(super) async fn next(&mut self) -> IfEvent {
        std::future::poll_fn(|cx| {
            while let Some(watcher) = &mut self.0 {
                match watcher.poll_if_event(cx) {
                    Poll::Ready(Ok(event)) => return Poll::Ready(event),
                    Poll::Ready(Err(_)) => self.0 = None,
                    Poll::Pending => return Poll::Pending,
                }
            }
            Poll::Pending
        })
        .await
    }
}
