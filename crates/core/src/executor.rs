use crate::{database::EvmDatabase, error::VerificationError};
use reth_evm::{
    ConfigureEvm, ConfigureEvmEnv, Evm,
    execute::{
        BlockExecutionError, BlockExecutionStrategy, BlockExecutionStrategyFactory,
        BlockExecutorProvider, BlockValidationError, Executor,
    },
};
#[cfg(not(feature = "scroll"))]
use reth_evm_ethereum::execute::EthExecutorProvider as ExecutorProvider;
use reth_execution_types::BlockExecutionOutput;
#[cfg(feature = "scroll")]
use reth_scroll_evm::ScrollExecutorProvider as ExecutorProvider;
use reth_scroll_evm::{
    BasicScrollReceiptBuilder, ReceiptBuilderCtx, ScrollEvmConfig, ScrollExecutionStrategyFactory,
    ScrollReceiptBuilder,
};
use sbv_kv::KeyValueStoreGet;
use sbv_primitives::{
    B256, Bytes, RecoveredBlock, U256,
    chainspec::ChainSpec,
    revm::{
        Database, DatabaseCommit, EvmContext, Inspector,
        db::CacheDB,
        inspectors::CustomPrintTracer,
        interpreter::Interpreter,
        primitives::{ExecutionResult, ResultAndState},
    },
    types::{
        consensus::SignedTransaction,
        reth::{Block, Receipt, ScrollPrimitives},
    },
};
use sbv_trie::TrieNode;
use std::{fmt::Debug, sync::Arc};

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
        // #[cfg(not(feature = "scroll"))]
        // let provider = ExecutorProvider::ethereum(self.chain_spec.clone());
        // #[cfg(feature = "scroll")]
        // let provider = ExecutorProvider::scroll(self.chain_spec.clone());
        //
        // #[allow(clippy::let_and_return)]
        // let output = measure_duration_millis!(
        //     handle_block_duration_milliseconds,
        //     cycle_track!(
        //         provider.executor(CacheDB::new(self.db)).execute(self.block),
        //         "handle_block"
        //     )
        // )?;
        //
        // #[cfg(feature = "metrics")]
        // sbv_helpers::metrics::REGISTRY.block_counter.inc();

        Ok(self.execute_raw()?)
    }

    fn execute_raw(self) -> Result<BlockExecutionOutput<Receipt>, BlockExecutionError> {
        let receipt_builder = BasicScrollReceiptBuilder::default();

        let evm_config = ScrollEvmConfig::new(self.chain_spec.clone());
        let strategy_factory = ScrollExecutionStrategyFactory::<ScrollPrimitives, _>::new(
            evm_config.clone(),
            receipt_builder,
        );
        let mut strategy = strategy_factory.create_strategy(CacheDB::new(self.db));
        strategy.apply_pre_execution_changes(&self.block)?;

        let mut evm = evm_config.evm_with_env_and_inspector(
            strategy.state_mut(),
            evm_config.evm_env(&self.block.header()),
            Tracer,
        );

        let mut cumulative_gas_used = 0;
        let mut receipts = Vec::with_capacity(self.block.body().transactions.len());

        for (sender, transaction) in self.block.transactions_with_sender() {
            let tx_env = evm_config.tx_env(transaction, *sender);
            // disable the base fee checks for l1 messages.
            evm.context.evm.inner.env.cfg.disable_base_fee = transaction.is_l1_message();

            // execute the transaction and commit the result to the database
            let ResultAndState { result, state } =
                evm.transact(tx_env)
                    .map_err(|err| BlockValidationError::EVM {
                        hash: transaction.recalculate_hash(),
                        error: Box::new(err),
                    })?;
            evm.db_mut().commit(state);

            let l1_fee = if transaction.is_l1_message() {
                // l1 messages do not get any gas refunded
                if let ExecutionResult::Success { gas_refunded, .. } = result {
                    cumulative_gas_used += gas_refunded
                }

                U256::ZERO
            } else {
                // compute l1 fee for all non-l1 transaction
                let l1_block_info = evm.context.evm.inner.l1_block_info.as_ref().unwrap();
                let transaction_rlp_bytes =
                    evm.context.evm.env.tx.scroll.rlp_bytes.as_ref().unwrap();
                l1_block_info.calculate_tx_l1_cost(transaction_rlp_bytes, evm.handler.cfg.spec_id)
            };

            cumulative_gas_used += result.gas_used();

            let ctx = ReceiptBuilderCtx {
                header: self.block.header(),
                tx: transaction,
                result,
                cumulative_gas_used,
                l1_fee,
            };
            receipts.push(receipt_builder.build_receipt(ctx))
        }
        drop(evm);

        let requests = strategy.apply_post_execution_changes(&self.block, &receipts)?;
        let state = strategy.finish();

        // #[allow(clippy::let_and_return)]
        // let output = measure_duration_millis!(
        //     handle_block_duration_milliseconds,
        //     cycle_track!(
        //         provider.executor(CacheDB::new(self.db)).execute(self.block),
        //         "handle_block"
        //     )
        // )?;

        #[cfg(feature = "metrics")]
        sbv_helpers::metrics::REGISTRY.block_counter.inc();

        Ok(BlockExecutionOutput {
            state,
            receipts,
            requests,
            gas_used: cumulative_gas_used,
        })
    }
}

struct Tracer;

impl<DB: Database> Inspector<DB> for Tracer {
    fn step(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        panic!("1");
    }
}
