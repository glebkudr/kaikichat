use clap::{Parser, Subcommand};
use std::{
    io::{BufRead, Read},
    path::PathBuf,
};
use zeroize::Zeroizing;
#[derive(Parser)]
#[command(name = env!("CARGO_BIN_NAME"), version, about = "Agentic Internet independent node")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Serve {
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        ipc: PathBuf,
        #[arg(long, default_value = "/ip4/0.0.0.0/udp/0/quic-v1")]
        listen: Vec<String>,
        /// An IP address other nodes reach this one at, such as a holder's
        /// public `/ip4/IP/udp/PORT/quic-v1` (at most 4): told them instead of
        /// every address the listeners bind.
        #[arg(long = "public-address", conflicts_with = "relay_only")]
        public_addresses: Vec<String>,
        /// Independent IP/PeerID discovery hints; at most4, with an optional relay hop.
        #[arg(long = "bootstrap")]
        bootstrap: Vec<String>,
        /// Discover peers with multicast DNS on the local network. Suppressed by relay-only.
        #[arg(long)]
        lan_discovery: bool,
        /// Serve bounded DHT queries for other peers. Independent of validator roles; suppressed by relay-only.
        #[arg(long)]
        dht_server: bool,
        /// Direct IP routes to independent Circuit Relay v2 providers (at most4).
        #[arg(long = "relay")]
        relays: Vec<String>,
        /// Advertise and dial application peers only through configured relays.
        #[arg(long, requires = "relays")]
        relay_only: bool,
        /// Opt in to forwarding bounded encrypted circuits for other peers.
        #[arg(long)]
        relay_server: bool,
        #[arg(long, default_value_t = 16, value_parser = clap::value_parser!(u8).range(1..=16))]
        relay_capacity: u8,
        #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u16).range(4..=3600))]
        relay_reservation_seconds: u16,
        #[arg(long, default_value_t = 120, value_parser = clap::value_parser!(u16).range(2..=120))]
        relay_circuit_seconds: u16,
        /// Direct IP routes to independent AutoNAT verification providers (at most4).
        #[arg(long = "autonat-peer")]
        autonat_peers: Vec<String>,
        /// Opt in to bounded, fresh inbound reachability probes for other peers.
        #[arg(long)]
        autonat_server: bool,
        /// Permit private/LAN clients only when explicitly providing AutoNAT service.
        #[arg(long, requires = "autonat_server")]
        autonat_allow_local: bool,
        #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u16).range(10..=900))]
        autonat_probe_seconds: u16,
        /// JSON-RPC endpoint (the operator's choice) to read bought books and grant rules from.
        /// Give it with --chain-id, --book-shop and --grant-issuer, or none of them.
        #[arg(long)]
        chain_rpc: Option<String>,
        #[arg(long)]
        chain_id: Option<u64>,
        /// `BookShop` contract address, 0x-hex.
        #[arg(long)]
        book_shop: Option<String>,
        /// `GrantIssuer` contract address, 0x-hex.
        #[arg(long)]
        grant_issuer: Option<String>,
        /// `NodeRegistry` contract address, 0x-hex: the units of the directory.
        #[arg(long)]
        registry: Option<String>,
        /// Blocks below the head a purchase or rule must be to count (at least 1).
        #[arg(long, default_value_t = 6)]
        chain_confirmations: u64,
        /// `OperatorPool` contract address, 0x-hex: a holder draws its
        /// prizes there and its operator withdraws them (with the chain flags).
        #[arg(long)]
        operator_pool: Option<String>,
        /// Identity server that grants coins to Google-verified people (`coins claim`).
        #[arg(long)]
        identity_server: Option<String>,
        /// The discovery service the owner's CLI and window use.
        #[arg(long)]
        directory: Option<String>,
        /// The key the discovery service signs bindings with (hex).
        #[arg(long, requires = "directory")]
        directory_key: Option<String>,
        #[arg(long, required = true)]
        secrets_stdin: bool,
    },
}
#[tokio::main]
async fn main() {
    if let Err(error) = start().await {
        eprintln!("kaiki-agentic-node: {error}");
        std::process::exit(1);
    }
}
async fn start() -> agentic_node::Result<()> {
    let Cli {
        command:
            Command::Serve {
                profile,
                ipc,
                listen,
                public_addresses,
                bootstrap,
                lan_discovery,
                dht_server,
                relays,
                relay_only,
                relay_server,
                relay_capacity,
                relay_reservation_seconds,
                relay_circuit_seconds,
                autonat_peers,
                autonat_server,
                autonat_allow_local,
                autonat_probe_seconds,
                chain_rpc,
                chain_id,
                book_shop,
                grant_issuer,
                registry,
                chain_confirmations,
                operator_pool,
                identity_server,
                directory,
                directory_key,
                secrets_stdin: _,
            },
    } = Cli::parse();
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .lock()
        .take(4097)
        .read_until(b'\n', &mut bytes)?;
    let secrets = agentic_node::Bootstrap::decode(&bytes)?;
    drop(bytes);
    #[cfg(unix)]
    {
        agentic_node::run(agentic_node::NodeConfig {
            profile,
            ipc,
            listen,
            public_addresses,
            bootstrap,
            lan_discovery,
            dht_server,
            relays,
            relay_only,
            relay_server,
            relay_capacity,
            relay_reservation_seconds,
            relay_circuit_seconds,
            autonat_peers,
            autonat_server,
            autonat_allow_local,
            autonat_probe_seconds,
            chain_rpc,
            chain_id,
            book_shop,
            grant_issuer,
            registry,
            chain_confirmations,
            operator_pool,
            identity_server,
            directory,
            directory_key,
            secrets,
        })
        .await
    }
    #[cfg(not(unix))]
    {
        let _ = (
            profile,
            ipc,
            listen,
            public_addresses,
            bootstrap,
            lan_discovery,
            dht_server,
            secrets,
            relays,
            relay_only,
            relay_server,
            relay_capacity,
            relay_reservation_seconds,
            relay_circuit_seconds,
            autonat_peers,
            autonat_server,
            autonat_allow_local,
            autonat_probe_seconds,
            chain_rpc,
            chain_id,
            book_shop,
            grant_issuer,
            registry,
            chain_confirmations,
            operator_pool,
            identity_server,
            directory,
            directory_key,
        );
        Err("this build requires Unix local IPC; Windows pipe support is pending".into())
    }
}
