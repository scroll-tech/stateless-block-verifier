use crate::helpers::run_async;
use clap::Subcommand;

mod file;
mod rpc;

#[derive(Subcommand, Debug)]
pub enum RunCommands {
    /// Run and verify a trace file
    #[command(name = "file")]
    RunFile(file::RunFileCommand),
    /// Run and verify from RPC
    #[command(name = "rpc")]
    RunRpc(rpc::RunRpcCommand),
}

impl RunCommands {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            RunCommands::RunFile(cmd) => cmd.run(),
            RunCommands::RunRpc(cmd) => Ok(run_async(cmd.run())?),
        }
    }
}
