use crate::error::DatabaseError;
use crate::{executor::hooks::ExecuteHooks, EvmDatabase, EvmExecutor, HardforkConfig};
use revm::db::CacheDB;
use sbv_primitives::zk_trie::db::kv::KVDatabase;
use sbv_primitives::zk_trie::db::NodeDb;
use sbv_primitives::Block;
use std::fmt::{self, Debug};
use sbv_primitives::alloy_primitives::ChainId;

/// Builder for EVM executor.
pub struct EvmExecutorBuilder<'a, H, C, CodeDb, ZkDb> {
    hardfork_config: H,
    chain_id: C,
    code_db: CodeDb,
    zktrie_db: &'a mut NodeDb<ZkDb>,
}

impl<H: Debug, C: Debug, CodeDb, ZkDb> Debug for EvmExecutorBuilder<'_, H, C, CodeDb, ZkDb> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmExecutorBuilder")
            .field("hardfork_config", &self.hardfork_config)
            .field("chain_id", &self.chain_id)
            .field("code_db", &"...")
            .field("zktrie_db", &"...")
            .finish()
    }
}

impl<'a, CodeDb, ZkDb> EvmExecutorBuilder<'a, (), (), CodeDb, ZkDb> {
    /// Create a new builder.
    pub fn new(code_db: CodeDb, zktrie_db: &'a mut NodeDb<ZkDb>) -> Self {
        Self {
            hardfork_config: (),
            chain_id: (),
            code_db,
            zktrie_db,
        }
    }
}

impl<'a, H, C, CodeDb, ZkDb> EvmExecutorBuilder<'a, H, C, CodeDb, ZkDb> {
    /// Set hardfork config.
    pub fn hardfork_config<H1>(
        self,
        hardfork_config: H1,
    ) -> EvmExecutorBuilder<'a, H1, C, CodeDb, ZkDb> {
        EvmExecutorBuilder {
            hardfork_config,
            chain_id: self.chain_id,
            code_db: self.code_db,
            zktrie_db: self.zktrie_db,
        }
    }

    /// Set code db.
    pub fn code_db<CodeDb1>(self, code_db: CodeDb1) -> EvmExecutorBuilder<'a, H, C, CodeDb1, ZkDb> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            chain_id: self.chain_id,
            code_db,
            zktrie_db: self.zktrie_db,
        }
    }

    /// Set zktrie db.
    pub fn zktrie_db<ZkDb1>(
        self,
        zktrie_db: &mut NodeDb<ZkDb1>,
    ) -> EvmExecutorBuilder<H, C, CodeDb, ZkDb1> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            chain_id: self.chain_id,
            code_db: self.code_db,
            zktrie_db,
        }
    }
}

impl<'a, CodeDb: KVDatabase, ZkDb: KVDatabase + 'static>
    EvmExecutorBuilder<'a, HardforkConfig, ChainId, CodeDb, ZkDb>
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
            chain_id: self.chain_id,
            db,
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
