use clap::Subcommand;
use stateless_block_verifier::HardforkConfig;

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
    pub async fn run(
        self,
        fork_config: impl Fn(u64) -> HardforkConfig + Send + Sync + Copy + 'static,
        disable_checks: bool,
    ) -> anyhow::Result<()> {
        match self {
            Commands::RunFile(cmd) => cmd.run(fork_config, disable_checks).await,
            Commands::RunRpc(cmd) => cmd.run(fork_config, disable_checks).await,
        }
    }
}
