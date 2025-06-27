use crate::{B256, BlockWitness};
use itertools::Itertools;

#[cfg(feature = "scroll-reth-primitives-types")]
pub mod reth;

/// BlockWitnessCodeExt trait
pub trait BlockWitnessChunkExt {
    /// Get the chain id.
    fn chain_id(&self) -> crate::ChainId;
    /// Get the previous state root.
    fn prev_state_root(&self) -> B256;
    /// Check if all witnesses have the same chain id.
    fn has_same_chain_id(&self) -> bool;
    /// Check if all witnesses have a sequence block number.
    fn has_seq_block_number(&self) -> bool;
}

/// Helper trait for hashing transaction bytes.
pub trait TxBytesHashExt {
    /// Hash the transaction bytes.
    ///
    /// Only L2 transactions are considered while computing the digest.
    fn tx_bytes_hash(self) -> (usize, B256);

    /// Hash the transaction bytes.
    ///
    /// Only L2 transactions are considered while computing the digest.
    fn tx_bytes_hash_in(self, rlp_buffer: &mut Vec<u8>) -> (usize, B256);
}

/// Chunk related extension methods for Block
pub trait BlockChunkExt {
    /// Hash the header of the block
    fn legacy_hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher);
    /// Hash the l1 messages of the block
    fn legacy_hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher);
    /// Hash the l1 messages of the block
    fn hash_msg_queue(&self, initial_queue_hash: &B256) -> B256;
    /// Number of L1 msg txs in the block
    fn num_l1_msgs(&self) -> usize;
}

impl<T: BlockWitness> BlockWitnessChunkExt for [T] {
    #[inline(always)]
    fn chain_id(&self) -> crate::ChainId {
        debug_assert!(self.has_same_chain_id(), "chain id mismatch");
        self.first().expect("empty witnesses").chain_id()
    }
    #[inline(always)]
    fn prev_state_root(&self) -> B256 {
        self.first().expect("empty witnesses").pre_state_root()
    }
    #[inline(always)]
    fn has_same_chain_id(&self) -> bool {
        self.iter()
            .tuple_windows()
            .all(|(a, b)| a.chain_id() == b.chain_id())
    }
    #[inline(always)]
    fn has_seq_block_number(&self) -> bool {
        self.iter()
            .tuple_windows()
            .all(|(a, b)| a.number() + 1 == b.number())
    }
}
