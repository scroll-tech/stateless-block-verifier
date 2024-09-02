use crate::error::ZkTrieError;
use crate::{executor::hooks::ExecuteHooks, EvmExecutor, HardforkConfig, ReadOnlyDB};
use mpt_zktrie::ZktrieState;
use revm::db::CacheDB;
use sbv_primitives::BlockTrace;

/// Builder for EVM executor.
#[derive(Debug)]
pub struct EvmExecutorBuilder<'e, 'z, H> {
    hardfork_config: H,
    execute_hooks: ExecuteHooks<'e>,
    zktrie_state: &'z ZktrieState,
}

impl<'e, 'z> EvmExecutorBuilder<'e, 'z, ()> {
    /// Create a new builder.
    pub fn new(zktrie_state: &'z ZktrieState) -> Self {
        Self {
            hardfork_config: (),
            execute_hooks: ExecuteHooks::default(),
            zktrie_state,
        }
    }
}

impl<'e, 'z, H> EvmExecutorBuilder<'e, 'z, H> {
    /// Set hardfork config.
    pub fn hardfork_config<H1>(self, hardfork_config: H1) -> EvmExecutorBuilder<'e, 'z, H1> {
        EvmExecutorBuilder {
            hardfork_config,
            execute_hooks: self.execute_hooks,
            zktrie_state: self.zktrie_state,
        }
    }

    /// Modify execute hooks.
    pub fn with_execute_hooks(mut self, modify: impl FnOnce(&mut ExecuteHooks<'e>)) -> Self {
        modify(&mut self.execute_hooks);
        self
    }

    /// Set zktrie state.
    pub fn zktrie_state(self, zktrie_state: &'z ZktrieState) -> EvmExecutorBuilder<'e, 'z, H> {
        EvmExecutorBuilder {
            zktrie_state,
            ..self
        }
    }
}

impl<'e, 'z> EvmExecutorBuilder<'e, 'z, HardforkConfig> {
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build<T: BlockTrace>(self, l2_trace: &'z T) -> Result<EvmExecutor<'e>, ZkTrieError> {
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
