//! What a node's signed record lists when its daemon listens on every
//! interface (0.0.0.0). A contact that later reaches the node only through
//! the cached record (a LAN without the Internet, no mDNS) dials exactly these
//! routes, and a record holds eight.
#![allow(clippy::unwrap_used)]
use super::*;
use if_watch::{IfEvent, IpNet};
use std::net::Ipv4Addr;
use tempfile::TempDir;

const TRANSPORTS: [&str; 2] = ["tcp/52000", "udp/52000/quic-v1"];

/// A node whose wildcard listeners bound on each of `interfaces` (address
/// and prefix, as the host reports them). The listeners report first: the
/// transports watch the interfaces on their own, in no fixed order with the
/// node's watcher.
fn wildcard_node(dir: &TempDir, interfaces: &[&str]) -> Runtime {
    let mut node = test_support::runtime(dir.path());
    let networks: Vec<IpNet> = interfaces.iter().map(|i| i.parse().unwrap()).collect();
    for network in &networks {
        for transport in TRANSPORTS {
            node.event(SwarmEvent::NewListenAddr {
                listener_id: ListenerId::next(),
                address: format!("/ip4/{}/{transport}", network.addr())
                    .parse()
                    .unwrap(),
            });
        }
    }
    for network in networks {
        node.interfaces.change(&IfEvent::Up(network));
    }
    node
}

/// The IPs a record lists.
fn ips(routes: &[String]) -> BTreeSet<String> {
    routes
        .iter()
        .map(|route| route.split('/').nth(2).unwrap().to_owned())
        .collect()
}

/// This Mac on 2026-09-30 (`ifconfig`): the Wi-Fi LAN and OrbStack's
/// bridges, three of them on their subnet's network address, where
/// connections are refused (EADDRNOTAVAIL here, another host's network
/// elsewhere). The record lists none of them and leads with the LAN address
/// the default route leaves from, so a contact on the LAN dials it first.
#[tokio::test(flavor = "current_thread")]
async fn a_macs_record_leads_with_its_lan_address_and_lists_no_subnets_network_address() {
    let dir = TempDir::new().unwrap();
    let mut node = wildcard_node(
        &dir,
        &[
            "127.0.0.1/8",
            "192.168.10.41/24", // en0, the LAN
            "192.168.139.3/23", // bridge100
            "192.168.215.0/24", // bridge101
            "172.16.42.0/24",   // bridge102
            "172.31.250.0/24",  // bridge103
        ],
    );
    node.interfaces.primary = Some(Ipv4Addr::new(192, 168, 10, 41));
    let advertised = node.advertised();
    assert!(
        advertised[..2]
            .iter()
            .all(|route| route.starts_with("/ip4/192.168.10.41/")),
        "{advertised:?}"
    );
    let listed = ips(&advertised);
    assert!(listed.contains("192.168.139.3"), "{advertised:?}");
    for dead in ["192.168.215.0", "172.16.42.0", "172.31.250.0"] {
        assert!(!listed.contains(dead), "{dead}: {advertised:?}");
    }
}

/// A Linux laptop with four Docker networks on an iPhone's hotspot, whose
/// addresses come from the same 172.16.0.0/12 as the bridges and sort after
/// them as text. The address the default route leaves from goes first.
#[tokio::test(flavor = "current_thread")]
async fn a_record_leads_with_the_default_routes_address_among_docker_bridges() {
    let dir = TempDir::new().unwrap();
    let mut node = wildcard_node(
        &dir,
        &[
            "127.0.0.1/8",
            "172.20.10.2/28", // wlan0 on the hotspot
            "172.17.0.1/16",  // docker0
            "172.18.0.1/16",  // br-*: compose networks
            "172.19.0.1/16",
            "172.20.0.1/16",
        ],
    );
    node.interfaces.primary = Some(Ipv4Addr::new(172, 20, 10, 2));
    let advertised = node.advertised();
    assert_eq!(advertised.len(), 8, "{advertised:?}");
    assert!(
        advertised[..2]
            .iter()
            .all(|route| route.starts_with("/ip4/172.20.10.2/")),
        "{advertised:?}"
    );
}

/// A Linux desktop with four Docker networks, whose addresses sort before
/// the LAN's as text, and its default route through a tailnet's exit node
/// (a /32 on the tunnel, which may end in .0): the LAN address still makes
/// the eight routes, ahead of the container bridges.
#[tokio::test(flavor = "current_thread")]
async fn a_lan_address_stays_in_the_record_behind_a_vpn_default_route_and_docker_bridges() {
    let dir = TempDir::new().unwrap();
    let mut node = wildcard_node(
        &dir,
        &[
            "127.0.0.1/8",
            "192.168.1.37/24",  // eth0, the LAN
            "100.101.102.0/32", // tailscale0
            "172.17.0.1/16",    // docker0
            "172.18.0.1/16",    // br-*: compose networks
            "172.19.0.1/16",
            "172.20.0.1/16",
        ],
    );
    node.interfaces.primary = Some(Ipv4Addr::new(100, 101, 102, 0));
    let advertised = node.advertised();
    assert_eq!(advertised.len(), 8, "{advertised:?}");
    let position = |ip: &str| {
        advertised
            .iter()
            .position(|route| route.starts_with(&format!("/ip4/{ip}/")))
    };
    let Some(lan) = position("192.168.1.37") else {
        panic!("the LAN address is not listed: {advertised:?}");
    };
    assert!(position("100.101.102.0").is_some(), "{advertised:?}");
    assert!(
        advertised[..lan]
            .iter()
            .all(|route| !route.starts_with("/ip4/172.")),
        "{advertised:?}"
    );
}

/// Only a subnet's network and broadcast addresses are no host's: a DHCP
/// lease on .0 in an office's /22 is dialed as any other. A host without a
/// default route (a static network without a router) keeps the rest.
#[tokio::test(flavor = "current_thread")]
async fn only_a_subnets_network_and_broadcast_addresses_leave_the_record() {
    let dir = TempDir::new().unwrap();
    let node = wildcard_node(
        &dir,
        &[
            "10.20.5.0/22",     // the LAN
            "10.37.129.255/24", // a bridge on its subnet's broadcast address
        ],
    );
    let advertised = node.advertised();
    assert_eq!(
        ips(&advertised),
        ["10.20.5.0"].map(str::to_owned).into(),
        "{advertised:?}"
    );
    assert_eq!(advertised.len(), 2, "{advertised:?}");
}
