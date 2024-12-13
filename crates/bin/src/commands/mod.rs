use clap::Subcommand;

mod run_file;
// FIXME
// mod run_rpc;

#[derive(Subcommand)]
pub enum Commands {
    /// Run and verify a trace file
    #[command(name = "run-file")]
    RunFile(run_file::RunFileCommand),
    // FIXME
    // /// Fetch and verify traces from geth rpc
    // #[command(name = "run-rpc")]
    // RunRpc(run_rpc::RunRpcCommand),
}

impl Commands {
    pub async fn run(self) -> anyhow::Result<()> {
        match self {
            Commands::RunFile(cmd) => cmd.run().await,
            // FIXME
            // Commands::RunRpc(cmd) => cmd.run().await,
        }
    }
}
