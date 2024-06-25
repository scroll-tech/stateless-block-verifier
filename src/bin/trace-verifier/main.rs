#![feature(lazy_cell)]
#![feature(slice_group_by)]
#[macro_use]
extern crate log;

use clap::Parser;
use stateless_block_verifier::HardforkConfig;

mod commands;
mod utils;

#[derive(Parser)]
#[command(version, about = "Stateless Block Verifier")]
struct Cli {
    #[command(subcommand)]
    commands: commands::Commands,
    /// Curie block number, defaults to be determined by chain id
    #[arg(short, long)]
    curie_block: Option<u64>,
    /// Disable additional checks
    #[arg(short = 'k', long)]
    disable_checks: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    let cmd = Cli::parse();

    let get_fork_config = |chain_id: u64| {
        let mut config = HardforkConfig::default_from_chain_id(chain_id);
        if let Some(curie_block) = cmd.curie_block {
            config.set_curie_block(curie_block);
        }
        config
    };

    cmd.commands
        .run(get_fork_config, cmd.disable_checks)
        .await?;
    Ok(())
}
