//! Verifier helper functions for stateless block verification.
use crate::{EvmDatabase, EvmExecutor, VerificationError};
use sbv_kv::nohash::NoHashMap;
use sbv_primitives::{
    BlockWitness,
    chainspec::ChainSpec,
    ext::{BlockWitnessExt, BlockWitnessRethExt},
};
use sbv_trie::BlockWitnessTrieExt;
use std::{collections::BTreeMap, sync::Arc};

/// Verify a block witness.
#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.header.number), err))]
pub fn verify(
    witness: &BlockWitness,
    chain_spec: Arc<ChainSpec>,
) -> Result<u64, VerificationError> {
    dev_trace!("{witness:#?}");

    let mut code_db = NoHashMap::default();
    witness.import_codes(&mut code_db);
    let mut nodes_provider = NoHashMap::default();
    witness.import_nodes(&mut nodes_provider).unwrap();
    #[cfg(not(feature = "scroll"))]
    let block_hashes = {
        let mut block_hashes = NoHashMap::default();
        witness.import_block_hashes(&mut block_hashes);
        block_hashes
    };
    #[cfg(feature = "scroll")]
    let block_hashes = &sbv_kv::null::NullProvider;
    let mut db = EvmDatabase::new_from_root(
        code_db,
        witness.pre_state_root,
        &nodes_provider,
        &block_hashes,
    )?;

    let block = witness.build_reth_block()?;

    #[cfg(not(feature = "scroll"))]
    let executor = EvmExecutor::new(chain_spec, &db, &block);
    #[cfg(feature = "scroll")]
    let executor = EvmExecutor::new(chain_spec, &db, &block, None::<Vec<sbv_primitives::U256>>);

    let output = executor.execute().inspect_err(|_e| {
        dev_error!(
            "Error occurs when executing block #{}: {_e:?}",
            block.number
        );
    })?;

    db.update(
        &nodes_provider,
        BTreeMap::from_iter(output.state.state.clone()).iter(),
    )?;
    let post_state_root = db.commit_changes();

    if block.state_root != post_state_root {
        dev_error!(
            "Block #{} root mismatch: root after in trace = {:x}, root after in reth = {:x}",
            block.number,
            block.state_root,
            post_state_root
        );

        return Err(VerificationError::root_mismatch(
            block.state_root,
            post_state_root,
            output.state,
        ));
    }
    dev_info!("Block #{} verified successfully", block.number);

    Ok(output.gas_used)
}

#[cfg(test)]
#[cfg(feature = "scroll-hardforks")]
mod tests {
    use super::*;
    use rstest::rstest;
    use sbv_primitives::{
        chainspec::{Chain, ForkCondition, SCROLL_DEV},
        hardforks::ScrollHardfork,
    };

    fn get_chain_spec_euclid_v1(chain_id: u64) -> Arc<ChainSpec> {
        let mut spec = (**SCROLL_DEV).clone();
        spec.inner.chain = Chain::from_id(chain_id);
        spec.inner
            .hardforks
            .insert(ScrollHardfork::Euclid, ForkCondition::Timestamp(0));
        spec.inner
            .hardforks
            .insert(ScrollHardfork::EuclidV2, ForkCondition::Never);
        spec.inner
            .hardforks
            .insert(ScrollHardfork::Feynman, ForkCondition::Never);

        Arc::new(spec)
    }

    fn get_chain_spec_euclid_v2(chain_id: u64) -> Arc<ChainSpec> {
        let mut spec = (**SCROLL_DEV).clone();
        spec.inner.chain = Chain::from_id(chain_id);
        spec.inner
            .hardforks
            .insert(ScrollHardfork::EuclidV2, ForkCondition::Timestamp(0));
        spec.inner
            .hardforks
            .insert(ScrollHardfork::Feynman, ForkCondition::Never);

        Arc::new(spec)
    }

    fn get_chain_spec_feynman(chain_id: u64) -> Arc<ChainSpec> {
        let mut spec = (**SCROLL_DEV).clone();
        spec.inner.chain = Chain::from_id(chain_id);
        spec.inner
            .hardforks
            .insert(ScrollHardfork::EuclidV2, ForkCondition::Timestamp(0));
        spec.inner
            .hardforks
            .insert(ScrollHardfork::Feynman, ForkCondition::Timestamp(0));

        Arc::new(spec)
    }

    #[rstest]
    fn test_euclid_v1(
        #[files("../../testdata/scroll_witness/euclid_v1/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = BlockWitness::from_json_str(witness_json).unwrap();
        let chain_spec = get_chain_spec_euclid_v1(witness.chain_id);
        verify(&witness, chain_spec).unwrap();
    }

    #[rstest]
    fn test_euclid_v2(
        #[files("../../testdata/scroll_witness/euclid_v2/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = BlockWitness::from_json_str(witness_json).unwrap();
        let chain_spec = get_chain_spec_euclid_v2(witness.chain_id);
        verify(&witness, chain_spec).unwrap();
    }

    #[rstest]
    fn test_feynman(
        #[files("../../testdata/scroll_witness/feynman/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = BlockWitness::from_json_str(witness_json).unwrap();
        let chain_spec = get_chain_spec_feynman(witness.chain_id);
        verify(&witness, chain_spec).unwrap();
    }
}
