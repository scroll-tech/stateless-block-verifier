use crate::{EvmDatabase, VerificationError};
use sbv_kv::KeyValueStoreGet;
use sbv_primitives::{
    B256, Bytes, U256,
    chainspec::ChainSpec,
    reth::{
        evm::{ConfigureEvm, EthEvmConfig, RethReceiptBuilder},
        execution_types::BlockExecutionOutput,
        primitives::{Block, EthPrimitives, Receipt, RecoveredBlock},
    },
    revm::database::CacheDB,
};
use sbv_trie::TrieNode;
use std::sync::Arc;

/// Ethereum-related EVM configuration.
pub type EvmConfig =
    EthEvmConfig<ChainSpec, EthPrimitives, RethReceiptBuilder, sbv_precompile::PrecompileProvider>;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<'a, CodeDb, NodesProvider, BlockHashProvider, CompressionRatios> {
    chain_spec: Arc<ChainSpec>,
    db: &'a EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>,
    block: &'a RecoveredBlock<Block>,
    compression_ratios: Option<CompressionRatios>,
}

impl<'a, CodeDb, NodesProvider, BlockHashProvider, CompressionRatios>
    EvmExecutor<'a, CodeDb, NodesProvider, BlockHashProvider, CompressionRatios>
{
    /// Create a new EVM executor
    pub fn new(
        chain_spec: Arc<ChainSpec>,
        db: &'a EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>,
        block: &'a RecoveredBlock<Block>,
        compression_ratios: Option<CompressionRatios>,
    ) -> Self {
        Self {
            chain_spec,
            db,
            block,
            compression_ratios,
        }
    }
}

impl<
    CodeDb: KeyValueStoreGet<B256, Bytes>,
    NodesProvider: KeyValueStoreGet<B256, TrieNode>,
    BlockHashProvider: KeyValueStoreGet<u64, B256>,
    CompressionRatios: IntoIterator<Item = U256>,
> EvmExecutor<'_, CodeDb, NodesProvider, BlockHashProvider, CompressionRatios>
{
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutput<Receipt>, VerificationError> {
        use sbv_primitives::{
            evm::ScrollBlockExecutor,
            reth::evm::execute::BlockExecutor,
            revm::database::{State, states::bundle_state::BundleRetention},
        };

        let provider = EvmConfig::new(self.chain_spec.clone(), Default::default());
        let factory = provider.block_executor_factory();

        let mut db = State::builder()
            .with_database(CacheDB::new(self.db))
            .with_bundle_update()
            .without_state_clear()
            .build();

        let evm = provider.evm_for_block(&mut db, self.block.header());
        let ctx = provider.context_for_block(self.block);
        let executor =
            ScrollBlockExecutor::new(evm, ctx, factory.spec(), factory.receipt_builder());

        let result = measure_duration_millis!(
            handle_block_duration_milliseconds,
            cycle_track!(
                match self.compression_ratios {
                    None => {
                        executor.execute_block(self.block.transactions_recovered())
                    }
                    Some(compression_ratios) => executor.execute_block_with_compression_cache(
                        self.block.transactions_recovered(),
                        compression_ratios,
                    ),
                },
                "handle_block"
            )
        )?;
        db.merge_transitions(BundleRetention::Reverts);

        #[cfg(feature = "metrics")]
        sbv_helpers::metrics::REGISTRY.block_counter.inc();

        Ok(BlockExecutionOutput {
            result,
            state: db.take_bundle(),
        })
    }
}
