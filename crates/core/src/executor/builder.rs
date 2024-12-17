use crate::{EvmDatabase, EvmExecutor};
use reth_primitives::BlockWithSenders;
use sbv_chainspec::ChainSpec;
use sbv_kv::KeyValueStore;
use sbv_primitives::alloy_primitives::Bytes;
use sbv_primitives::{keccak256, BlockWitness, B256};
use sbv_trie::{decode_nodes, TrieNode};
use std::fmt::{self, Debug};
use std::sync::Arc;

/// Builder for EVM executor.
pub struct EvmExecutorBuilder<Spec, CodeDb, NodesProvider, Witness, Block> {
    chain_spec: Spec,
    code_db: CodeDb,
    nodes_provider: NodesProvider,
    witness: Witness,
    block: Block,
}

impl Debug for EvmExecutorBuilder<(), (), (), (), ()> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmExecutorBuilder").finish()
    }
}

impl Default for EvmExecutorBuilder<(), (), (), (), ()> {
    fn default() -> Self {
        Self {
            chain_spec: (),
            code_db: (),
            nodes_provider: (),
            witness: (),
            block: (),
        }
    }
}

impl<Spec, CodeDb, NodesProvider, Witness, Block>
    EvmExecutorBuilder<Spec, CodeDb, NodesProvider, Witness, Block>
{
    /// Create a new EVM executor builder.
    pub fn new() -> EvmExecutorBuilder<(), (), (), (), ()> {
        EvmExecutorBuilder::default()
    }

    /// Set hardfork config.
    pub fn chain_spec<S>(
        self,
        chain_spec: S,
    ) -> EvmExecutorBuilder<S, CodeDb, NodesProvider, Witness, Block> {
        EvmExecutorBuilder {
            chain_spec,
            code_db: self.code_db,
            nodes_provider: self.nodes_provider,
            witness: self.witness,
            block: self.block,
        }
    }

    /// Set code database.
    pub fn code_db<C>(
        self,
        code_db: C,
    ) -> EvmExecutorBuilder<Spec, C, NodesProvider, Witness, Block> {
        EvmExecutorBuilder {
            chain_spec: self.chain_spec,
            code_db,
            nodes_provider: self.nodes_provider,
            witness: self.witness,
            block: self.block,
        }
    }

    /// Set nodes provider.
    pub fn nodes_provider<N>(
        self,
        nodes_provider: N,
    ) -> EvmExecutorBuilder<Spec, CodeDb, N, Witness, Block> {
        EvmExecutorBuilder {
            chain_spec: self.chain_spec,
            code_db: self.code_db,
            nodes_provider,
            witness: self.witness,
            block: self.block,
        }
    }

    /// Set witness.
    pub fn witness<W>(
        self,
        witness: W,
    ) -> EvmExecutorBuilder<Spec, CodeDb, NodesProvider, W, Block> {
        EvmExecutorBuilder {
            chain_spec: self.chain_spec,
            code_db: self.code_db,
            nodes_provider: self.nodes_provider,
            witness,
            block: self.block,
        }
    }

    /// Set Block
    pub fn block<B>(self, block: B) -> EvmExecutorBuilder<Spec, CodeDb, NodesProvider, Witness, B> {
        EvmExecutorBuilder {
            chain_spec: self.chain_spec,
            code_db: self.code_db,
            nodes_provider: self.nodes_provider,
            witness: self.witness,
            block,
        }
    }
}

impl<
        'a,
        Spec: Into<Arc<ChainSpec>>,
        CodeDb: KeyValueStore<B256, Bytes>,
        NodesProvider: KeyValueStore<B256, TrieNode>,
        Witness: BlockWitness,
    > EvmExecutorBuilder<Spec, CodeDb, NodesProvider, Witness, &'a BlockWithSenders>
{
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn build(mut self) -> EvmExecutor<'a, CodeDb, NodesProvider> {
        for code in self.witness.codes_iter() {
            let code = code.as_ref();
            let code_hash = keccak256(code);
            self.code_db.insert(code_hash, Bytes::copy_from_slice(code))
        }
        decode_nodes(&mut self.nodes_provider, self.witness.states_iter()).unwrap();

        let db = cycle_track!(
            EvmDatabase::new_from_root(
                self.code_db,
                self.witness.pre_state_root(),
                self.nodes_provider
            ),
            "build ReadOnlyDB"
        );

        EvmExecutor {
            chain_spec: self.chain_spec.into(),
            db,
            block: self.block,
        }
    }
}
