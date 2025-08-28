//! Partial Merkle Patricia Trie
#[macro_use]
extern crate sbv_helpers;

use alloy_rlp::{Decodable, Encodable, encode_fixed_size};
use alloy_trie::{
    EMPTY_ROOT_HASH, Nibbles, TrieMask,
    nodes::{CHILD_INDEX_RANGE, RlpNode},
};
use auto_impl::auto_impl;
use reth_trie::TRIE_ACCOUNT_RLP_MAX_SIZE;
use reth_trie_sparse::{
    SerialSparseTrie, SparseTrieInterface, TrieMasks, errors::SparseTrieError,
    provider::DefaultTrieNodeProvider,
};
use sbv_kv::{HashMap, nohash::NoHashMap};
use sbv_primitives::{
    Address, B256, Bytes, U256, keccak256,
    types::{BlockWitness, revm::database::BundleAccount},
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap, fmt::Debug};

pub use alloy_trie::{TrieAccount, nodes::TrieNode};
pub use reth_trie::{KeccakKeyHasher, KeyHasher};

/// Extension trait for BlockWitness
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitnessTrieExt {
    /// Import nodes into a KeyValueStore
    fn import_nodes<P: sbv_kv::KeyValueStoreInsert<B256, Bytes>>(&self, provider: &mut P);
}

impl BlockWitnessTrieExt for BlockWitness {
    fn import_nodes<P: sbv_kv::KeyValueStoreInsert<B256, Bytes>>(&self, provider: &mut P) {
        for state in self.states.iter() {
            let node_hash = cycle_track!(keccak256(state.as_ref()), "keccak256");
            provider.insert(node_hash, state.clone());
        }
    }
}

impl BlockWitnessTrieExt for [BlockWitness] {
    fn import_nodes<P: sbv_kv::KeyValueStoreInsert<B256, Bytes>>(&self, provider: &mut P) {
        for w in self.iter() {
            for state in w.states.iter() {
                let node_hash = cycle_track!(keccak256(state.as_ref()), "keccak256");
                provider.insert(node_hash, state.clone());
            }
        }
    }
}

/// A partial trie that can be updated
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialStateTrie {
    state: SerialSparseTrie,
    /// address -> storage root
    storage_roots: RefCell<NoHashMap<Address, B256>>,
    /// address -> storage tire
    storage_tries: RefCell<NoHashMap<Address, Option<SerialSparseTrie>>>,
    /// shared rlp buffer
    #[serde(skip, default = "default_rlp_buffer")]
    rlp_buffer: Vec<u8>,
}

/// Partial state trie error
#[derive(thiserror::Error, Debug)]
pub enum PartialStateTrieError {
    /// reth sparse trie error
    #[error("error occurred in reth_trie_sparse: {0}")]
    Impl(String), // FIXME: wtf, why `SparseTrieError` they don't require Sync?
    /// an error occurred while previously try to open the storage trie
    #[error("an error occurred while previously try to open the storage trie")]
    PreviousError,
    /// missing trie witness for node
    #[error("missing trie witness for node: {0}")]
    MissingWitness(B256),
    /// rlp error
    #[error(transparent)]
    Rlp(#[from] alloy_rlp::Error),
    /// extra data in the leaf
    #[error("{0}")]
    ExtraData(&'static str),
}

type Result<T, E = PartialStateTrieError> = std::result::Result<T, E>;

impl PartialStateTrie {
    /// Open a partial trie from a root node
    pub fn open<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
        nodes_provider: &P,
        root: B256,
    ) -> Result<Self> {
        let state = cycle_track!(open_trie(nodes_provider, root), "open_trie")?;

        Ok(PartialStateTrie {
            state,
            storage_roots: RefCell::new(HashMap::with_capacity_and_hasher(256, Default::default())),
            storage_tries: RefCell::new(HashMap::with_capacity_and_hasher(256, Default::default())),
            rlp_buffer: default_rlp_buffer(), // pre-allocate 128 bytes
        })
    }

    /// Open a partial trie from a root node, and preload all account storage tries
    pub fn open_preloaded<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
        nodes_provider: &P,
        root: B256,
        access_list: Vec<Address>,
    ) -> Result<Self> {
        let trie = Self::open(nodes_provider, root)?;

        for address in access_list.into_iter() {
            let Some(account) = trie.get_account(address).ok().flatten() else {
                continue;
            };
            let Ok(storage_trie) = open_trie(nodes_provider, account.storage_root) else {
                continue;
            };
            trie.storage_tries
                .borrow_mut()
                .insert(address, Some(storage_trie));
        }

        Ok(trie)
    }

    /// Get account
    #[cfg_attr(
        feature = "dev",
        tracing::instrument(level = tracing::Level::TRACE, skip(self), ret)
    )]
    pub fn get_account(&self, address: Address) -> Result<Option<TrieAccount>> {
        let path = Nibbles::unpack(keccak256(address));
        let Some(value) = self.state.get_leaf_value(&path) else {
            return Ok(None);
        };
        let account = TrieAccount::decode(&mut value.as_ref())?;
        self.storage_roots
            .borrow_mut()
            .insert(address, account.storage_root);
        Ok(Some(account))
    }

    /// Get storage
    #[cfg_attr(
        feature = "dev",
        tracing::instrument(level = tracing::Level::TRACE, skip(self, nodes_provider), ret, err)
    )]
    pub fn get_storage<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
        &self,
        nodes_provider: &P,
        address: Address,
        index: U256,
    ) -> Result<Option<U256>> {
        let Some(storage_root) = self.storage_roots.borrow().get(&address).copied() else {
            return Ok(None);
        };
        let path = Nibbles::unpack(keccak256(index.to_be_bytes::<{ U256::BYTES }>()));

        let mut tries = self.storage_tries.borrow_mut();
        let storage_trie = tries
            .entry(address)
            .or_insert_with(|| {
                dev_trace!("open storage trie of {address} at {storage_root}");
                open_trie(nodes_provider, storage_root).inspect_err(|_e| {
                    println!(
                        "failed to open storage trie of {address} at {storage_root}, cause: {_e}"
                    )
                }).ok()
            })
            .as_mut()
            .ok_or(PartialStateTrieError::PreviousError)?;
        let Some(value) = storage_trie.get_leaf_value(&path) else {
            return Ok(None);
        };
        let slot = U256::decode(&mut value.as_ref())?;
        Ok(Some(slot))
    }

    /// Commit state changes and calculate the new state root
    #[must_use]
    #[cfg_attr(feature = "dev", tracing::instrument(level = tracing::Level::TRACE, skip_all, ret))]
    pub fn commit_state(&mut self) -> B256 {
        self.state.root()
    }

    /// Update the trie with the new state
    #[cfg_attr(feature = "dev", tracing::instrument(level = tracing::Level::TRACE, skip_all, err))]
    pub fn update<'a, P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
        &mut self,
        nodes_provider: P,
        post_state: impl IntoIterator<Item = (&'a Address, &'a BundleAccount)>,
    ) -> Result<()> {
        for (address, account) in post_state.into_iter() {
            dev_trace!("update account: {address} {:?}", account.info);
            let account_path = Nibbles::unpack(keccak256(address));

            if account.was_destroyed() {
                self.state
                    .remove_leaf(&account_path, DefaultTrieNodeProvider)?;
                continue;
            }

            let storage_root = if !account.storage.is_empty() {
                dev_trace!("non-empty storage, trie needs to be updated");
                let trie = self
                    .storage_tries
                    .get_mut()
                    .entry(*address)
                    .or_insert_with(|| {
                        let storage_root = self
                            .storage_roots
                            .get_mut()
                            .get(address)
                            .copied()
                            .unwrap_or(EMPTY_ROOT_HASH);
                        dev_trace!("open storage trie of {address} at {storage_root}");
                        open_trie(&nodes_provider, storage_root)
                            .inspect_err(|_e| {
                                println!(
                                    "failed to open storage trie of {address} at {storage_root}, cause: {_e}"
                                )
                            }).ok()
                    })
                    .as_mut()
                    .ok_or(PartialStateTrieError::PreviousError)?;
                dev_trace!("opened storage trie of {address} at {}", trie.root());

                for (key, slot) in BTreeMap::from_iter(account.storage.clone()) {
                    let key_hash = keccak256(key.to_be_bytes::<{ U256::BYTES }>());
                    let path = Nibbles::unpack(key_hash);

                    dev_trace!(
                        "update storage of {address}: {key:#064X}={:#064X}, key_hash={key_hash}",
                        slot.present_value
                    );

                    if slot.present_value.is_zero() {
                        trie.remove_leaf(&path, DefaultTrieNodeProvider)?;
                    } else {
                        let value = encode_fixed_size(&slot.present_value);
                        trie.update_leaf(path, value.to_vec(), DefaultTrieNodeProvider)?;
                    }
                }
                trie.root()
            } else {
                dev_trace!("empty storage, skip trie update");
                self.storage_roots
                    .get_mut()
                    .get(address)
                    .copied()
                    .unwrap_or(EMPTY_ROOT_HASH)
            };

            dev_trace!("current storage root: {storage_root}");
            let info = account.info.as_ref().unwrap();
            let account = TrieAccount {
                nonce: info.nonce,
                balance: info.balance,
                storage_root,
                code_hash: info.code_hash,
            };
            dev_trace!("update account: {address} {:?}", account);
            self.rlp_buffer.clear();
            account.encode(&mut self.rlp_buffer);
            self.state.update_leaf(
                account_path,
                self.rlp_buffer.clone(),
                DefaultTrieNodeProvider,
            )?;
        }

        Ok(())
    }
}

#[inline(always)]
fn open_trie<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
    nodes_provider: &P,
    root: B256,
) -> Result<SerialSparseTrie> {
    if root == EMPTY_ROOT_HASH {
        return Ok(SerialSparseTrie::default());
    }
    let root_node = nodes_provider
        .get(&root)
        .ok_or(PartialStateTrieError::MissingWitness(root))?;
    let root = TrieNode::decode(&mut root_node.as_ref())?;
    let mut trie = SerialSparseTrie::from_root(root.clone(), TrieMasks::none(), false)?;
    cycle_track!(
        traverse_import_partial_trie(Nibbles::default(), root, nodes_provider, &mut trie),
        "traverse_import_partial_trie"
    )?;
    Ok(trie)
}

#[inline(always)]
fn traverse_import_partial_trie<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
    path: Nibbles,
    node: TrieNode,
    nodes: &P,
    trie: &mut SerialSparseTrie,
) -> Result<()> {
    match node {
        TrieNode::EmptyRoot => trie.reveal_node(path, node, TrieMasks::none())?,
        TrieNode::Branch(ref branch) => {
            let mut stack_ptr = branch.as_ref().first_child_index();
            let mut hash_mask = TrieMask::default();
            let mut tree_mask = TrieMask::default();

            for idx in CHILD_INDEX_RANGE {
                if branch.state_mask.is_bit_set(idx) {
                    let mut child_path = path;
                    child_path.push(idx);
                    let child_node = decode_rlp_node(nodes, &branch.stack[stack_ptr])?;
                    stack_ptr += 1;

                    if let Some(child_node) = child_node {
                        traverse_import_partial_trie(child_path, child_node, nodes, trie)?;
                        tree_mask.set_bit(idx);
                    } else {
                        hash_mask.set_bit(idx);
                    }
                }
            }

            let trie_mask = TrieMasks {
                hash_mask: Some(hash_mask),
                tree_mask: Some(tree_mask),
            };
            trie.reveal_node(path, node, trie_mask)?;
        }
        TrieNode::Leaf(_) => trie.reveal_node(path, node, TrieMasks::none())?,
        TrieNode::Extension(ref extension) => {
            let mut child_path = path;
            child_path.extend(&extension.key);

            if let Some(child_node) = decode_rlp_node(nodes, &extension.child)? {
                traverse_import_partial_trie(child_path, child_node, nodes, trie)?;
            }
            trie.reveal_node(path, node, TrieMasks::none())?;
        }
    };

    Ok(())
}

#[inline(always)]
fn decode_rlp_node<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
    nodes_provider: P,
    node: &RlpNode,
) -> Result<Option<TrieNode>> {
    if node.len() == B256::len_bytes() + 1 {
        let hash = B256::from_slice(&node[1..]);
        let Some(node_bytes) = nodes_provider.get(&hash) else {
            return Ok(None);
        };
        Ok(Some(TrieNode::decode(&mut node_bytes.as_ref())?))
    } else {
        let mut buf = node.as_ref();
        Ok(Some(TrieNode::decode(&mut buf)?))
    }
}

fn default_rlp_buffer() -> Vec<u8> {
    Vec::with_capacity(TRIE_ACCOUNT_RLP_MAX_SIZE) // pre-allocate 128 bytes
}

impl From<SparseTrieError> for PartialStateTrieError {
    #[inline]
    fn from(value: SparseTrieError) -> Self {
        PartialStateTrieError::Impl(format!("{value:?}"))
    }
}
