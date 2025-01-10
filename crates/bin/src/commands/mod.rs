use clap::Subcommand;

mod run_file;
#[derive(Subcommand)]
pub enum Commands {
    /// Run and verify a trace file
    #[command(name = "run-file")]
    RunFile(run_file::RunFileCommand),
}

impl Commands {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Commands::RunFile(cmd) => cmd.run(),
        }
    }
}
