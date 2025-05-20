use clap::Args;
#[cfg(feature = "dev")]
use sbv::helpers::tracing;
use sbv::{primitives::types::BlockWitness, utils::verifier::*};
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
            gas_used += run_witness(path)?.gas_used
        }
        dev_info!("Gas used: {}", gas_used);

        Ok(())
    }

    #[cfg(feature = "scroll")]
    fn run_chunk(self) -> anyhow::Result<()> {
        use anyhow::bail;
        use sbv::primitives::{
            ext::BlockWitnessChunkExt,
            types::{BlockWitness, scroll::ChunkInfoBuilder},
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

        let output = verify_catch_panics(&witnesses)?;

        let mut chunk_info_builder = ChunkInfoBuilder::new(
            &output.chain_spec,
            witnesses.prev_state_root(),
            &output.blocks,
        );
        if let Some(prev_msg_queue_hash) = self.prev_msg_queue_hash {
            chunk_info_builder.set_prev_msg_queue_hash(prev_msg_queue_hash);
        }
        let chunk_info = chunk_info_builder.build(output.withdraw_root);
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
fn run_witness(path: PathBuf) -> anyhow::Result<VerifyOutput> {
    let witness = read_witness(&path)?;
    verify_catch_panics(&[witness]).inspect(|_| dev_info!("verified"))
}
