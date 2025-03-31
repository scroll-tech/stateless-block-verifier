use clap::Subcommand;

mod run;
mod witness;

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(subcommand, about = "Run and verify witness")]
    Run(run::RunCommands),
    #[command(subcommand, about = "Witness helpers")]
    Witness(witness::WitnessCommands),
}

impl Commands {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Commands::Run(cmd) => cmd.run(),
            Commands::Witness(cmd) => cmd.run(),
        }
    }
}
