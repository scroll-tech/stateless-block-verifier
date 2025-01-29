use clap::Subcommand;

mod dump;
mod rkyv_convert;

#[derive(Subcommand)]
pub enum WitnessCommands {
    #[command(about = "Dump a witness from reth RPC")]
    Dump(dump::DumpWitnessCommand),
    #[command(about = "Convert a witness json to rkyv")]
    Rkyv(rkyv_convert::RkyvConvertCommand),
}

impl WitnessCommands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            WitnessCommands::Dump(cmd) => cmd.run().await.map(|_| ()),
            WitnessCommands::Rkyv(cmd) => cmd.run().await,
        }
    }
}
