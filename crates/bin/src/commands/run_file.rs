use crate::utils;
use anyhow::anyhow;
use clap::Args;
use sbv::primitives::types::BlockWitness;
use std::panic::catch_unwind;
use std::path::PathBuf;
use tokio::task::JoinSet;

#[derive(Args)]
pub struct RunFileCommand {
    /// Path to the witness file
    #[arg(default_value = "witness.json")]
    path: Vec<PathBuf>,
    // FIXME
    // /// Chunk mode
    // #[arg(short, long)]
    // chunk_mode: bool,
}

impl RunFileCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        self.run_witnesses().await
    }

    async fn run_witnesses(self) -> anyhow::Result<()> {
        let mut tasks = JoinSet::new();

        for path in self.path.into_iter() {
            tasks.spawn(run_witness(path));
        }

        while let Some(task) = tasks.join_next().await {
            if let Err(err) = task? {
                dev_error!("{:?}", err);
            }
        }

        Ok(())
    }

    // FIXME
    // async fn run_chunk(
    //     self,
    //     fork_config: impl Fn(u64) -> HardforkConfig + Send + Sync + Copy + 'static,
    // ) -> anyhow::Result<()> {
    //     let traces = futures::future::join_all(self.path.iter().map(read_block_trace))
    //         .await
    //         .into_iter()
    //         .collect::<Result<Vec<BlockTrace>, _>>()?;
    //
    //     let has_same_chain_id = traces
    //         .iter()
    //         .all(|trace| trace.chain_id() == traces[0].chain_id());
    //     if !has_same_chain_id {
    //         bail!("All traces must have the same chain id in chunk mode");
    //     }
    //
    //     let has_seq_block_number = traces
    //         .windows(2)
    //         .all(|w| w[0].number() + 1 == w[1].number());
    //     if !has_seq_block_number {
    //         bail!("All traces must have sequential block numbers in chunk mode");
    //     }
    //
    //     let fork_config = fork_config(traces[0].chain_id());
    //     let (chunk_info, mut zktrie_db) = ChunkInfo::from_block_traces(&traces);
    //     let mut code_db = HashMapDb::default();
    //
    //     let mut tx_bytes_hasher = Keccak::v256();
    //
    //     let mut executor = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
    //         .hardfork_config(fork_config)
    //         .chain_id(traces[0].chain_id())
    //         .build(traces[0].root_before())?;
    //     for trace in traces.iter() {
    //         executor.insert_codes(trace)?;
    //     }
    //
    //     for trace in traces.iter() {
    //         let BlockExecutionResult { tx_rlps, .. } = executor.handle_block(trace)?;
    //         for tx_rlp in tx_rlps {
    //             tx_bytes_hasher.update(&tx_rlp);
    //         }
    //     }
    //
    //     let post_state_root = executor.commit_changes()?;
    //     if post_state_root != chunk_info.post_state_root() {
    //         bail!("post state root mismatch");
    //     }
    //     drop(executor);
    //
    //     let mut tx_bytes_hash = B256::ZERO;
    //     tx_bytes_hasher.finalize(&mut tx_bytes_hash.0);
    //     let _public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash);
    //
    //     dev_info!("[chunk mode] public input hash: {:?}", _public_input_hash);
    //
    //     Ok(())
    // }
}

async fn read_witness(path: &PathBuf) -> anyhow::Result<BlockWitness> {
    let witness = tokio::fs::read(&path).await?;
    let jd = &mut serde_json::Deserializer::from_slice(&witness);
    let witness = serde_path_to_error::deserialize::<_, BlockWitness>(jd)?;
    Ok(witness)
}

async fn run_witness(path: PathBuf) -> anyhow::Result<()> {
    let witness = read_witness(&path).await?;
    if let Err(e) = tokio::task::spawn_blocking(move || catch_unwind(|| utils::verify(&witness)))
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
