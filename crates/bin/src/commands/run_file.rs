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
    /// Chunk mode
    #[cfg(feature = "scroll")]
    #[arg(short, long)]
    chunk_mode: bool,
}

impl RunFileCommand {
    #[cfg(not(feature = "scroll"))]
    pub async fn run(self) -> anyhow::Result<()> {
        self.run_witnesses().await
    }

    #[cfg(feature = "scroll")]
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

    #[cfg(feature = "scroll")]
    async fn run_chunk(self) -> anyhow::Result<()> {
        use anyhow::bail;
        use sbv::{
            core::{ChunkInfo, EvmDatabase, EvmExecutor},
            kv::{nohash::NoHashMap, null::NullProvider},
            primitives::{
                chainspec::{get_chain_spec, Chain},
                ext::{BlockWitnessChunkExt, BlockWitnessExt, TxBytesHashExt},
                types::BlockWitness,
                BlockWitness as _,
            },
            trie::BlockWitnessTrieExt,
        };

        let witnesses = futures::future::join_all(self.path.iter().map(read_witness))
            .await
            .into_iter()
            .collect::<Result<Vec<BlockWitness>, _>>()?;

        if !witnesses.has_same_chain_id() {
            bail!("All traces must have the same chain id in chunk mode");
        }

        if !witnesses.has_seq_block_number() {
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

        let mut db = EvmDatabase::new_from_root(
            &code_db,
            chunk_info.prev_state_root(),
            &nodes_provider,
            &NullProvider,
        );
        for block in blocks.iter() {
            let output = EvmExecutor::new(chain_spec.clone(), &db, block).execute()?;
            db.update(&nodes_provider, output.state.state.iter());
        }
        let post_state_root = db.commit_changes();
        if post_state_root != chunk_info.post_state_root() {
            bail!("post state root mismatch");
        }

        let withdraw_root = db.withdraw_root()?;
        let tx_bytes_hash = blocks
            .iter()
            .flat_map(|b| b.block.body.transactions.iter())
            .tx_bytes_hash();
        let _public_input_hash = chunk_info.public_input_hash(&withdraw_root, &tx_bytes_hash);
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
    utils::verify(&witness).unwrap();
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
