use crate::utils;
use anyhow::{anyhow, bail};
use clap::Args;
use sbv::primitives::B256;
use sbv::{
    core::{ChunkInfo, EvmDatabase, EvmExecutor},
    kv::nohash::NoHashMap,
    primitives::{
        chainspec::{get_chain_spec, Chain},
        eips::Encodable2718,
        types::BlockWitness,
        BlockWitness as _, BlockWitnessBlockHashExt, BlockWitnessCodeExt,
    },
    trie::BlockWitnessTrieExt,
};
use std::panic::catch_unwind;
use std::path::PathBuf;
use tiny_keccak::{Hasher, Keccak};
use tokio::task::JoinSet;

#[derive(Args)]
pub struct RunFileCommand {
    /// Path to the witness file
    #[arg(default_value = "witness.json")]
    path: Vec<PathBuf>,
    /// Chunk mode
    #[arg(short, long)]
    chunk_mode: bool,
}

impl RunFileCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        if self.chunk_mode {
            self.run_chunk().await
        } else {
            self.run_witnesses().await
        }
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

    async fn run_chunk(self) -> anyhow::Result<()> {
        let witnesses = futures::future::join_all(self.path.iter().map(read_witness))
            .await
            .into_iter()
            .collect::<Result<Vec<BlockWitness>, _>>()?;

        let has_same_chain_id = witnesses.windows(2).all(|w| w[0].chain_id == w[1].chain_id);
        if !has_same_chain_id {
            bail!("All traces must have the same chain id in chunk mode");
        }

        let has_seq_block_number = witnesses
            .windows(2)
            .all(|w| w[0].header.number + 1 == w[1].header.number);
        if !has_seq_block_number {
            bail!("All traces must have sequential block numbers in chunk mode");
        }

        let blocks = witnesses
            .iter()
            .map(|w| w.build_reth_block())
            .collect::<Result<Vec<_>, _>>()?;
        let chunk_info = ChunkInfo::from_blocks_iter(
            witnesses[0].chain_id,
            witnesses[0].pre_state_root,
            blocks.iter().map(|b| &b.block),
        );

        let chain_spec = get_chain_spec(Chain::from_id(chunk_info.chain_id())).unwrap();
        let mut code_db = NoHashMap::default();
        witnesses.import_codes(&mut code_db);
        let mut nodes_provider = NoHashMap::default();
        witnesses.import_nodes(&mut nodes_provider)?;
        let mut block_hashes = NoHashMap::default();
        witnesses.import_block_hashes(&mut block_hashes);

        let mut db = EvmDatabase::new_from_root(
            &code_db,
            chunk_info.prev_state_root(),
            &nodes_provider,
            &block_hashes,
        );
        for block in blocks.iter() {
            let output = EvmExecutor::new(chain_spec.clone(), &db, block).execute()?;
            db.update(&nodes_provider, output.state.state.iter());
        }
        let post_state_root = db.commit_changes();
        if post_state_root != chunk_info.post_state_root() {
            bail!("post state root mismatch");
        }

        let mut rlp_buffer = Vec::new();
        let mut tx_bytes_hasher = Keccak::v256();
        for block in blocks.iter() {
            for tx in block.block.body.transactions.iter() {
                tx.encode_2718(&mut rlp_buffer);
                tx_bytes_hasher.update(&rlp_buffer);
                rlp_buffer.clear();
            }
        }
        let mut tx_bytes_hash = B256::ZERO;
        tx_bytes_hasher.finalize(&mut tx_bytes_hash.0);
        let _public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash);
        dev_info!("[chunk mode] public input hash: {_public_input_hash:?}");

        Ok(())
    }
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
