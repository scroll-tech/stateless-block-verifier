use crate::helpers::run_async;
use clap::Subcommand;

pub mod dump;

#[derive(Debug, Subcommand)]
pub enum WitnessCommands {
    #[command(about = "Dump a witness from reth RPC")]
    Dump(dump::DumpWitnessCommand),
}

impl WitnessCommands {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            WitnessCommands::Dump(cmd) => Ok(run_async(cmd.run())?),
        }
    }
}
