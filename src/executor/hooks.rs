use crate::EvmExecutor;
use std::fmt::{Debug, Formatter};

/// Post transaction execution handler.
pub type PostTxExecutionHandler = dyn Fn(&EvmExecutor, usize) + Send + Sync + 'static;

/// Hooks for the EVM executor.
#[derive(Default)]
pub struct ExecuteHooks {
    post_tx_execution_handlers: Vec<Box<PostTxExecutionHandler>>,
}

impl ExecuteHooks {
    /// Create a new hooks.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a post transaction execution handler.
    pub fn add_post_tx_execution_handler<F>(&mut self, handler: F)
    where
        F: Fn(&EvmExecutor, usize) + Send + Sync + 'static,
    {
        self.post_tx_execution_handlers.push(Box::new(handler));
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
