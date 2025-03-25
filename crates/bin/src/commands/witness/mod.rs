use crate::helpers::run_async;
use clap::Subcommand;

pub mod dump;
pub mod rkyv_convert;

#[derive(Debug, Subcommand)]
pub enum WitnessCommands {
    #[command(about = "Dump a witness from reth RPC")]
    Dump(dump::DumpWitnessCommand),
    #[command(about = "Convert a witness json to rkyv")]
    Rkyv(rkyv_convert::RkyvConvertCommand),
}

impl WitnessCommands {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            WitnessCommands::Dump(cmd) => Ok(run_async(cmd.run())?),
            WitnessCommands::Rkyv(cmd) => cmd.run(),
        }
    }
}
