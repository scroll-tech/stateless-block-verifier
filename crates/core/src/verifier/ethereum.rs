use crate::{
    BlockWitness,
    verifier::{VerifyResult, run},
};
use reth_stateless::validation::StatelessValidationError;
use sbv_primitives::{B256, chainspec::ChainSpec};
use std::{collections::BTreeMap, sync::Arc};

/// Verify the block witness and return the gas used.
pub fn run_host(
    witnesses: &[BlockWitness],
    chain_spec: Arc<ChainSpec>,
) -> Result<VerifyResult, StatelessValidationError> {
    run(witnesses, chain_spec)
}

pub(super) fn import_block_hashes(witnesses: &[BlockWitness]) -> BTreeMap<u64, B256> {
    let mut block_hashes = BTreeMap::new();
    for witness in witnesses.iter() {
        let block_number = witness.header.number;
        for (i, hash) in witness.block_hashes.iter().enumerate() {
            let block_number = block_number
                .checked_sub(i as u64 + 1)
                .expect("block number underflow");
            block_hashes.insert(block_number, *hash);
        }
    }
    block_hashes
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbv_primitives::chainspec::{Chain, get_chain_spec};

    #[rstest::rstest]
    fn test_mainnet(
        #[files("../../testdata/ethereum/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec = get_chain_spec(Chain::from_id(witness.chain_id)).unwrap();
        crate::verifier::run(&[witness], chain_spec).unwrap();
    }
}
