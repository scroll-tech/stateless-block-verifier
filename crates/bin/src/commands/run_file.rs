use crate::utils;
use anyhow::anyhow;
use clap::Args;
use sbv::primitives::types::BlockWitness;
use std::{panic::catch_unwind, path::PathBuf};

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
    pub fn run(self) -> anyhow::Result<()> {
        self.run_witnesses()
    }

    #[cfg(feature = "scroll")]
    pub fn run(self) -> anyhow::Result<()> {
        if self.chunk_mode {
            self.run_chunk()
        } else {
            self.run_witnesses()
        }
    }

    fn run_witnesses(self) -> anyhow::Result<()> {
        for path in self.path.into_iter() {
            run_witness(path)?
        }

        Ok(())
    }

    #[cfg(feature = "scroll")]
    fn run_chunk(self) -> anyhow::Result<()> {
        use anyhow::bail;
        use sbv::{
            core::{ChunkInfoBuilder, EvmDatabase, EvmExecutor},
            kv::{nohash::NoHashMap, null::NullProvider},
            primitives::{
                BlockWitness as _,
                chainspec::{Chain, get_chain_spec},
                ext::{BlockWitnessChunkExt, BlockWitnessExt, TxBytesHashExt},
                types::BlockWitness,
            },
            trie::BlockWitnessTrieExt,
        };

        let witnesses = self
            .path
            .iter()
            .map(read_witness)
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
        let chunk_info = ChunkInfoBuilder::from_blocks_iter(
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
        )?;
        for block in blocks.iter() {
            let output = EvmExecutor::new(chain_spec.clone(), &db, block).execute()?;
            db.update(&nodes_provider, output.state.state.iter())?;
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
        let _public_input_hash = chunk_info.build(withdraw_root, tx_bytes_hash).hash();
        dev_info!("[chunk mode] public input hash: {_public_input_hash:?}");

        Ok(())
    }
}

fn read_witness(path: &PathBuf) -> anyhow::Result<BlockWitness> {
    let witness = std::fs::File::open(path)?;
    let jd = &mut serde_json::Deserializer::from_reader(&witness);
    let witness = serde_path_to_error::deserialize::<_, BlockWitness>(jd)?;
    Ok(witness)
}

fn run_witness(path: PathBuf) -> anyhow::Result<()> {
    let witness = read_witness(&path)?;
    if let Err(e) = catch_unwind(|| utils::verify(&witness)).map_err(|e| {
        e.downcast_ref::<&str>()
            .map(|s| anyhow!("task panics with: {s}"))
            .or_else(|| {
                e.downcast_ref::<String>()
                    .map(|s| anyhow!("task panics with: {s}"))
            })
            .unwrap_or_else(|| anyhow!("task panics"))
    }) {
        dev_error!(
            "Error occurs when verifying block ({}): {:?}",
            path.display(),
            e
        );
        return Err(e);
    }
    Ok(())
}
