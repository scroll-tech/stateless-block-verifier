use crate::utils;
use anyhow::bail;
use clap::Args;
use sbv::{
    core::{ChunkInfo, EvmExecutorBuilder, HardforkConfig},
    primitives::{types::BlockTrace, Block, B256},
};
use std::{cell::RefCell, path::PathBuf};
use tiny_keccak::{Hasher, Keccak};
use tokio::task::JoinSet;

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
        fork_config: impl Fn(u64) -> HardforkConfig + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        if self.chunk_mode {
            self.run_chunk(fork_config).await
        } else {
            self.run_traces(fork_config).await
        }
    }

    async fn run_traces(
        self,
        fork_config: impl Fn(u64) -> HardforkConfig + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        let mut tasks = JoinSet::new();

        for path in self.path.into_iter() {
            tasks.spawn(run_trace(path, fork_config));
        }

        while let Some(task) = tasks.join_next().await {
            if let Err(err) = task? {
                bail!("{:?}", err);
            }
        }

        Ok(())
    }

    async fn run_chunk(
        self,
        fork_config: impl Fn(u64) -> HardforkConfig + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        let traces = futures::future::join_all(self.path.iter().map(read_block_trace))
            .await
            .into_iter()
            .collect::<Result<Vec<BlockTrace>, _>>()?;

        let has_same_chain_id = traces
            .iter()
            .all(|trace| trace.chain_id() == traces[0].chain_id());
        if !has_same_chain_id {
            bail!("All traces must have the same chain id in chunk mode");
        }

        let has_seq_block_number = traces
            .windows(2)
            .all(|w| w[0].number() + 1 == w[1].number());
        if !has_seq_block_number {
            bail!("All traces must have sequential block numbers in chunk mode");
        }

        let fork_config = fork_config(traces[0].chain_id());
        let (chunk_info, zktrie_db) = ChunkInfo::from_block_traces(&traces);

        let tx_bytes_hasher = RefCell::new(Keccak::v256());

        let mut executor = EvmExecutorBuilder::new(zktrie_db.clone())
            .hardfork_config(fork_config)
            .with_execute_hooks(|hooks| {
                hooks.add_tx_rlp_handler(|_, rlp| {
                    tx_bytes_hasher.borrow_mut().update(rlp);
                });
            })
            .build(&traces[0])?;
        executor.handle_block(&traces[0])?;

        for trace in traces[1..].iter() {
            executor.update_db(trace)?;
            executor.handle_block(trace)?;
        }

        let post_state_root = executor.commit_changes(&zktrie_db);
        if post_state_root != chunk_info.post_state_root() {
            bail!("post state root mismatch");
        }
        drop(executor);

        let mut tx_bytes_hash = B256::ZERO;
        tx_bytes_hasher.into_inner().finalize(&mut tx_bytes_hash.0);
        let _public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash);

        dev_info!("[chunk mode] public input hash: {:?}", _public_input_hash);

        Ok(())
    }
}

async fn read_block_trace(path: &PathBuf) -> anyhow::Result<BlockTrace> {
    let trace = tokio::fs::read_to_string(&path).await?;
    tokio::task::spawn_blocking(move || deserialize_block_trace(&trace)).await?
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
) -> anyhow::Result<()> {
    let trace = read_block_trace(&path).await?;
    let fork_config = fork_config(trace.chain_id());
    tokio::task::spawn_blocking(move || utils::verify(&trace, &fork_config)).await??;
    Ok(())
}
