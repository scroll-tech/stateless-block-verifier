use crate::witness::{BlockWitnessChunkExt, BlockWitnessExt};
use crate::{BlockWitness, EvmDatabase, EvmExecutor, VerificationError};
use itertools::Itertools;
use sbv_kv::{nohash::NoHashMap, null::NullProvider};
use sbv_primitives::{
    B256, Bytes, U256,
    chainspec::ChainSpec,
    types::reth::primitives::{Block, RecoveredBlock},
};
use sbv_trie::PartialStateTrie;
use std::{collections::BTreeMap, sync::Arc};

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
    pub withdraw_root: B256,
    /// Gas used during the verification process.
    pub gas_used: u64,
}

/// Verify the block witness and return the gas used.
pub fn run_host(
    witnesses: &[BlockWitness],
    chain_spec: Arc<ChainSpec>,
) -> Result<VerifyResult, VerificationError> {
    let compression_ratios = witnesses
        .iter()
        .map(|block| block.compression_ratios())
        .collect::<Vec<_>>();
    let cached_trie = PartialStateTrie::new(
        witnesses[0].prev_state_root,
        witnesses.iter().flat_map(|w| w.states.iter()),
    );
    run(witnesses, chain_spec, compression_ratios, cached_trie)
}

/// Verify the block witness and return the gas used.
pub fn run(
    witnesses: &[BlockWitness],
    chain_spec: Arc<ChainSpec>,
    compression_ratios: Vec<Vec<U256>>,
    cached_trie: PartialStateTrie,
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
    if !witnesses.has_seq_state_root() {
        return Err(VerificationError::NonSequentialWitnesses);
    }

    let code_db = {
        // build code db
        let num_codes = witnesses.iter().map(|w| w.codes.len()).sum();
        let mut code_db =
            NoHashMap::<B256, Bytes>::with_capacity_and_hasher(num_codes, Default::default());
        witnesses.import_codes(&mut code_db);
        manually_drop_on_zkvm!(code_db)
    };

    let pre_state_root = witnesses[0].prev_state_root;
    let post_state_root = witnesses.last().unwrap().header.state_root;

    let blocks = witnesses
        .iter()
        .map(|w| {
            dev_trace!("{w:#?}");
            w.build_reth_block()
        })
        .collect::<Result<Vec<RecoveredBlock<Block>>, _>>()?;
    if !blocks
        .iter()
        .tuple_windows()
        .all(|(a, b)| a.hash() == b.header().parent_hash)
    {
        return Err(VerificationError::NonSequentialWitnesses);
    }

    let mut gas_used = 0;
    let mut db = EvmDatabase::new(code_db, cached_trie, NullProvider);

    let mut execute_block = |block, compression_ratio| -> Result<(), VerificationError> {
        let executor = EvmExecutor::new(chain_spec.clone(), &db, block, compression_ratio);
        let output = executor.execute()?;
        gas_used += output.gas_used;

        #[cfg(not(target_os = "zkvm"))]
        let state_for_debug = output.state.clone();

        let post_state_root = db.commit(BTreeMap::from_iter(output.state.state))?;
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
                #[cfg(not(target_os = "zkvm"))]
                state_for_debug,
            ));
        }

        Ok(())
    };

    for (block, compression_ratios) in blocks.iter().zip_eq(compression_ratios) {
        execute_block(
            block,
            Some(compression_ratios.into_iter().map(|u| u.into())),
        )?;
    }

    let withdraw_root = db.withdraw_root()?;

    Ok(VerifyResult {
        blocks,
        pre_state_root,
        post_state_root,
        withdraw_root,
        gas_used,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbv_primitives::{
        U256,
        chainspec::{Chain, build_chain_spec_force_hardfork},
        hardforks::Hardfork,
    };

    #[rstest::rstest]
    fn test_euclid_v2(
        #[files("../../testdata/scroll_witness/euclidv2/**/*.json")]
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
        #[files("../../testdata/scroll_witness/feynman/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::Feynman);
        run_host(&[witness], chain_spec).unwrap();
    }
}
