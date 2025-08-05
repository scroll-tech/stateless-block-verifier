use crate::helpers::dump::dump_bundle_state;
use anyhow::anyhow;
#[cfg(feature = "dev")]
use sbv::helpers::tracing;
use sbv::{
    core::VerificationError,
    primitives::{
        BlockWitness,
        chainspec::{Chain, ChainSpec, get_chain_spec_or_build},
    },
};
use std::{env, panic::catch_unwind, sync::Arc};

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.header.number), err))]
pub fn verify_catch_panics(witness: &BlockWitness) -> anyhow::Result<u64> {
    catch_unwind(|| verify(witness))
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
}

pub fn get_chain_spec(chain_id: u64) -> Arc<ChainSpec> {
    get_chain_spec_or_build(Chain::from_id(chain_id), |_spec| {
        #[cfg(feature = "scroll")]
        {
            use sbv::primitives::hardforks::{ForkCondition, ScrollHardfork};
            _spec
                .inner
                .hardforks
                .insert(ScrollHardfork::EuclidV2, ForkCondition::Timestamp(0));
            _spec
                .inner
                .hardforks
                .insert(ScrollHardfork::Feynman, ForkCondition::Timestamp(0));
        }
    })
}

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.header.number), err))]
fn verify(witness: &BlockWitness) -> Result<u64, VerificationError> {
    let chain_spec = get_chain_spec(witness.chain_id);
    match sbv::core::verifier::verify(witness, chain_spec) {
        Ok(gas_used) => Ok(gas_used),
        Err(VerificationError::RootMismatch {
            expected,
            actual,
            state,
        }) => {
            let dump_dir = env::temp_dir()
                .join("dumps")
                .join(format!("{}-{}", witness.chain_id, witness.header.number));
            dump_bundle_state(&state, &dump_dir)
                .inspect(|_| {
                    dev_info!("Dumped bundle state to: {}", dump_dir.display());
                })
                .inspect_err(|_e| {
                    dev_error!(
                        "Failed to dump bundle state to {}: {_e}",
                        dump_dir.display(),
                    );
                })
                .ok();
            Err(VerificationError::root_mismatch(expected, actual, state))
        }
        Err(e) => Err(e),
    }
}
