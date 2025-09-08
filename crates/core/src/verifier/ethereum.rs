use crate::{
    BlockWitness, EvmDatabase, EvmExecutor, VerificationError,
    witness::{BlockWitnessChunkExt, BlockWitnessExt},
};
use itertools::Itertools;
use sbv_kv::nohash::NoHashMap;
use sbv_primitives::{
    B256, Bytes,
    chainspec::ChainSpec,
    types::reth::primitives::{Block, RecoveredBlock},
};
use sbv_trie::PartialStateTrie;
use std::{collections::BTreeMap, sync::Arc};

/// Result of the block witness verification process.
#[derive(Debug)]
pub struct VerifyResult {
    /// Recovered blocks from the witnesses.
    pub blocks: Vec<RecoveredBlock<Block>>,
    /// Pre-state root of the first block.
    pub pre_state_root: B256,
    /// Post-state root after executing the witnesses.
    pub post_state_root: B256,
    /// Gas used during the verification process.
    pub gas_used: u64,
}

/// Verify the block witness and return the gas used.
pub fn run_host(
    witnesses: &[BlockWitness],
    chain_spec: Arc<ChainSpec>,
) -> Result<VerifyResult, VerificationError> {
    let cached_trie = PartialStateTrie::new(
        witnesses[0].prev_state_root,
        witnesses.iter().flat_map(|w| w.states.iter()),
    )?;
    run(witnesses, chain_spec, cached_trie)
}

/// Verify the block witness and return the gas used.
pub fn run(
    witnesses: &[BlockWitness],
    chain_spec: Arc<ChainSpec>,
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

    let (code_db, block_hash_provider) = make_providers(witnesses);

    let pre_state_root = witnesses[0].prev_state_root;
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

    let mut db =
        manually_drop_on_zkvm!(EvmDatabase::new(code_db, cached_trie, block_hash_provider));

    let mut gas_used = 0;
    let mut post_state_root = B256::ZERO;
    for block in blocks.iter() {
        let executor = EvmExecutor::new(chain_spec.clone(), &db, block);
        let output = executor.execute()?;
        gas_used += output.gas_used;

        #[cfg(not(target_os = "zkvm"))]
        let state_for_debug = output.state.clone();

        post_state_root = db.commit(BTreeMap::from_iter(output.state.state.clone()))?;
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
        dev_info!("Block #{} verified successfully", block.number);
    }

    Ok(VerifyResult {
        blocks,
        pre_state_root,
        post_state_root,
        gas_used,
    })
}

type CodeDb = NoHashMap<B256, Bytes>;
type BlockHashProvider = NoHashMap<u64, B256>;

/// Create the providers needed for the EVM executor from a list of witnesses.
fn make_providers(witnesses: &[BlockWitness]) -> (CodeDb, BlockHashProvider) {
    let code_db = {
        // build code db
        let num_codes = witnesses.iter().map(|w| w.codes.len()).sum();
        let mut code_db =
            NoHashMap::<B256, Bytes>::with_capacity_and_hasher(num_codes, Default::default());
        witnesses.import_codes(&mut code_db);
        code_db
    };
    let block_hash_provider = {
        let num_blocks = witnesses.iter().map(|w| w.block_hashes.len()).sum();
        let mut block_hash_provider =
            NoHashMap::<u64, B256>::with_capacity_and_hasher(num_blocks, Default::default());
        witnesses.import_block_hashes(&mut block_hash_provider);
        block_hash_provider
    };

    (code_db, block_hash_provider)
}

// FIXME: fetch new traces
// #[cfg(test)]
// mod tests {
//     use sbv_primitives::{
//         chainspec::{Chain, get_chain_spec},
//         types::BlockWitness,
//     };
//
//     #[rstest::rstest]
//     fn test_mainnet(
//         #[files("../../../testdata/holesky_witness/**/*.json")]
//         #[mode = str]
//         witness_json: &str,
//     ) {
//         let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
//         let chain_spec = get_chain_spec(Chain::from_id(witness.chain_id)).unwrap();
//         crate::verifier::run(&[witness], chain_spec).unwrap();
//     }
// }
