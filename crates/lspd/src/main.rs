use lspd::config::Config;

#[tokio::main]
async fn main() -> lspd::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,lspd=debug".to_string()),
        )
        .with_writer(std::io::stderr)
        .init();

    lspd::lsp_api::serve(Config::from_env()?).await
}
