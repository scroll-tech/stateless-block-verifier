use crate::{database::EvmDatabase, error::VerificationError};
use reth_evm::execute::{BlockExecutorProvider, Executor};
use reth_execution_types::BlockExecutionOutput;
use sbv_kv::KeyValueStoreGet;
use sbv_primitives::{
    B256, Bytes,
    chainspec::ChainSpec,
    types::{
        reth::{Block, Receipt, RecoveredBlock},
        revm::db::CacheDB,
    },
};
use sbv_trie::TrieNode;
use std::{fmt::Debug, sync::Arc};

#[cfg(not(feature = "scroll"))]
use reth_evm_ethereum::execute::EthExecutorProvider as ExecutorProvider;
#[cfg(feature = "scroll")]
use reth_scroll_evm::ScrollExecutorProvider as ExecutorProvider;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<'a, CodeDb, NodesProvider, BlockHashProvider> {
    chain_spec: Arc<ChainSpec>,
    db: &'a EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>,
    block: &'a RecoveredBlock<Block>,
}

impl<'a, CodeDb, NodesProvider, BlockHashProvider>
    EvmExecutor<'a, CodeDb, NodesProvider, BlockHashProvider>
{
    /// Create a new EVM executor
    pub fn new(
        chain_spec: Arc<ChainSpec>,
        db: &'a EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>,
        block: &'a RecoveredBlock<Block>,
    ) -> Self {
        Self {
            chain_spec,
            db,
            block,
        }
    }
}

impl<
    CodeDb: KeyValueStoreGet<B256, Bytes>,
    NodesProvider: KeyValueStoreGet<B256, TrieNode>,
    BlockHashProvider: KeyValueStoreGet<u64, B256>,
> EvmExecutor<'_, CodeDb, NodesProvider, BlockHashProvider>
{
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutput<Receipt>, VerificationError> {
        #[cfg(not(feature = "scroll"))]
        let provider = ExecutorProvider::ethereum(self.chain_spec.clone());
        #[cfg(feature = "scroll")]
        let provider = ExecutorProvider::scroll(self.chain_spec.clone());

        #[allow(clippy::let_and_return)]
        let output = measure_duration_millis!(
            handle_block_duration_milliseconds,
            cycle_track!(
                provider.executor(CacheDB::new(self.db)).execute(self.block),
                "handle_block"
            )
        )?;

        #[cfg(feature = "metrics")]
        sbv_helpers::metrics::REGISTRY.block_counter.inc();

        Ok(output)
    }
}
