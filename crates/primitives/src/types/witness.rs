use crate::types::block_header::ToHelper as _;
use crate::types::{BlockHeader, Transaction, Withdrawal};
use alloy_primitives::map::B256HashMap;
use alloy_primitives::{Bytes, ChainId, B256};
use alloy_rpc_types_eth::Block;
use reth_primitives::TransactionSigned;

/// Represents the execution witness of a block. Contains an optional map of state preimages.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub struct BlockWitness {
    /// Chain id
    #[rkyv(attr(doc = "Chain id"))]
    pub chain_id: ChainId,
    /// Block header representation.
    #[rkyv(attr(doc = "Block header representation"))]
    pub header: BlockHeader,
    /// State trie root before the block.
    #[rkyv(attr(doc = "State trie root before the block"))]
    pub pre_state_root: B256,
    /// Transactions in the block.
    #[rkyv(attr(doc = "Transactions in the block"))]
    pub transaction: Vec<Transaction>,
    /// Withdrawals in the block.
    #[rkyv(attr(doc = "Withdrawals in the block"))]
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Last 256 Ancestor block hashes.
    #[rkyv(attr(doc = "Ancestor block hashes"))]
    #[cfg(not(feature = "scroll"))]
    pub block_hashes: Vec<B256>,
    /// Rlp encoded state trie nodes.
    #[rkyv(attr(doc = "Rlp encoded state trie nodes"))]
    pub states: Vec<Bytes>,
    /// Code bytecodes
    #[rkyv(attr(doc = "Code bytecodes"))]
    pub codes: Vec<Bytes>,
}

impl BlockWitness {
    /// Creates a new block witness from a block, pre-state root, execution witness.
    pub fn new_from_block(
        chain_id: ChainId,
        block: Block,
        pre_state_root: B256,
        block_hashes: Vec<B256>,
        witness: ExecutionWitness,
    ) -> Self {
        let header = BlockHeader::from(block.header);
        let transaction = block
            .transactions
            .into_transactions()
            .map(Transaction::from_alloy)
            .collect();
        let withdrawals = block
            .withdrawals
            .map(|w| w.iter().map(Withdrawal::from).collect());
        let states = witness.state.into_values().collect();
        let codes = witness.codes.into_values().collect();
        Self {
            chain_id,
            header,
            transaction,
            block_hashes,
            withdrawals,
            pre_state_root,
            states,
            codes,
        }
    }
}

impl crate::BlockWitness for BlockWitness {
    fn chain_id(&self) -> ChainId {
        self.chain_id
    }
    fn header(&self) -> impl alloy_consensus::BlockHeader {
        &self.header
    }
    fn build_alloy_header(&self) -> alloy_consensus::Header {
        self.header.to_alloy()
    }
    fn pre_state_root(&self) -> B256 {
        self.pre_state_root
    }
    fn num_transactions(&self) -> usize {
        self.transaction.len()
    }
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, alloy_primitives::SignatureError>>
    {
        self.transaction.iter().map(|tx| tx.try_into())
    }
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

impl crate::BlockWitness for ArchivedBlockWitness {
    fn chain_id(&self) -> ChainId {
        self.chain_id.to_native()
    }
    fn header(&self) -> impl alloy_consensus::BlockHeader {
        &self.header
    }
    fn build_alloy_header(&self) -> alloy_consensus::Header {
        self.header.to_alloy()
    }
    fn pre_state_root(&self) -> B256 {
        self.pre_state_root.into()
    }
    fn num_transactions(&self) -> usize {
        self.transaction.len()
    }
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, alloy_primitives::SignatureError>>
    {
        self.transaction.iter().map(|tx| tx.try_into())
    }
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
