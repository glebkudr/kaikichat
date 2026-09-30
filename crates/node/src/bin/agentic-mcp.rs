// Executable test scaffold. MCP implementation follows independent test review.
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Scoped Agentic Internet MCP stdio adapter")]
struct Cli {
    #[arg(long)]
    credentials: PathBuf,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    #[cfg(unix)]
    let result = agentic_node::mcp::run(&cli.credentials).await;
    #[cfg(not(unix))]
    let result: agentic_node::Result<()> = Err("Unix IPC required".into());
    if result.is_err() {
        // Never interpolate parser, credential, signing or IPC data into logs.
        eprintln!("agentic-mcp: startup or transport failed");
        std::process::exit(1);
    }
}
