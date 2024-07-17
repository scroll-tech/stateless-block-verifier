use crate::executor::hooks::ExecuteHooks;
use crate::utils::{collect_account_proofs, collect_storage_proofs};
use crate::{EvmExecutor, HardforkConfig, ReadOnlyDB};
use eth_types::l2_types::{BlockTrace, BlockTraceV2};
use mpt_zktrie::ZktrieState;
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
    /// Initialize an EVM executor from a legacy block trace as the initial state.
    pub fn build_legacy(self, l2_trace: &BlockTrace) -> EvmExecutor {
        let v2_trace = BlockTraceV2::from(l2_trace.clone());
        self.build(&v2_trace)
    }

    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build(self, l2_trace: &BlockTraceV2) -> EvmExecutor {
        let block_number = l2_trace.header.number.unwrap().as_u64();
        let spec_id = self.hardfork_config.get_spec_id(block_number);
        trace!("use spec id {:?}", spec_id);

        let mut db = CacheDB::new(ReadOnlyDB::new(l2_trace));
        self.hardfork_config.migrate(block_number, &mut db).unwrap();

        let old_root = l2_trace.storage_trace.root_before;
        let zktrie_state = ZktrieState::from_trace_with_additional(
            old_root,
            collect_account_proofs(&l2_trace.storage_trace),
            collect_storage_proofs(&l2_trace.storage_trace),
            l2_trace
                .storage_trace
                .deletion_proofs
                .iter()
                .map(|s| s.as_ref()),
        )
        .unwrap();
        let root = *zktrie_state.root();
        debug!("building partial statedb done, root {}", hex::encode(root));

        let mem_db = zktrie_state.into_inner();
        let zktrie = mem_db.new_trie(&root).unwrap();

        EvmExecutor {
            db,
            zktrie,
            spec_id,
            hooks: self.execute_hooks,
        }
    }
}
