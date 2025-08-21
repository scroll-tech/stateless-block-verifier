use crate::helpers::verifier::*;
use clap::Args;
use eyre::ContextCompat;
use sbv::{
    core::verifier::VerifyResult,
    primitives::{
        chainspec::{Chain, build_chain_spec_force_hardfork, get_chain_spec},
        hardforks::Hardfork,
        types::BlockWitness,
    },
};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct RunFileCommand {
    /// Path to the witness file
    #[arg(default_value = "witness.json")]
    path: Vec<PathBuf>,
    /// Hardfork
    #[arg(long, value_parser = clap::value_parser!(Hardfork))]
    hardfork: Option<Hardfork>,
}

impl RunFileCommand {
    pub fn run(self) -> eyre::Result<()> {
        let mut gas_used = 0;
        for path in self.path.into_iter() {
            gas_used += run_witness(path, self.hardfork)?.gas_used;
        }
        dev_info!("Gas used: {}", gas_used);

        Ok(())
    }
}

fn read_witness(path: &PathBuf) -> eyre::Result<BlockWitness> {
    let witness = std::fs::File::open(path)?;
    let jd = &mut serde_json::Deserializer::from_reader(&witness);
    let witness = serde_path_to_error::deserialize::<_, BlockWitness>(jd)?;
    Ok(witness)
}

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(path = %path.display()), err))]
fn run_witness(path: PathBuf, hardfork: Option<Hardfork>) -> eyre::Result<VerifyResult> {
    let witness = read_witness(&path)?;
    let chain = Chain::from_id(witness.chain_id);
    let chain_spec = if let Some(hardfork) = hardfork {
        dev_info!("Overriding hardfork to: {hardfork:?}");
        build_chain_spec_force_hardfork(chain, hardfork)
    } else {
        get_chain_spec(chain).context("chain not support")?
    };
    verify_catch_panics(witness, chain_spec).inspect(|_| dev_info!("verified"))
}
