use revm::db::CacheDB;

use crate::{
    cycle_tracker_end, cycle_tracker_start, dev_debug, dev_trace,
    executor::hooks::ExecuteHooks,
    utils::ext::{BlockRevmDbExt, BlockTraceRevmExt, BlockZktrieExt},
    EvmExecutor, HardforkConfig, ReadOnlyDB,
};

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

impl<H> EvmExecutorBuilder<H> {
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

        dev_trace!("use spec id {:?}", spec_id);

        cycle_tracker_start!("build ZktrieState");
        let zktrie_state = l2_trace.zktrie_state();
        cycle_tracker_end!("build ZktrieState");

        let mut db = CacheDB::new(ReadOnlyDB::new(l2_trace, &zktrie_state));
        self.hardfork_config.migrate(block_number, &mut db).unwrap();

        cycle_tracker_start!("build Zktrie");
        let root = *zktrie_state.root();
        dev_debug!("building partial statedb done, root {}", hex::encode(root));
        let zktrie_db = zktrie_state.into_inner();
        let zktrie = zktrie_db.new_trie(&root).unwrap();
        cycle_tracker_end!("build Zktrie");

        EvmExecutor {
            db,
            zktrie_db,
            zktrie,
            spec_id,
            hooks: self.execute_hooks,
        }
    }
}
