use crate::helpers::dump::dump_bundle_state;
use eyre::eyre;
use sbv::{
    core::{
        VerificationError,
        verifier::{self, VerifyResult},
    },
    primitives::{
        chainspec::ChainSpec,
        ext::{BlockWitnessExt, BlockWitnessRethExt},
    },
    trie::BlockWitnessTrieExt,
};
use std::{
    env,
    panic::{AssertUnwindSafe, UnwindSafe, catch_unwind},
    sync::Arc,
};

pub fn verify_catch_panics<
    T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt + UnwindSafe,
>(
    witness: T,
    chain_spec: Arc<ChainSpec>,
) -> eyre::Result<VerifyResult> {
    let chain_id = witness.chain_id();
    let block_number = witness.number();

    catch_unwind(AssertUnwindSafe(|| {
        verifier::run(
            &[witness],
            chain_spec,
            #[cfg(feature = "scroll")]
            verifier::StateCommitMode::Block,
            #[cfg(feature = "scroll")]
            None::<Vec<Vec<sbv::primitives::U256>>>,
        )
        .inspect_err(|e| {
            if let VerificationError::BlockRootMismatch { bundle_state, .. } = e {
                let dump_dir = env::temp_dir()
                    .join("dumps")
                    .join(format!("{chain_id}-{block_number}"));
                dump_bundle_state(bundle_state, &dump_dir)
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
            }
        })
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
