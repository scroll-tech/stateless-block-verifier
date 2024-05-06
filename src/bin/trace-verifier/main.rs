#[macro_use]
extern crate log;

use clap::Parser;

mod commands;
mod utils;

#[derive(Parser)]
#[command(version, about = "Stateless Block Verifier")]
struct Cli {
    #[command(subcommand)]
    commands: commands::Commands,
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
    cmd.commands.run(cmd.disable_checks).await?;
    Ok(())
}
