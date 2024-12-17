use crate::{database::EvmDatabase, error::VerificationError};
use reth_evm::execute::{BlockExecutorProvider, Executor};
use reth_evm_ethereum::execute::EthExecutorProvider;
use reth_execution_types::BlockExecutionInput;
use revm::db::CacheDB;
use sbv_kv::KeyValueStore;
use sbv_primitives::{chainspec::ChainSpec, BlockWithSenders, Bytes, B256};
use sbv_trie::TrieNode;
use std::fmt::Debug;
use std::sync::Arc;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<'a, CodeDb, NodesProvider> {
    chain_spec: Arc<ChainSpec>,
    db: EvmDatabase<CodeDb, NodesProvider>,
    block: &'a BlockWithSenders,
}

/// Block execution result
#[derive(Debug)]
pub struct BlockExecutionOutcome {
    /// Gas used in this block
    pub gas_used: u64,
    /// State after
    pub post_state_root: B256,
    /// RLP bytes of transactions
    #[cfg(feature = "scroll")]
    pub tx_rlps: Vec<Bytes>,
}

impl<'a, CodeDb, NodesProvider> EvmExecutor<'a, CodeDb, NodesProvider> {
    /// Create a new EVM executor
    pub fn new(
        chain_spec: Arc<ChainSpec>,
        db: EvmDatabase<CodeDb, NodesProvider>,
        block: &'a BlockWithSenders,
    ) -> Self {
        Self {
            chain_spec,
            db,
            block,
        }
    }
}

impl<CodeDb: KeyValueStore<B256, Bytes>, NodesProvider: KeyValueStore<B256, TrieNode>>
    EvmExecutor<'_, CodeDb, NodesProvider>
{
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutcome, VerificationError> {
        #[allow(clippy::let_and_return)]
        let gas_used = measure_duration_millis!(
            handle_block_duration_milliseconds,
            cycle_track!(self.execute_inner(), "handle_block")
        )?;

        #[cfg(feature = "metrics")]
        sbv_helpers::metrics::REGISTRY.block_counter.inc();

        Ok(gas_used)
    }

    #[inline(always)]
    fn execute_inner(mut self) -> Result<BlockExecutionOutcome, VerificationError> {
        let input = BlockExecutionInput::new(self.block, self.block.header.difficulty);
        let output = EthExecutorProvider::ethereum(self.chain_spec.clone())
            .executor(CacheDB::new(&self.db))
            .execute(input)
            .unwrap();

        self.db
            .state
            .update(&self.db.nodes_provider, output.state.state.iter());
        let post_state_root = self.db.state.commit_state();

        Ok(BlockExecutionOutcome {
            gas_used: output.gas_used,
            post_state_root,
        })
    }
}
