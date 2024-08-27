use crate::utils;
use anyhow::bail;
use clap::Args;
use eth_types::l2_types::BlockTrace;
use futures::TryFutureExt;
use stateless_block_verifier::{dev_error, dev_info, HardforkConfig};
use std::future::ready;
use std::path::PathBuf;
use tokio::task::JoinSet;

#[derive(Args)]
pub struct RunFileCommand {
    /// Path to the trace file
    #[arg(default_value = "trace.json")]
    path: Vec<PathBuf>,
}

fn deserialize_block_trace(trace: &str) -> anyhow::Result<BlockTrace> {
    Ok(
        // Try to deserialize `BlockTrace` from JSON. In case of failure, try to
        // deserialize `BlockTrace` from a JSON-RPC response that has the actual block
        // trace nested in the value of the key "result".
        serde_json::from_str::<BlockTrace>(trace).or_else(|_| {
            #[derive(serde::Deserialize, Default, Debug, Clone)]
            pub struct BlockTraceJsonRpcResult {
                pub result: BlockTrace,
            }
            Ok::<_, serde_json::Error>(
                serde_json::from_str::<BlockTraceJsonRpcResult>(trace)?.result,
            )
        })?,
    )
}

async fn run_trace(
    path: PathBuf,
    fork_config: impl Fn(u64) -> HardforkConfig,
    disable_checks: bool,
) -> anyhow::Result<()> {
    let trace = tokio::fs::read_to_string(&path).await?;
    let trace = tokio::task::spawn_blocking(move || deserialize_block_trace(&trace)).await??;
    let fork_config = fork_config(trace.chain_id);
    tokio::task::spawn_blocking(move || utils::verify(trace, &fork_config, disable_checks, false))
        .await??;
    Ok(())
}

impl RunFileCommand {
    pub async fn run(
        self,
        fork_config: impl Fn(u64) -> HardforkConfig + Send + Sync + Copy + 'static,
        disable_checks: bool,
        parallel: usize,
    ) -> anyhow::Result<()> {
        let mut tasks = JoinSet::new();

        for path in self.path.into_iter() {
            tasks.spawn(run_trace(path, fork_config, disable_checks));
        }

        while let Some(task) = tasks.join_next().await {
            if let Err(err) = task? {
                bail!("{:?}", err);
            }
        }

        Ok(())
    }
}
