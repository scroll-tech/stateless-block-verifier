use crate::{database::EvmDatabase, error::VerificationError};
use reth_evm::{ConfigureEvm, execute::Executor};
use reth_execution_types::BlockExecutionOutput;
use sbv_kv::KeyValueStoreGet;
use sbv_primitives::{
    B256, Bytes,
    chainspec::ChainSpec,
    types::{
        reth::{Block, EthPrimitives, Receipt, RecoveredBlock, evm},
        revm::database::CacheDB,
    },
};
use sbv_trie::TrieNode;
use std::{fmt::Debug, sync::Arc};

#[cfg(not(feature = "scroll"))]
pub type ExecutorProvider = EthEvmConfig;

#[cfg(feature = "scroll")]
pub type ExecutorProvider = evm::EthEvmConfig<
    ChainSpec,
    EthPrimitives,
    evm::RethReceiptBuilder,
    sbv_precompile::PrecompileProvider,
>;

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
        let provider = ExecutorProvider::new(self.chain_spec.clone(), Default::default());

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
