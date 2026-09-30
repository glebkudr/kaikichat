#[tokio::main]
async fn main() -> std::process::ExitCode {
    #[cfg(unix)]
    let code = agentic_node::messaging_cli::run().await;
    #[cfg(not(unix))]
    let code = {
        println!(
            "{}",
            serde_json::json!({"error":{"code":"unavailable","message":"Unix IPC required","retryable":true}})
        );
        4
    };
    std::process::ExitCode::from(code)
}
