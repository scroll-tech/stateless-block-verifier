use crate::error::DatabaseError;
use crate::{executor::hooks::ExecuteHooks, EvmDatabase, EvmExecutor, HardforkConfig};
use revm::db::CacheDB;
use sbv_primitives::zk_trie::db::kv::KVDatabase;
use sbv_primitives::zk_trie::db::NodeDb;
use sbv_primitives::Block;
use std::fmt::{self, Debug};

/// Builder for EVM executor.
pub struct EvmExecutorBuilder<'a, H, CodeDb, ZkDb> {
    hardfork_config: H,
    code_db: CodeDb,
    zktrie_db: &'a mut NodeDb<ZkDb>,
}

impl<H: Debug, CodeDb, ZkDb> Debug for EvmExecutorBuilder<'_, H, CodeDb, ZkDb> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmExecutorBuilder")
            .field("hardfork_config", &self.hardfork_config)
            .field("code_db", &"...")
            .field("zktrie_db", &"...")
            .finish()
    }
}

impl<'a, CodeDb, ZkDb> EvmExecutorBuilder<'a, (), CodeDb, ZkDb> {
    /// Create a new builder.
    pub fn new(code_db: CodeDb, zktrie_db: &'a mut NodeDb<ZkDb>) -> Self {
        Self {
            hardfork_config: (),
            code_db,
            zktrie_db,
        }
    }
}

impl<'a, H, CodeDb, ZkDb> EvmExecutorBuilder<'a, H, CodeDb, ZkDb> {
    /// Set hardfork config.
    pub fn hardfork_config<H1>(
        self,
        hardfork_config: H1,
    ) -> EvmExecutorBuilder<'a, H1, CodeDb, ZkDb> {
        EvmExecutorBuilder {
            hardfork_config,
            code_db: self.code_db,
            zktrie_db: self.zktrie_db,
        }
    }

    /// Set code db.
    pub fn code_db<CodeDb1>(self, code_db: CodeDb1) -> EvmExecutorBuilder<'a, H, CodeDb1, ZkDb> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            code_db,
            zktrie_db: self.zktrie_db,
        }
    }

    /// Set zktrie db.
    pub fn zktrie_db<'b, ZkDb1>(
        self,
        zktrie_db: &'b mut NodeDb<ZkDb1>,
    ) -> EvmExecutorBuilder<'b, H, CodeDb, ZkDb1> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            code_db: self.code_db,
            zktrie_db,
        }
    }
}

impl<'a, CodeDb: KVDatabase, ZkDb: KVDatabase + 'static>
    EvmExecutorBuilder<'a, HardforkConfig, CodeDb, ZkDb>
{
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn with_hooks<'h, T: Block, F: FnOnce(&mut ExecuteHooks<'h, CodeDb, ZkDb>)>(
        self,
        l2_trace: &T,
        with_execute_hooks: F,
    ) -> Result<EvmExecutor<'a, 'h, CodeDb, ZkDb>, DatabaseError> {
        let mut execute_hooks = ExecuteHooks::new();
        with_execute_hooks(&mut execute_hooks);

        let block_number = l2_trace.number();
        let spec_id = self.hardfork_config.get_spec_id(block_number);

        dev_trace!("use spec id {:?}", spec_id);

        let db = cycle_track!(
            CacheDB::new(EvmDatabase::new(l2_trace, self.code_db, self.zktrie_db)?),
            "build ReadOnlyDB"
        );

        Ok(EvmExecutor {
            hardfork_config: self.hardfork_config,
            db,
            spec_id,
            hooks: execute_hooks,
        })
    }

    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build<'e, T: Block>(
        self,
        l2_trace: &T,
    ) -> Result<EvmExecutor<'a, 'e, CodeDb, ZkDb>, DatabaseError> {
        self.with_hooks(l2_trace, |_| {})
    }
}
