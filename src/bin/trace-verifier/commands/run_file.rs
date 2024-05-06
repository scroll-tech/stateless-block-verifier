use crate::utils;
use clap::Args;
use eth_types::l2_types::BlockTrace;
use std::path::PathBuf;

#[derive(Args)]
pub struct RunFileCommand {
    /// Path to the trace file
    #[arg(short, long, default_value = "trace.json")]
    path: Vec<PathBuf>,
}

impl RunFileCommand {
    pub async fn run(self, disable_checks: bool) -> anyhow::Result<()> {
        for path in self.path {
            info!("Reading trace from {:?}", path);
            let trace = tokio::fs::read_to_string(&path).await?;
            let l2_trace: BlockTrace = serde_json::from_str(&trace).unwrap_or_else(|_| {
                #[derive(serde::Deserialize, Default, Debug, Clone)]
                pub struct BlockTraceJsonRpcResult {
                    pub result: BlockTrace,
                }
                serde_json::from_str::<BlockTraceJsonRpcResult>(&trace)
                    .unwrap()
                    .result
            });
            tokio::task::spawn_blocking(move || utils::verify(l2_trace, disable_checks)).await?;
        }
        Ok(())
    }
}
