//! Standard block witness verifier example.

use crate::{DatabaseError, VerificationError};
use sbv_primitives::{
    B256, BlockWitness,
    chainspec::ChainSpec,
    ext::{BlockWitnessChunkExt, BlockWitnessRethExt},
    types::reth::primitives::{Block, RecoveredBlock},
};
use std::sync::Arc;

mod inner;

/// State commit mode for the block witness verification process.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
pub enum StateCommitMode {
    /// Commit state by chunk.
    Chunk,
    /// Commit state by block.
    Block,
    /// Use chunk mode first if it fails, fallback to block mode.
    Auto,
}

/// Result of the block witness verification process.
#[derive(Debug)]
pub struct VerifyResult {
    /// Recovered blocks from the witnesses.
    pub blocks: Vec<RecoveredBlock<Block>>,
    /// Pre-state root of the first block.
    pub pre_state_root: B256,
    /// Post-state root after executing the witnesses.
    pub post_state_root: B256,
    /// Withdrawal root after executing the witnesses.
    #[cfg(feature = "scroll")]
    pub withdraw_root: B256,
    /// Gas used during the verification process.
    pub gas_used: u64,
}

/// Verify the block witness and return the gas used.
pub fn run<T: BlockWitness + BlockWitnessRethExt>(
    witnesses: &[T],
    chain_spec: Arc<ChainSpec>,
    state_commit_mode: StateCommitMode,
    #[cfg(feature = "scroll")] compression_ratios: Option<
        impl IntoIterator<Item = impl IntoIterator<Item = impl Into<sbv_primitives::U256>>> + Clone,
    >,
) -> Result<VerifyResult, VerificationError> {
    if witnesses.is_empty() {
        return Err(VerificationError::EmptyWitnesses);
    }
    if !witnesses.has_same_chain_id() {
        return Err(VerificationError::ChainIdMismatch);
    }
    if !witnesses.has_seq_block_number() {
        return Err(VerificationError::NonSequentialWitnesses);
    }

    let (code_db, nodes_provider, block_hashes) = inner::make_providers(&witnesses);
    let code_db = manually_drop_on_zkvm!(code_db);
    let nodes_provider = manually_drop_on_zkvm!(nodes_provider);

    let blocks = witnesses
        .iter()
        .map(|w| {
            dev_trace!("{w:#?}");
            w.build_reth_block()
        })
        .collect::<Result<Vec<RecoveredBlock<Block>>, _>>()?;
    let pre_state_root = witnesses[0].pre_state_root();

    let mut args = inner::ExecuteInnerArgs {
        code_db: &code_db,
        nodes_provider: &nodes_provider,
        block_hashes: &block_hashes,
        pre_state_root,
        blocks: &blocks,
        chain_spec: chain_spec.clone(),
        defer_commit: true,
        #[cfg(feature = "scroll")]
        compression_ratios,
    };

    let result = match state_commit_mode {
        StateCommitMode::Chunk | StateCommitMode::Block => {
            args.defer_commit = matches!(state_commit_mode, StateCommitMode::Chunk);
            inner::execute(args)?
        }
        StateCommitMode::Auto => match inner::execute(args.clone()) {
            Ok(result) => result,
            Err(VerificationError::Database(DatabaseError::PartialStateTrie(_e))) => {
                dev_warn!(
                    "Failed to execute with defer commit enabled: {_e}; retrying with defer commit disabled"
                );
                #[cfg(target_os = "zkvm")]
                {
                    println!(format!(
                        "failed to update db: {_e}; retrying with defer commit disabled"
                    ));
                }
                args.defer_commit = false;
                inner::execute(args)?
            }
            Err(e) => return Err(e),
        },
    };

    #[cfg(feature = "scroll")]
    let (post_state_root, withdraw_root, gas_used) = result;
    #[cfg(not(feature = "scroll"))]
    let (post_state_root, gas_used) = result;

    Ok(VerifyResult {
        blocks,
        pre_state_root,
        post_state_root,
        #[cfg(feature = "scroll")]
        withdraw_root,
        gas_used,
    })
}

#[cfg(test)]
#[cfg(feature = "scroll")]
mod tests {
    use super::*;
    use sbv_primitives::{
        U256,
        chainspec::{Chain, build_chain_spec_force_hardfork},
        hardforks::Hardfork,
        types::BlockWitness,
    };

    #[rstest::rstest]
    fn test_euclid_v1(
        #[files("../../testdata/scroll_witness/euclidv1/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::Euclid);
        run(
            &[witness],
            chain_spec,
            StateCommitMode::Block,
            None::<Vec<Vec<U256>>>,
        )
        .unwrap();
    }

    #[rstest::rstest]
    fn test_euclid_v2(
        #[files("../../testdata/scroll_witness/euclidv2/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::EuclidV2);
        run(
            &[witness],
            chain_spec,
            StateCommitMode::Block,
            None::<Vec<Vec<U256>>>,
        )
        .unwrap();
    }

    #[rstest::rstest]
    fn test_feynman(
        #[files("../../testdata/scroll_witness/feynman/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::Feynman);
        run(
            &[witness],
            chain_spec,
            StateCommitMode::Block,
            None::<Vec<Vec<U256>>>,
        )
        .unwrap();
    }
}

#[cfg(test)]
#[cfg(not(feature = "scroll"))]
mod tests {
    use sbv_primitives::{
        chainspec::{Chain, get_chain_spec},
        types::BlockWitness,
    };

    #[rstest::rstest]
    fn test_mainnet(
        #[files("../../testdata/holesky_witness/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec = get_chain_spec(Chain::from_id(witness.chain_id)).unwrap();
        crate::verifier::run(
            &[witness],
            chain_spec,
            crate::verifier::StateCommitMode::Block,
        )
        .unwrap();
    }
}
