use crate::{
    B256, BlockNumber, Bytes, ChainId, U256,
    alloy_primitives::map::B256HashMap,
    types::{BlockHeader, Transaction, Withdrawal},
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
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockWitness {
    /// Chain id
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Chain id")))]
    pub chain_id: ChainId,
    /// Block header representation.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Block header representation")))]
    pub header: BlockHeader,
    /// State trie root before the block.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "State trie root before the block")))]
    pub pre_state_root: B256,
    /// Transactions in the block.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Transactions in the block")))]
    pub transaction: Vec<Transaction>,
    /// Withdrawals in the block.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Withdrawals in the block")))]
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Last 256 Ancestor block hashes.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Ancestor block hashes")))]
    #[cfg(not(feature = "scroll"))]
    pub block_hashes: Vec<B256>,
    /// Rlp encoded state trie nodes.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Rlp encoded state trie nodes")))]
    pub states: Vec<Bytes>,
    /// Code bytecodes
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Code bytecodes")))]
    pub codes: Vec<Bytes>,
}

impl BlockWitness {
    /// Calculates compression ratios for all transactions in the block witness.
    ///
    /// # Panics
    ///
    /// Panics if called without the "scroll-compress-ratio" feature enabled, as this
    /// functionality is not intended to be used in guest environments.
    pub fn compression_ratios(&self) -> Vec<U256> {
        #[cfg(feature = "scroll-compress-ratio")]
        {
            self.transaction
                .iter()
                .map(|tx| crate::types::evm::compute_compression_ratio(&tx.input))
                .collect()
        }
        #[cfg(not(feature = "scroll-compress-ratio"))]
        {
            unimplemented!("you should not build ChunkWitness in guest?");
        }
    }
}

impl crate::BlockWitness for BlockWitness {
    fn chain_id(&self) -> ChainId {
        self.chain_id
    }
    fn number(&self) -> BlockNumber {
        self.header.number
    }
    fn pre_state_root(&self) -> B256 {
        self.pre_state_root
    }
    fn post_state_root(&self) -> B256 {
        self.header.state_root
    }
    fn withdrawals_root(&self) -> Option<B256> {
        self.header.withdrawals_root
    }
    fn num_transactions(&self) -> usize {
        self.transaction.len()
    }
    #[cfg(not(feature = "scroll"))]
    fn block_hashes_iter(&self) -> impl ExactSizeIterator<Item = B256> {
        self.block_hashes.iter().copied()
    }
    fn withdrawals_iter(&self) -> Option<impl ExactSizeIterator<Item = impl crate::Withdrawal>> {
        self.withdrawals.as_ref().map(|w| w.iter())
    }
    fn states_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>> {
        self.states.iter().map(|s| s.as_ref())
    }
    fn codes_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>> {
        self.codes.iter().map(|c| c.as_ref())
    }
}

#[cfg(feature = "rkyv")]
impl crate::BlockWitness for ArchivedBlockWitness {
    fn chain_id(&self) -> ChainId {
        self.chain_id.to_native()
    }
    fn number(&self) -> BlockNumber {
        self.header.number.to_native()
    }
    fn pre_state_root(&self) -> B256 {
        self.pre_state_root.into()
    }
    fn post_state_root(&self) -> B256 {
        self.header.state_root.into()
    }
    fn withdrawals_root(&self) -> Option<B256> {
        self.header.withdrawals_root.as_ref().map(|x| x.0.into())
    }
    fn num_transactions(&self) -> usize {
        self.transaction.len()
    }
    #[cfg(not(feature = "scroll"))]
    fn block_hashes_iter(&self) -> impl ExactSizeIterator<Item = B256> {
        self.block_hashes.iter().map(|h| B256::from(h.0))
    }
    fn withdrawals_iter(&self) -> Option<impl ExactSizeIterator<Item = impl crate::Withdrawal>> {
        self.withdrawals.as_ref().map(|w| w.iter())
    }
    fn states_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>> {
        self.states.iter().map(|s| s.as_ref())
    }
    fn codes_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>> {
        self.codes.iter().map(|c| c.as_ref())
    }
}
