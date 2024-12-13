use clap::Subcommand;

mod create;
mod dump;

#[derive(Subcommand)]
pub enum WitnessCommands {
    #[command(about = "Create a new witness")]
    Create(create::CreateWitnessCommand),
    #[command(about = "Dump a witness")]
    Dump(dump::DumpWitnessCommand),
}

impl WitnessCommands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            WitnessCommands::Create(cmd) => cmd.run().await,
            WitnessCommands::Dump(cmd) => cmd.run().await,
        }
    }
}
