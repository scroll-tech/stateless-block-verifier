use crate::error::DatabaseError;
use crate::{executor::hooks::ExecuteHooks, EvmDatabase, EvmExecutor, HardforkConfig};
use revm::db::CacheDB;
use revm::primitives::alloy_primitives::ChainId;
use sbv_primitives::zk_trie::db::KVDatabase;
use sbv_primitives::{Block, B256};
use std::fmt::{self, Debug};

/// Builder for EVM executor.
pub struct EvmExecutorBuilder<H, EvmDb, C> {
    hardfork_config: H,
    evm_db: EvmDb,
    chain_id: C,
}

impl<H: Debug, EvmDb, C: Debug> Debug for EvmExecutorBuilder<H, EvmDb, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmExecutorBuilder")
            .field("hardfork_config", &self.hardfork_config)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

impl EvmExecutorBuilder<(), (), ()> {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            hardfork_config: (),
            evm_db: (),
            chain_id: (),
        }
    }
}

impl<H, EvmDb, C> EvmExecutorBuilder<H, EvmDb, C> {
    /// Set hardfork config.
    pub fn hardfork_config<H1>(self, hardfork_config: H1) -> EvmExecutorBuilder<H1, EvmDb, C> {
        EvmExecutorBuilder {
            hardfork_config,
            evm_db: self.evm_db,
            chain_id: self.chain_id,
        }
    }

    /// Set evm db.
    pub fn evm_db<EvmDb1>(self, evm_db: EvmDb1) -> EvmExecutorBuilder<H, EvmDb1, C> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            evm_db,
            chain_id: self.chain_id,
        }
    }

    /// Build evm executor from a block trace.
    pub fn evm_db_from_trace<T: Block, CodeDb: KVDatabase, ZkDb: KVDatabase + Clone + 'static>(
        self,
        l2_trace: &T,
        code_db: CodeDb,
        zktrie_db: ZkDb,
    ) -> Result<EvmExecutorBuilder<H, EvmDatabase<CodeDb, ZkDb>, C>, DatabaseError> {
        Ok(EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            evm_db: EvmDatabase::new_from_trace(l2_trace, code_db, zktrie_db)?,
            chain_id: self.chain_id,
        })
    }

    /// Build evm executor from a block trace.
    pub fn evm_db_from_root<CodeDb: KVDatabase, ZkDb: KVDatabase + Clone + 'static>(
        self,
        committed_zktrie_root: B256,
        code_db: CodeDb,
        zktrie_db: ZkDb,
    ) -> Result<EvmExecutorBuilder<H, EvmDatabase<CodeDb, ZkDb>, C>, DatabaseError> {
        Ok(EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            evm_db: EvmDatabase::new_with_root(committed_zktrie_root, code_db, zktrie_db)?,
            chain_id: self.chain_id,
        })
    }

    /// Set chain id.
    pub fn chain_id<C1>(self, chain_id: C1) -> EvmExecutorBuilder<H, EvmDb, C1> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            evm_db: self.evm_db,
            chain_id,
        }
    }
}

impl<CodeDb: KVDatabase, ZkDb: KVDatabase + Clone + 'static>
    EvmExecutorBuilder<HardforkConfig, EvmDatabase<CodeDb, ZkDb>, ChainId>
{
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn with_hooks<'e, F: FnOnce(&mut ExecuteHooks<'e, CodeDb, ZkDb>)>(
        self,
        with_execute_hooks: F,
    ) -> EvmExecutor<'e, CodeDb, ZkDb> {
        let mut execute_hooks = ExecuteHooks::new();
        with_execute_hooks(&mut execute_hooks);

        let db = CacheDB::new(self.evm_db);

        EvmExecutor {
            chain_id: self.chain_id,
            hardfork_config: self.hardfork_config,
            db,
            hooks: execute_hooks,
        }
    }

    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build<'e>(self) -> EvmExecutor<'e, CodeDb, ZkDb> {
        self.with_hooks(|_| {})
    }
}
