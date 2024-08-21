use crate::{
    cycle_tracker_end, cycle_tracker_start, dev_trace, executor::hooks::ExecuteHooks,
    BlockTraceExt, EvmExecutor, HardforkConfig, ReadOnlyDB,
};
use mpt_zktrie::ZktrieState;
use revm::db::CacheDB;
use std::borrow::Cow;

/// Builder for EVM executor.
#[derive(Debug)]
pub struct EvmExecutorBuilder<'a, H> {
    hardfork_config: H,
    execute_hooks: ExecuteHooks,
    zktrie_state: Option<Cow<'a, ZktrieState>>,
}

impl Default for EvmExecutorBuilder<'static, ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl EvmExecutorBuilder<'static, ()> {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            hardfork_config: (),
            execute_hooks: ExecuteHooks::default(),
            zktrie_state: None,
        }
    }
}

impl<'a, H> EvmExecutorBuilder<'a, H> {
    /// Set hardfork config.
    pub fn hardfork_config<H1>(self, hardfork_config: H1) -> EvmExecutorBuilder<'a, H1> {
        EvmExecutorBuilder {
            hardfork_config,
            execute_hooks: self.execute_hooks,
            zktrie_state: self.zktrie_state,
        }
    }

    /// Modify execute hooks.
    pub fn with_execute_hooks(mut self, modify: impl FnOnce(&mut ExecuteHooks)) -> Self {
        modify(&mut self.execute_hooks);
        self
    }

    /// Set zktrie state.
    pub fn zktrie_state(self, zktrie_state: &ZktrieState) -> EvmExecutorBuilder<H> {
        EvmExecutorBuilder {
            zktrie_state: Some(Cow::Borrowed(zktrie_state)),
            ..self
        }
    }
}

impl EvmExecutorBuilder<'_, HardforkConfig> {
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build<T: BlockTraceExt>(self, l2_trace: &T) -> EvmExecutor {
        let block_number = l2_trace.number();
        let spec_id = self.hardfork_config.get_spec_id(block_number);

        dev_trace!("use spec id {:?}", spec_id);

        let zktrie_state = self.zktrie_state.unwrap_or_else(|| {
            cycle_tracker_start!("build ZktrieState");
            let old_root = l2_trace.root_before();
            let mut zktrie_state = ZktrieState::construct(old_root);
            l2_trace.build_zktrie_state(&mut zktrie_state);
            cycle_tracker_end!("build ZktrieState");
            Cow::Owned(zktrie_state)
        });

        cycle_tracker_start!("build ReadOnlyDB");
        let mut db = ReadOnlyDB::new();
        db.update(l2_trace, &zktrie_state);
        let db = CacheDB::new(db);
        cycle_tracker_end!("build ReadOnlyDB");

        let zktrie_db = zktrie_state.zk_db.clone();
        let zktrie = zktrie_db.new_trie(&l2_trace.root_before().0).unwrap();

        EvmExecutor {
            hardfork_config: self.hardfork_config,
            db,
            zktrie_db,
            zktrie,
            spec_id,
            hooks: self.execute_hooks,
        }
    }
}
