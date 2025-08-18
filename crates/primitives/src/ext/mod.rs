use crate::{B256, BlockWitness, Bytes};
use itertools::Itertools;
use sbv_kv::KeyValueStore;

mod imps;
#[cfg(feature = "reth-primitives-types")]
mod reth;
#[cfg(feature = "reth-primitives-types")]
pub use reth::BlockWitnessRethExt;

/// BlockWitnessExt trait
pub trait BlockWitnessExt {
    /// Import codes into code db
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, code_db: CodeDb);
    /// Import block hashes into block hash provider
    #[cfg(not(feature = "scroll"))]
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        block_hashes: BlockHashProvider,
    );
}

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
