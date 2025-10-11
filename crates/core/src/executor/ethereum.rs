use crate::database::WitnessDatabase;
use sbv_primitives::{
    chainspec::ChainSpec,
    types::reth::{
        evm::{ConfigureEvm, EthEvmConfig, block::BlockExecutionError, execute::Executor},
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
}

impl<'a> crate::EvmExecutor<'a> {
    /// Create a new EVM executor
    pub fn new(
        chain_spec: Arc<ChainSpec>,
        db: WitnessDatabase<'a>,
        block: &'a RecoveredBlock<Block>,
    ) -> Self {
        Self {
            chain_spec,
            db,
            block,
        }
    }
}

impl EvmExecutor<'_> {
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutput<Receipt>, BlockExecutionError> {
        let provider = EthEvmConfig::new(self.chain_spec.clone());

        let output = cycle_track!(
            provider.executor(self.db).execute(self.block),
            "handle_block"
        )?;

        Ok(output)
    }
}
