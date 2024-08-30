use crate::error::ZkTrieError;
use crate::{
    executor::hooks::ExecuteHooks, BlockTraceExt, EvmExecutor, HardforkConfig, ReadOnlyDB,
};
use mpt_zktrie::ZktrieState;
use revm::db::CacheDB;

/// Builder for EVM executor.
#[derive(Debug)]
pub struct EvmExecutorBuilder<'a, H> {
    hardfork_config: H,
    execute_hooks: ExecuteHooks,
    zktrie_state: &'a ZktrieState,
}

impl<'a> EvmExecutorBuilder<'a, ()> {
    /// Create a new builder.
    pub fn new(zktrie_state: &'a ZktrieState) -> Self {
        Self {
            hardfork_config: (),
            execute_hooks: ExecuteHooks::default(),
            zktrie_state,
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
            zktrie_state,
            ..self
        }
    }
}

impl<'a> EvmExecutorBuilder<'a, HardforkConfig> {
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build<T: BlockTraceExt>(self, l2_trace: &'a T) -> Result<EvmExecutor, ZkTrieError> {
        let block_number = l2_trace.number();
        let spec_id = self.hardfork_config.get_spec_id(block_number);

        dev_trace!("use spec id {:?}", spec_id);

        let db = cycle_track!(
            CacheDB::new(ReadOnlyDB::new(l2_trace, self.zktrie_state)?),
            "build ReadOnlyDB"
        );

        Ok(EvmExecutor {
            hardfork_config: self.hardfork_config,
            db,
            spec_id,
            hooks: self.execute_hooks,
        })
    }
}
