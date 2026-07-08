use lspd::FiberRpcClient;

#[tokio::main]
async fn main() -> lspd::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,lspd=debug".to_string()),
        )
        .with_writer(std::io::stderr)
        .init();

    let url =
        std::env::var("FIBER_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8427".to_string());
    let client = FiberRpcClient::new(url);
    let info = client.node_info().await?;

    println!("{}", serde_json::to_string_pretty(&info)?);
    Ok(())
}
