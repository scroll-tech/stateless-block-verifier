use crate::helpers::verifier::*;
use clap::Args;
#[cfg(feature = "dev")]
use sbv::helpers::tracing;
use sbv::primitives::types::BlockWitness;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct RunFileCommand {
    /// Path to the witness file
    #[arg(default_value = "witness.json")]
    path: Vec<PathBuf>,
    /// Chunk mode
    #[cfg(feature = "scroll")]
    #[arg(short, long)]
    chunk_mode: bool,
    #[cfg(feature = "scroll")]
    #[arg(long)]
    prev_msg_queue_hash: Option<sbv::primitives::B256>,
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
        let mut gas_used = 0;
        for path in self.path.into_iter() {
            gas_used += run_witness(path)?
        }
        dev_info!("Gas used: {}", gas_used);

        Ok(())
    }

    #[cfg(feature = "scroll")]
    fn run_chunk(self) -> anyhow::Result<()> {
        use anyhow::bail;
        use sbv::{
            core::{EvmDatabase, EvmExecutor},
            kv::{nohash::NoHashMap, null::NullProvider},
            primitives::{
                ext::{BlockWitnessChunkExt, BlockWitnessExt, BlockWitnessRethExt},
                types::{BlockWitness, scroll::ChunkInfoBuilder},
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

        let chain_spec = get_chain_spec(witnesses.chain_id());

        let mut chunk_info_builder =
            ChunkInfoBuilder::new(&chain_spec, witnesses.prev_state_root(), &blocks);
        if let Some(prev_msg_queue_hash) = self.prev_msg_queue_hash {
            chunk_info_builder.set_prev_msg_queue_hash(prev_msg_queue_hash);
        }

        let mut code_db = NoHashMap::default();
        witnesses.import_codes(&mut code_db);
        let mut nodes_provider = NoHashMap::default();
        witnesses.import_nodes(&mut nodes_provider)?;

        let mut db = EvmDatabase::new_from_root(
            &code_db,
            chunk_info_builder.prev_state_root(),
            &nodes_provider,
            &NullProvider,
        )?;
        for block in &blocks {
            use sbv::primitives::U256;

            let output =
                EvmExecutor::new(chain_spec.clone(), &db, block, None::<Vec<U256>>).execute()?;
            db.update(
                &nodes_provider,
                std::collections::BTreeMap::from_iter(output.state.state).iter(),
            )?;
        }
        let post_state_root = db.commit_changes();
        if post_state_root != chunk_info_builder.post_state_root() {
            bail!("post state root mismatch");
        }

        let chunk_info = chunk_info_builder.build(db.withdraw_root()?);
        let _public_input_hash = chunk_info.pi_hash();
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

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(path = %path.display()), err))]
fn run_witness(path: PathBuf) -> anyhow::Result<u64> {
    let witness = read_witness(&path)?;
    verify_catch_panics(&witness).inspect(|_| dev_info!("verified"))
}
