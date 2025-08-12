//! Standard block witness verifier example.
use crate::{EvmDatabase, EvmExecutor, VerificationError};
use sbv_kv::nohash::NoHashMap;
use sbv_primitives::{
    chainspec::ChainSpec,
    ext::{BlockWitnessExt, BlockWitnessRethExt},
};
use sbv_trie::BlockWitnessTrieExt;
use std::{collections::BTreeMap, sync::Arc};

/// Verify the block witness and return the gas used.
#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.number()), err))]
pub fn run<T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt>(
    witness: T,
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
        witness.pre_state_root(),
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
mod tests {
    use super::*;
    use sbv_primitives::{
        chainspec::build_chain_spec_force_hardfork, hardforks::Hardfork, types::BlockWitness,
    };

    #[rstest::rstest]
    fn test_euclid_v1(
        #[files("../../testdata/scroll_witness/euclidv1/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec = build_chain_spec_force_hardfork(witness.chain_id, Hardfork::Euclid);
        run(&witness, chain_spec).unwrap();
    }

    #[rstest::rstest]
    fn test_euclid_v2(
        #[files("../../testdata/scroll_witness/euclidv2/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec = build_chain_spec_force_hardfork(witness.chain_id, Hardfork::EuclidV2);
        run(&witness, chain_spec).unwrap();
    }

    #[rstest::rstest]
    fn test_feynman(
        #[files("../../testdata/scroll_witness/feynman/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec = build_chain_spec_force_hardfork(witness.chain_id, Hardfork::Feynman);
        run(&witness, chain_spec).unwrap();
    }
}

#[cfg(test)]
#[cfg(not(feature = "scroll"))]
mod tests {
    use super::*;
    use sbv_primitives::chainspec::{Chain, get_chain_spec};

    #[rstest::rstest]
    fn test_mainnet(
        #[files("../../testdata/holesky_witness/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = BlockWitness::from_json_str(witness_json).unwrap();
        let chain_spec = get_chain_spec(Chain::from_id(witness.chain_id)).unwrap();
        run(&witness, chain_spec).unwrap();
    }
}
