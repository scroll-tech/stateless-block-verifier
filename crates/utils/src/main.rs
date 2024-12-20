//! Command line utility.
use clap::Parser;

mod commands;
mod helpers;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    commands: commands::Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmd = Cli::parse();

    cmd.commands.run().await?;

    Ok(())
}
