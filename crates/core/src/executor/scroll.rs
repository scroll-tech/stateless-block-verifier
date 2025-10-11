use crate::database::WitnessDatabase;
use sbv_primitives::{
    U256,
    chainspec::ChainSpec,
    types::reth::{
        evm::{ConfigureEvm, EthEvmConfig, block::BlockExecutionError},
        execution_types::BlockExecutionOutput,
        primitives::{Block, Receipt, RecoveredBlock},
    },
};
use std::sync::Arc;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<'a> {
    chain_spec: Arc<ChainSpec>,
    db: WitnessDatabase<'a>,
    block: &'a RecoveredBlock<Block>,
    compression_ratios: Option<Vec<U256>>,
}

impl<'a> EvmExecutor<'a> {
    /// Create a new EVM executor
    pub fn new(
        chain_spec: Arc<ChainSpec>,
        db: WitnessDatabase<'a>,
        block: &'a RecoveredBlock<Block>,
        compression_ratios: Option<Vec<U256>>,
    ) -> Self {
        Self {
            chain_spec,
            db,
            block,
            compression_ratios,
        }
    }
}

impl EvmExecutor<'_> {
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutput<Receipt>, BlockExecutionError> {
        use sbv_primitives::types::{
            evm::ScrollBlockExecutor,
            reth::evm::execute::BlockExecutor,
            revm::database::{State, states::bundle_state::BundleRetention},
        };

        let provider = EthEvmConfig::scroll(self.chain_spec.clone());
        let factory = provider.block_executor_factory();

        let mut db = State::builder()
            .with_database(self.db)
            .with_bundle_update()
            .without_state_clear()
            .build();

        let evm = provider
            .evm_for_block(&mut db, self.block.header())
            .expect("infallible");
        let ctx = provider.context_for_block(self.block).expect("infallible");
        let executor =
            ScrollBlockExecutor::new(evm, ctx, factory.spec(), factory.receipt_builder());

        let result = cycle_track!(
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
        )?;
        db.merge_transitions(BundleRetention::Reverts);

        Ok(BlockExecutionOutput {
            result,
            state: db.take_bundle(),
        })
    }
}
