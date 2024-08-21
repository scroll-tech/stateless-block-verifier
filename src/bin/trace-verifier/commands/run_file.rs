use clap::Args;
use eth_types::l2_types::BlockTrace;
use futures::TryFutureExt;
use stateless_block_verifier::{dev_info, ChunkInfo, HardforkConfig};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tiny_keccak::{Hasher, Keccak};

use crate::utils;

#[derive(Args)]
pub struct RunFileCommand {
    /// Path to the trace file
    #[arg(default_value = "trace.json")]
    path: Vec<PathBuf>,
    /// Chunk mode
    #[arg(short, long)]
    chunk_mode: bool,
}

impl RunFileCommand {
    pub async fn run(
        self,
        fork_config: impl Fn(u64) -> HardforkConfig,
        disable_checks: bool,
    ) -> anyhow::Result<()> {
        let traces = futures::future::join_all(self.path.into_iter().map(|path| {
            tokio::fs::read_to_string(path)
                .map_err(anyhow::Error::from)
                .and_then(|trace| {
                    // Try to deserialize `BlockTrace` from JSON. In case of failure, try to
                    // deserialize `BlockTrace` from a JSON-RPC response that has the actual block
                    // trace nested in the value of the key "result".
                    #[derive(serde::Deserialize, Default, Debug, Clone)]
                    pub struct BlockTraceJsonRpcResult {
                        pub result: BlockTrace,
                    }
                    let result = serde_json::from_str::<BlockTraceJsonRpcResult>(&trace)
                        .map(|res| res.result)
                        .or_else(|_| serde_json::from_str::<BlockTrace>(&trace))
                        .map_err(anyhow::Error::from);
                    futures::future::ready(result)
                })
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        // Check if traces can be chunked
        let tx_bytes_hasher = if self.chunk_mode {
            let has_same_chain_id = traces
                .iter()
                .all(|trace| trace.chain_id == traces[0].chain_id);
            if !has_same_chain_id {
                anyhow::bail!("All traces must have the same chain id in chunk mode");
            }

            let has_seq_block_number = traces
                .windows(2)
                .all(|w| w[0].header.number.unwrap() + 1 == w[1].header.number.unwrap());
            if !has_seq_block_number {
                anyhow::bail!("All traces must have sequential block numbers in chunk mode");
            }

            Some(Rc::new(RefCell::new(Keccak::v256())))
        } else {
            None
        };

        for trace in traces.iter() {
            let fork_config = fork_config(trace.chain_id);
            utils::verify(
                trace,
                &fork_config,
                disable_checks,
                tx_bytes_hasher.clone(),
                false,
            )?;
        }

        if self.chunk_mode {
            let chunk_info = ChunkInfo::from_block_traces(&traces);
            let mut tx_bytes_hash = [0u8; 32];
            let haser = Rc::into_inner(tx_bytes_hasher.unwrap()).unwrap();
            haser.into_inner().finalize(&mut tx_bytes_hash);
            let public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash.into());
            dev_info!("[chunk mode] public input hash: {:?}", public_input_hash);
        }

        Ok(())
    }
}
