use eyre::eyre;
use sbv::{
    core::{
        verifier::{self, VerifyResult},
        witness::BlockWitness,
    },
    primitives::chainspec::ChainSpec,
};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

pub fn verify_catch_panics(
    witness: BlockWitness,
    chain_spec: Arc<ChainSpec>,
) -> eyre::Result<VerifyResult> {
    catch_unwind(AssertUnwindSafe(|| {
        verifier::run_host(&[witness], chain_spec)
    }))
    .map_err(|e| {
        e.downcast_ref::<&str>()
            .map(|s| eyre!("task panics with: {s}"))
            .or_else(|| {
                e.downcast_ref::<String>()
                    .map(|s| eyre!("task panics with: {s}"))
            })
            .unwrap_or_else(|| eyre!("task panics"))
    })
    .and_then(|r| r.map_err(eyre::Error::from))
}
