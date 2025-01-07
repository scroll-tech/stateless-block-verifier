//! Stateless Block Verifier

#[macro_use]
extern crate sbv;

use clap::Parser;

#[cfg(feature = "dev")]
use tracing_subscriber::EnvFilter;

mod commands;

mod utils;

#[derive(Parser)]
#[command(version, about = "Stateless Block Verifier")]
struct Cli {
    #[command(subcommand)]
    commands: commands::Commands,
    /// Start metrics server
    #[cfg(feature = "metrics")]
    #[arg(long)]
    metrics: bool,
    /// Metrics server address
    #[cfg(feature = "metrics")]
    #[arg(long, default_value = "127.0.0.1:9090")]
    metrics_addr: std::net::SocketAddr,
}

fn main() -> anyhow::Result<()> {
    // Install the tracing subscriber that will listen for events and filters. We try to use the
    // `RUST_LOG` environment variable and default to RUST_LOG=info if unset.
    #[cfg(feature = "dev")]
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cmd = Cli::parse();

    #[cfg(feature = "metrics")]
    if cmd.metrics {
        sbv::helpers::metrics::start_metrics_server(cmd.metrics_addr);
    }

    cmd.commands.run()?;

    Ok(())
}
