use clap::Subcommand;

mod run_file;
mod run_rpc;

#[derive(Subcommand)]
pub enum Commands {
    /// Run and verify a trace file
    #[command(name = "run-file")]
    RunFile(run_file::RunFileCommand),
    /// Fetch and verify traces from geth rpc
    #[command(name = "run-rpc")]
    RunRpc(run_rpc::RunRpcCommand),
}

impl Commands {
    pub async fn run(self, curie_block: Option<u64>, disable_checks: bool) -> anyhow::Result<()> {
        match self {
            Commands::RunFile(cmd) => cmd.run(curie_block, disable_checks).await,
            Commands::RunRpc(cmd) => cmd.run(curie_block, disable_checks).await,
        }
    }
}
