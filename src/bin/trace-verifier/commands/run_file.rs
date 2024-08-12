use clap::Args;
use eth_types::l2_types::BlockTrace;
use stateless_block_verifier::HardforkConfig;
use std::path::PathBuf;

use crate::utils;

#[derive(Args)]
pub struct RunFileCommand {
    /// Path to the trace file
    #[arg(short, long, default_value = "trace.json")]
    path: Vec<PathBuf>,
}

impl RunFileCommand {
    pub async fn run(
        self,
        fork_config: impl Fn(u64) -> HardforkConfig,
        disable_checks: bool,
    ) -> anyhow::Result<()> {
        for path in self.path {
            info!("Reading trace from {:?}", path);

            let trace: BlockTrace = tokio::fs::read_to_string(&path).await.and_then(|trace| {
                Ok(
                    // Try to deserialize `BlockTrace` from JSON. In case of failure, try to
                    // deserialize `BlockTrace` from a JSON-RPC response that has the actual block
                    // trace nested in the value of the key "result".
                    serde_json::from_str::<BlockTrace>(&trace).or::<serde_json::Error>({
                        #[derive(serde::Deserialize, Default, Debug, Clone)]
                        pub struct BlockTraceJsonRpcResult {
                            pub result: BlockTrace,
                        }
                        Ok(serde_json::from_str::<BlockTraceJsonRpcResult>(&trace)?.result)
                    })?,
                )
            })?;

            let fork_config = fork_config(trace.chain_id);
            tokio::task::spawn_blocking(move || {
                utils::verify(trace, &fork_config, disable_checks, false)
            })
            .await?;
        }
        Ok(())
    }
}
