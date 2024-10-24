//! This is a simple example of how to use the stateful executor to verify the state transition of the L2 chain.

use alloy::providers::ProviderBuilder;
use clap::Parser;
use stateful_block_verifier::StatefulBlockExecutor;
use std::path::PathBuf;
use url::Url;

#[cfg(feature = "dev")]
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
struct Cli {
    /// RPC URL
    #[arg(short, long, default_value = "http://localhost:8545")]
    url: Url,
    /// Path to the sled database
    #[arg(short, long)]
    db: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "dev")]
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cmd = Cli::parse();

    let provider = ProviderBuilder::new().on_http(cmd.url);
    let mut executor = StatefulBlockExecutor::new(sled::open(cmd.db)?, provider.clone()).await?;

    tokio::select! {
        _ = executor.run() => {}
        _ = tokio::signal::ctrl_c() => {}
    }

    Ok(())
}
