use clap::Subcommand;

pub mod witness;

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand, about = "Witness commands")]
    Witness(witness::WitnessCommands),
}

impl Commands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            Commands::Witness(cmd) => cmd.run().await,
        }
    }
}
