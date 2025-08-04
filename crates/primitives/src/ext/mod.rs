use crate::{B256, Bytes};
use auto_impl::auto_impl;
use sbv_kv::KeyValueStore;

mod imps;
#[cfg(feature = "reth-primitives-types")]
mod reth;
#[cfg(feature = "reth-primitives-types")]
pub use reth::BlockWitnessRethExt;

#[cfg(feature = "scroll")]
mod scroll;
#[cfg(feature = "scroll")]
pub use scroll::{BlockChunkExt, BlockWitnessChunkExt, TxBytesHashExt};

/// BlockWitnessExt trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
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
