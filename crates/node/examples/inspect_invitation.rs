//! Test fixture: an independent profile verifies the actual signed invitation.
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
fn main() -> agentic_node::Result<()> {
    let mut input = String::new();
    std::io::stdin().take(140_000).read_to_string(&mut input)?;
    let invitation: String = serde_json::from_str(&input)?;
    let temporary = tempfile::tempdir()?;
    let store = agentic_store::ProfileStore::open(temporary.path().join("profile.db"), &[19; 32])?;
    let core = agentic_core::AppCore::new(store, agentic_node::NETWORK_DOMAIN)?;
    let addresses = core.invitation_addresses(
        &invitation,
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    )?;
    println!("{}", serde_json::to_string(&addresses)?);
    Ok(())
}
