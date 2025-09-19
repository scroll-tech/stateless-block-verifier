use crate::{EvmDatabase, VerificationError};
use sbv_kv::KeyValueStoreGet;
use sbv_primitives::{
    B256, Bytes,
    chainspec::ChainSpec,
    types::{
        reth::{
            evm::{ConfigureEvm, EthEvmConfig, execute::Executor},
            execution_types::BlockExecutionOutput,
            primitives::{Block, Receipt, RecoveredBlock},
        },
        revm::database::CacheDB,
    },
};
use std::sync::Arc;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<'a, CodeDb, BlockHashProvider> {
    chain_spec: Arc<ChainSpec>,
    db: &'a EvmDatabase<CodeDb, BlockHashProvider>,
    block: &'a RecoveredBlock<Block>,
}

impl<'a, CodeDb, BlockHashProvider> EvmExecutor<'a, CodeDb, BlockHashProvider> {
    /// Create a new EVM executor
    pub fn new(
        chain_spec: Arc<ChainSpec>,
        db: &'a EvmDatabase<CodeDb, BlockHashProvider>,
        block: &'a RecoveredBlock<Block>,
    ) -> Self {
        Self {
            chain_spec,
            db,
            block,
        }
    }
}

impl<CodeDb: KeyValueStoreGet<B256, Bytes>, BlockHashProvider: KeyValueStoreGet<u64, B256>>
    EvmExecutor<'_, CodeDb, BlockHashProvider>
{
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutput<Receipt>, VerificationError> {
        let provider = EthEvmConfig::new(self.chain_spec.clone());

        let output = cycle_track!(
            provider.executor(CacheDB::new(self.db)).execute(self.block),
            "handle_block"
        )?;

        Ok(output)
    }
}
