use crate::{error::DatabaseError, EvmDatabase, EvmExecutor, HardforkConfig};
use revm::db::CacheDB;
use sbv_primitives::{
    alloy_primitives::ChainId,
    zk_trie::{
        db::{kv::KVDatabase, NodeDb},
        hash::HashScheme,
    },
    B256,
};
use std::fmt::{self, Debug};

/// Builder for EVM executor.
pub struct EvmExecutorBuilder<'a, HC, C, CodeDb, ZkDb, H> {
    hardfork_config: HC,
    chain_id: C,
    code_db: &'a mut CodeDb,
    zktrie_db: &'a mut NodeDb<ZkDb>,
    hash_scheme: H,
}

impl<HC: Debug, C: Debug, CodeDb, ZkDb, H: Debug> Debug
    for EvmExecutorBuilder<'_, HC, C, CodeDb, ZkDb, H>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmExecutorBuilder")
            .field("hardfork_config", &self.hardfork_config)
            .field("chain_id", &self.chain_id)
            .field("code_db", &"...")
            .field("zktrie_db", &"...")
            .field("hash_scheme", &self.hash_scheme)
            .finish()
    }
}

impl<'a, CodeDb, ZkDb> EvmExecutorBuilder<'a, (), (), CodeDb, ZkDb, ()> {
    /// Create a new builder.
    pub fn new(code_db: &'a mut CodeDb, zktrie_db: &'a mut NodeDb<ZkDb>) -> Self {
        Self {
            hardfork_config: (),
            chain_id: (),
            code_db,
            zktrie_db,
            hash_scheme: (),
        }
    }
}

impl<'a, HC, C, CodeDb, ZkDb, H> EvmExecutorBuilder<'a, HC, C, CodeDb, ZkDb, H> {
    /// Set hardfork config.
    pub fn hardfork_config<H1>(
        self,
        hardfork_config: H1,
    ) -> EvmExecutorBuilder<'a, H1, C, CodeDb, ZkDb, H> {
        EvmExecutorBuilder {
            hardfork_config,
            chain_id: self.chain_id,
            code_db: self.code_db,
            zktrie_db: self.zktrie_db,
            hash_scheme: self.hash_scheme,
        }
    }

    /// Set chain id.
    pub fn chain_id<C1>(self, chain_id: C1) -> EvmExecutorBuilder<'a, HC, C1, CodeDb, ZkDb, H> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            chain_id,
            code_db: self.code_db,
            zktrie_db: self.zktrie_db,
            hash_scheme: self.hash_scheme,
        }
    }

    /// Set code db.
    pub fn code_db<CodeDb1>(
        self,
        code_db: &'a mut CodeDb1,
    ) -> EvmExecutorBuilder<'a, HC, C, CodeDb1, ZkDb, H> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            chain_id: self.chain_id,
            code_db,
            zktrie_db: self.zktrie_db,
            hash_scheme: self.hash_scheme,
        }
    }

    /// Set zktrie db.
    pub fn zktrie_db<ZkDb1>(
        self,
        zktrie_db: &'a mut NodeDb<ZkDb1>,
    ) -> EvmExecutorBuilder<HC, C, CodeDb, ZkDb1, H> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            chain_id: self.chain_id,
            code_db: self.code_db,
            zktrie_db,
            hash_scheme: self.hash_scheme,
        }
    }

    /// Set hash scheme.
    pub fn hash_scheme<H1>(
        self,
        hash_scheme: H1,
    ) -> EvmExecutorBuilder<'a, HC, C, CodeDb, ZkDb, H1> {
        EvmExecutorBuilder {
            hardfork_config: self.hardfork_config,
            chain_id: self.chain_id,
            code_db: self.code_db,
            zktrie_db: self.zktrie_db,
            hash_scheme,
        }
    }
}

impl<'a, CodeDb: KVDatabase, ZkDb: KVDatabase + 'static, H: HashScheme>
    EvmExecutorBuilder<'a, HardforkConfig, ChainId, CodeDb, ZkDb, H>
{
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build(self, root: B256) -> Result<EvmExecutor<'a, CodeDb, ZkDb, H>, DatabaseError> {
        let db = cycle_track!(
            CacheDB::new(EvmDatabase::new_from_root(
                root,
                self.code_db,
                self.zktrie_db
            )?),
            "build ReadOnlyDB"
        );

        Ok(EvmExecutor {
            hardfork_config: self.hardfork_config,
            chain_id: self.chain_id,
            db,
        })
    }
}
