use crate::{
    B256, BlockHeader, Bytes, ChainId, Transaction, Withdrawals, alloy_primitives::map::B256HashMap,
};

/// Represents the execution witness of a block. Contains an optional map of state preimages.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutionWitness {
    /// Map of all hashed trie nodes to their preimages that were required during the execution of
    /// the block, including during state root recomputation.
    ///
    /// `keccak(rlp(node)) => rlp(node)`
    pub state: B256HashMap<Bytes>,
    /// Map of all contract codes (created / accessed) to their preimages that were required during
    /// the execution of the block, including during state root recomputation.
    ///
    /// `keccak(bytecodes) => bytecodes`
    pub codes: B256HashMap<Bytes>,
}

/// Witness for a block.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockWitness {
    /// Chain id
    pub chain_id: ChainId,
    /// Block header representation.
    pub header: BlockHeader,
    /// State trie root before the block.
    pub pre_state_root: B256,
    /// Transactions in the block.
    pub transaction: Vec<Transaction>,
    /// Withdrawals in the block.
    pub withdrawals: Option<Withdrawals>,
    /// Last 256 Ancestor block hashes.
    #[cfg(not(feature = "scroll"))]
    pub block_hashes: Vec<B256>,
    /// Rlp encoded state trie nodes.
    pub states: Vec<Bytes>,
    /// Code bytecodes
    pub codes: Vec<Bytes>,
}

impl BlockWitness {
    /// Calculates compression ratios for all transactions in the block witness.
    #[cfg(feature = "scroll-compress-ratio")]
    pub fn compression_ratios(&self) -> Vec<crate::U256> {
        self.transaction
            .iter()
            .map(|tx| crate::evm::compute_compression_ratio(&tx.input))
            .collect()
    }
}
