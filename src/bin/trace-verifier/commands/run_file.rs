use clap::Args;
use eth_types::l2_types::BlockTrace;
use eth_types::H256;
use futures::TryFutureExt;
use stateless_block_verifier::{dev_info, ChunkInfo, EvmExecutorBuilder, HardforkConfig};
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
        if self.chunk_mode {
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
        }

        if !self.chunk_mode {
            for trace in traces.iter() {
                let fork_config = fork_config(trace.chain_id);
                utils::verify(trace, &fork_config, disable_checks, false)?;
            }
        } else {
            let fork_config = HardforkConfig::default_from_chain_id(traces[0].chain_id);
            let (chunk_info, zktrie_state) = ChunkInfo::from_block_traces(&traces);

            let tx_bytes_hasher = Rc::new(RefCell::new(Keccak::v256()));

            let mut executor = EvmExecutorBuilder::new()
                .hardfork_config(fork_config)
                .with_execute_hooks(|hooks| {
                    let hasher = tx_bytes_hasher.clone();
                    hooks.add_tx_rlp_handler(move |_, rlp| {
                        hasher.borrow_mut().update(rlp);
                    });
                })
                .zktrie_state(&zktrie_state)
                .build(&traces[0]);
            executor.handle_block(&traces[0])?;

            for trace in traces[1..].iter() {
                executor.update_db(trace, &zktrie_state);
                executor.handle_block(trace)?;
            }

            let post_state_root = executor.commit_changes();
            if post_state_root != chunk_info.post_state_root() {
                anyhow::bail!("post state root mismatch");
            }
            drop(executor);

            let mut tx_bytes_hash = H256::zero();
            let hasher = Rc::into_inner(tx_bytes_hasher).unwrap();
            hasher.into_inner().finalize(&mut tx_bytes_hash.0);
            let _public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash);
            dev_info!("[chunk mode] public input hash: {:?}", _public_input_hash);
        }

        Ok(())
    }
}
