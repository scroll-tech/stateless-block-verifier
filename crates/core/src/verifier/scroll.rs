use crate::{
    BlockWitness,
    verifier::{VerifyResult, run},
};
use reth_stateless::{StatelessTrie, validation::StatelessValidationError};
use sbv_primitives::{
    Address, B256, U256, chainspec::ChainSpec, types::reth::evm::execute::ProviderError,
};
use sbv_trie::r0::SparseState;
use std::sync::Arc;

/// State commit mode for the block witness verification process.
#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Verify the block witness and return the gas used.
pub fn run_host(
    witnesses: &[BlockWitness],
    chain_spec: Arc<ChainSpec>,
) -> Result<VerifyResult, StatelessValidationError> {
    let compression_ratios = witnesses
        .iter()
        .map(|block| block.compression_ratios())
        .collect::<Vec<_>>();
    run(witnesses, chain_spec, compression_ratios)
}

/// Get the withdrawal trie root of scroll.
///
/// Note: this should not be confused with the withdrawal of the beacon chain.
pub(super) fn withdraw_root(state: &SparseState) -> Result<B256, ProviderError> {
    /// L2MessageQueue pre-deployed address
    pub const ADDRESS: Address =
        sbv_primitives::address!("5300000000000000000000000000000000000000");
    /// the slot of withdraw root in L2MessageQueue
    pub const WITHDRAW_TRIE_ROOT_SLOT: U256 = U256::ZERO;

    state
        .account(ADDRESS)?
        .expect("L2MessageQueue contract not found");
    let withdraw_root = state.storage(ADDRESS, WITHDRAW_TRIE_ROOT_SLOT)?;
    Ok(withdraw_root.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbv_primitives::{
        chainspec::{Chain, build_chain_spec_force_hardfork},
        hardforks::Hardfork,
    };

    #[rstest::rstest]
    fn test_euclid_v2(
        #[files("../../testdata/scroll/euclidv2/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::EuclidV2);
        run_host(&[witness], chain_spec).unwrap();
    }

    #[rstest::rstest]
    fn test_feynman(
        #[files("../../testdata/scroll/feynman/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::Feynman);
        run_host(&[witness], chain_spec).unwrap();
    }
}
