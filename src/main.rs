mod app;
mod config;
mod platform;
mod terminal;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("bot_terminal=info,teloxide=info")),
        )
        .with_target(false)
        .compact()
        .init();

    let config = config::Config::from_env()?;
    app::run(config).await
}
