use crate::{EvmDatabase, EvmExecutor};
use revm::db::CacheDB;
use sbv_chainspec::ChainSpec;
use sbv_kv::KeyValueStore;
use sbv_primitives::alloy_primitives::Bytes;
use sbv_primitives::{keccak256, BlockWitness, B256};
use sbv_trie::TrieNode;
use std::fmt::{self, Debug};

/// Builder for EVM executor.
pub struct EvmExecutorBuilder<Spec, CodeDb, NodesProvider, Witness> {
    chain_spec: Spec,
    code_db: CodeDb,
    nodes_provider: NodesProvider,
    witness: Witness,
}

impl Debug for EvmExecutorBuilder<(), (), (), ()> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmExecutorBuilder").finish()
    }
}

impl Default for EvmExecutorBuilder<(), (), (), ()> {
    fn default() -> Self {
        Self {
            chain_spec: (),
            code_db: (),
            nodes_provider: (),
            witness: (),
        }
    }
}

impl<Spec, CodeDb, NodesProvider, Witness>
    EvmExecutorBuilder<Spec, CodeDb, NodesProvider, Witness>
{
    /// Create a new EVM executor builder.
    pub fn new() -> EvmExecutorBuilder<(), (), (), ()> {
        EvmExecutorBuilder::default()
    }

    /// Set hardfork config.
    pub fn chain_spec<S>(
        self,
        chain_spec: S,
    ) -> EvmExecutorBuilder<S, CodeDb, NodesProvider, Witness> {
        EvmExecutorBuilder {
            chain_spec,
            code_db: self.code_db,
            nodes_provider: self.nodes_provider,
            witness: self.witness,
        }
    }

    /// Set code database.
    pub fn code_db<C>(self, code_db: C) -> EvmExecutorBuilder<Spec, C, NodesProvider, Witness> {
        EvmExecutorBuilder {
            chain_spec: self.chain_spec,
            code_db,
            nodes_provider: self.nodes_provider,
            witness: self.witness,
        }
    }

    /// Set nodes provider.
    pub fn nodes_provider<N>(
        self,
        nodes_provider: N,
    ) -> EvmExecutorBuilder<Spec, CodeDb, N, Witness> {
        EvmExecutorBuilder {
            chain_spec: self.chain_spec,
            code_db: self.code_db,
            nodes_provider,
            witness: self.witness,
        }
    }

    /// Set witness.
    pub fn witness<W>(self, witness: W) -> EvmExecutorBuilder<Spec, CodeDb, NodesProvider, W> {
        EvmExecutorBuilder {
            chain_spec: self.chain_spec,
            code_db: self.code_db,
            nodes_provider: self.nodes_provider,
            witness,
        }
    }
}

impl<
        CodeDb: KeyValueStore<B256, Bytes>,
        NodesProvider: KeyValueStore<B256, TrieNode>,
        Witness: BlockWitness,
    > EvmExecutorBuilder<ChainSpec, CodeDb, NodesProvider, Witness>
{
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build(mut self, root: B256) -> EvmExecutor<CodeDb, NodesProvider, Witness> {
        for code in self.witness.codes_iter() {
            let code = code.as_ref();
            let code_hash = keccak256(code);
            self.code_db.insert(code_hash, Bytes::copy_from_slice(code))
        }

        let db = cycle_track!(
            CacheDB::new(EvmDatabase::new_from_root(
                self.code_db,
                root,
                self.nodes_provider
            )),
            "build ReadOnlyDB"
        );

        EvmExecutor {
            chain_spec: self.chain_spec,
            db,
            witness: self.witness,
        }
    }
}
