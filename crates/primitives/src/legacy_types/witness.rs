use crate::{
    B256, Bytes, ChainId,
    legacy_types::{BlockHeader, Transaction, Withdrawal},
};

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
