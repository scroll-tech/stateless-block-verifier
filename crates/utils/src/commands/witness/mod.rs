use clap::Subcommand;

mod create;

#[derive(Subcommand)]
pub enum WitnessCommands {
    #[command(about = "Create a new witness")]
    Create(create::CreateWitnessCommand),
}

impl WitnessCommands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            WitnessCommands::Create(cmd) => cmd.run().await,
        }
    }
}
