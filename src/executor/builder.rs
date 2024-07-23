use crate::executor::hooks::ExecuteHooks;
use crate::utils::ext::{BlockRevmDbExt, BlockTraceRevmExt, BlockZktrieExt};
use crate::{EvmExecutor, HardforkConfig, ReadOnlyDB};
use revm::db::CacheDB;

/// Builder for EVM executor.
#[derive(Debug)]
pub struct EvmExecutorBuilder<H> {
    hardfork_config: H,
    execute_hooks: ExecuteHooks,
}

impl Default for EvmExecutorBuilder<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl EvmExecutorBuilder<()> {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            hardfork_config: (),
            execute_hooks: ExecuteHooks::default(),
        }
    }
}

impl<H1> EvmExecutorBuilder<H1> {
    /// Set hardfork config.
    pub fn hardfork_config<H2>(self, hardfork_config: H2) -> EvmExecutorBuilder<H2> {
        EvmExecutorBuilder {
            hardfork_config,
            execute_hooks: self.execute_hooks,
        }
    }

    /// Modify execute hooks.
    pub fn with_execute_hooks(mut self, modify: impl FnOnce(&mut ExecuteHooks)) -> Self {
        modify(&mut self.execute_hooks);
        self
    }
}

impl EvmExecutorBuilder<HardforkConfig> {
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build<T: BlockTraceRevmExt + BlockZktrieExt + BlockRevmDbExt>(
        self,
        l2_trace: &T,
    ) -> EvmExecutor {
        let block_number = l2_trace.number();
        let spec_id = self.hardfork_config.get_spec_id(block_number);
        trace!("use spec id {:?}", spec_id);

        let mut db = CacheDB::new(ReadOnlyDB::new(l2_trace));
        self.hardfork_config.migrate(block_number, &mut db).unwrap();

        let zktrie = l2_trace.zktrie();

        EvmExecutor {
            db,
            zktrie,
            spec_id,
            hooks: self.execute_hooks,
        }
    }
}
