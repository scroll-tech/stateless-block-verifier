use crate::EvmExecutor;
use std::fmt::{Debug, Formatter};

/// Transaction RLP handler.
pub type TxRLPHandler = dyn Fn(&EvmExecutor, &[u8]) + 'static;
/// Post transaction execution handler.
pub type PostTxExecutionHandler = dyn Fn(&EvmExecutor, usize) + 'static;

/// Hooks for the EVM executor.
#[derive(Default)]
pub struct ExecuteHooks {
    tx_rlp_handlers: Vec<Box<TxRLPHandler>>,
    post_tx_execution_handlers: Vec<Box<PostTxExecutionHandler>>,
}

impl ExecuteHooks {
    /// Create a new hooks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a transaction RLP handler.
    pub fn add_tx_rlp_handler<F>(&mut self, handler: F)
    where
        F: Fn(&EvmExecutor, &[u8]) + 'static,
    {
        self.tx_rlp_handlers.push(Box::new(handler));
    }

    /// Add a post transaction execution handler.
    pub fn add_post_tx_execution_handler<F>(&mut self, handler: F)
    where
        F: Fn(&EvmExecutor, usize) + 'static,
    {
        self.post_tx_execution_handlers.push(Box::new(handler));
    }

    /// Execute transaction RLP handlers.
    pub(crate) fn tx_rlp(&self, executor: &EvmExecutor, rlp: &[u8]) {
        for handler in &self.tx_rlp_handlers {
            handler(executor, rlp);
        }
    }

    pub(crate) fn post_tx_execution(&self, executor: &EvmExecutor, tx_index: usize) {
        for handler in &self.post_tx_execution_handlers {
            handler(executor, tx_index);
        }
    }
}

impl Debug for ExecuteHooks {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteHooks")
            .field(
                "post_tx_execution_handlers",
                &self.post_tx_execution_handlers.len(),
            )
            .finish()
    }
}
