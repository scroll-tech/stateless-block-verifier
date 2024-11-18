use crate::utils;
use anyhow::{anyhow, bail};
use clap::Args;
use sbv::{
    core::{BlockExecutionResult, ChunkInfo, EvmExecutorBuilder, HardforkConfig},
    primitives::{
        types::{BlockTrace, LegacyStorageTrace},
        zk_trie::{db::kv::HashMapDb, hash::poseidon::Poseidon},
        Block, B256,
    },
};
use serde::Deserialize;
use std::panic::catch_unwind;
use std::path::PathBuf;
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
                dev_error!("{:?}", err);
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
        let (chunk_info, mut zktrie_db) = ChunkInfo::from_block_traces(&traces);
        let mut code_db = HashMapDb::default();

        let mut tx_bytes_hasher = Keccak::v256();

        let mut executor = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
            .hardfork_config(fork_config)
            .chain_id(traces[0].chain_id())
            .hash_scheme(Poseidon)
            .build(traces[0].root_before())?;
        for trace in traces.iter() {
            executor.insert_codes(trace)?;
        }

        for trace in traces.iter() {
            let BlockExecutionResult { tx_rlps, .. } = executor.handle_block(trace)?;
            for tx_rlp in tx_rlps {
                tx_bytes_hasher.update(&tx_rlp);
            }
        }

        let post_state_root = executor.commit_changes()?;
        if post_state_root != chunk_info.post_state_root() {
            bail!("post state root mismatch");
        }
        drop(executor);

        let mut tx_bytes_hash = B256::ZERO;
        tx_bytes_hasher.finalize(&mut tx_bytes_hash.0);
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
    let block_trace = deserialize_may_wrapped::<BlockTrace>(trace)?;
    if block_trace.storage_trace.flatten_proofs.is_empty() {
        dev_warn!("Storage trace is empty, try to deserialize as legacy storage trace");
        let legacy_trace = deserialize_may_wrapped::<BlockTrace<LegacyStorageTrace>>(trace)?;
        return Ok(legacy_trace.into());
    }
    Ok(block_trace)
}
fn deserialize_may_wrapped<'de, T: Deserialize<'de>>(trace: &'de str) -> anyhow::Result<T> {
    // Try to deserialize `BlockTrace` from JSON. In case of failure, try to
    // deserialize `BlockTrace` from a JSON-RPC response that has the actual block
    // trace nested in the value of the key "result".
    Ok(serde_json::from_str::<T>(trace).or_else(|_| {
        #[derive(serde::Deserialize, Default, Debug, Clone)]
        pub struct BlockTraceJsonRpcResult<T> {
            pub result: T,
        }
        Ok::<_, serde_json::Error>(
            serde_json::from_str::<BlockTraceJsonRpcResult<T>>(trace)?.result,
        )
    })?)
}

async fn run_trace(
    path: PathBuf,
    fork_config: impl Fn(u64) -> HardforkConfig,
) -> anyhow::Result<()> {
    let trace = read_block_trace(&path).await?;
    let fork_config = fork_config(trace.chain_id());
    if let Err(e) =
        tokio::task::spawn_blocking(move || catch_unwind(|| utils::verify(&trace, &fork_config)))
            .await?
            .map_err(|e| {
                e.downcast_ref::<&str>()
                    .map(|s| anyhow!("task panics with: {s}"))
                    .or_else(|| {
                        e.downcast_ref::<String>()
                            .map(|s| anyhow!("task panics with: {s}"))
                    })
                    .unwrap_or_else(|| anyhow!("task panics"))
            })
            .and_then(|r| r.map_err(anyhow::Error::from))
    {
        dev_error!(
            "Error occurs when verifying block ({}): {:?}",
            path.display(),
            e
        );
        return Err(e);
    }
    Ok(())
}
