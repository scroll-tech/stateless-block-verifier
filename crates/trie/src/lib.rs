//! Partial Merkle Patricia Trie
#[macro_use]
extern crate sbv_helpers;

use alloy_rlp::{Decodable, Encodable};
use alloy_trie::{
    nodes::{RlpNode, CHILD_INDEX_RANGE},
    Nibbles, TrieMask, EMPTY_ROOT_HASH,
};
use reth_trie_sparse::RevealedSparseTrie;
use revm::db::BundleAccount;
use sbv_helpers::dev_trace;
use sbv_kv::{nohash::NoHashMap, HashMap};
use sbv_primitives::{keccak256, Address, BlockWitness, B256, U256};
use std::cell::RefCell;

pub use alloy_trie::{nodes::TrieNode, TrieAccount};
pub use reth_trie::{KeccakKeyHasher, KeyHasher};

/// Extension trait for BlockWitness
pub trait BlockWitnessTrieExt {
    /// Import nodes into a KeyValueStore
    fn import_nodes<P: sbv_kv::KeyValueStoreInsert<B256, TrieNode>>(
        &self,
        provider: &mut P,
    ) -> Result<(), alloy_rlp::Error>;
}

impl<T: BlockWitness> BlockWitnessTrieExt for T {
    fn import_nodes<P: sbv_kv::KeyValueStoreInsert<B256, TrieNode>>(
        &self,
        provider: &mut P,
    ) -> Result<(), alloy_rlp::Error> {
        decode_nodes(provider, self.states_iter())
    }
}

impl<T: BlockWitness> BlockWitnessTrieExt for [T] {
    fn import_nodes<P: sbv_kv::KeyValueStoreInsert<B256, TrieNode>>(
        &self,
        provider: &mut P,
    ) -> Result<(), alloy_rlp::Error> {
        decode_nodes(provider, self.iter().flat_map(|w| w.states_iter()))
    }
}

/// Fill a KeyValueStore<B256, TrieNode> from a list of nodes
pub fn decode_nodes<
    B: AsRef<[u8]>,
    P: sbv_kv::KeyValueStoreInsert<B256, TrieNode>,
    I: Iterator<Item = B>,
>(
    provider: &mut P,
    iter: I,
) -> Result<(), alloy_rlp::Error> {
    for byte in iter {
        let mut buf = byte.as_ref();
        let node_hash = cycle_track!(keccak256(buf), "keccak256");
        let node = cycle_track!(TrieNode::decode(&mut buf), "TrieNode::decode")?;
        assert!(
            buf.is_empty(),
            "the rlp buffer should only contains the node"
        );
        provider.insert(node_hash, node);
    }
    Ok(())
}

/// A partial trie that can be updated
#[derive(Debug)]
pub struct PartialStateTrie {
    state: PartialTrie<TrieAccount>,
    /// address -> hashed address
    address_hashes: RefCell<HashMap<Address, B256>>,
    /// hashed address -> storage root
    storage_roots: RefCell<NoHashMap<B256, B256>>,
    /// hashed address -> storage tire
    storage_tries: RefCell<NoHashMap<B256, PartialTrie<U256>>>,
    /// shared rlp buffer
    rlp_buffer: Vec<u8>,
}

impl PartialStateTrie {
    /// Open a partial trie from a root node
    pub fn open<P: sbv_kv::KeyValueStoreGet<B256, TrieNode> + Copy>(
        nodes_provider: P,
        root: B256,
    ) -> Self {
        let state = cycle_track!(
            PartialTrie::open(nodes_provider, root, decode_trie_account),
            "PartialTrie::open"
        );

        PartialStateTrie {
            state,
            address_hashes: Default::default(),
            storage_roots: Default::default(),
            storage_tries: Default::default(),
            rlp_buffer: Vec::with_capacity(128), // pre-allocate 128 bytes
        }
    }

    /// Get account
    #[must_use]
    pub fn get_account(&self, address: Address) -> Option<&TrieAccount> {
        cycle_track!(
            self.get_account_inner(address),
            "PartialStateTrie::get_account"
        )
    }

    fn get_account_inner(&self, address: Address) -> Option<&TrieAccount> {
        let hashed_address = self.hashed_address(address);
        let path = Nibbles::unpack(hashed_address);
        self.state.get(&path).inspect(|account| {
            self.storage_roots
                .borrow_mut()
                .insert(hashed_address, account.storage_root);
        })
    }

    /// Get storage
    #[must_use]
    pub fn get_storage<P: sbv_kv::KeyValueStoreGet<B256, TrieNode> + Copy>(
        &self,
        nodes_provider: P,
        address: Address,
        index: U256,
    ) -> Option<U256> {
        cycle_track!(
            self.get_storage_inner(nodes_provider, address, index),
            "PartialStateTrie::get_storage"
        )
    }

    fn get_storage_inner<P: sbv_kv::KeyValueStoreGet<B256, TrieNode> + Copy>(
        &self,
        nodes_provider: P,
        address: Address,
        index: U256,
    ) -> Option<U256> {
        let hashed_address = self.hashed_address(address);
        let storage_root = *self.storage_roots.borrow().get(&hashed_address)?;
        let path = Nibbles::unpack(keccak256(index.to_be_bytes::<{ U256::BYTES }>()));

        self.storage_tries
            .borrow_mut()
            .entry(hashed_address)
            .or_insert_with(|| {
                dev_trace!("open storage trie of {address} at {storage_root}");
                PartialTrie::open(nodes_provider, storage_root, decode_u256_rlp)
            })
            .get(&path)
            .copied()
    }

    /// Commit state changes and calculate the new state root
    #[must_use]
    pub fn commit_state(&mut self) -> B256 {
        self.state.trie.root()
    }

    /// Update the trie with the new state
    pub fn update<'a, P: sbv_kv::KeyValueStoreGet<B256, TrieNode> + Copy>(
        &mut self,
        nodes_provider: P,
        post_state: impl IntoIterator<Item = (&'a Address, &'a BundleAccount)>,
    ) {
        for (address, account) in post_state.into_iter() {
            dev_trace!("update account: {address} {:?}", account.info);
            let hashed_address = self.hashed_address(*address);
            let account_path = Nibbles::unpack(hashed_address);

            if account.was_destroyed() {
                self.state.remove_leaf(&account_path);
                continue;
            }

            let trie = self
                .storage_tries
                .get_mut()
                .entry(hashed_address)
                .or_insert_with(|| {
                    let storage_root = self
                        .storage_roots
                        .get_mut()
                        .get(&hashed_address)
                        .copied()
                        .unwrap_or(EMPTY_ROOT_HASH);
                    dev_trace!("open storage trie of {address} at {storage_root}");
                    PartialTrie::open(nodes_provider, storage_root, decode_u256_rlp)
                });
            dev_trace!("opened storage trie of {address} at {}", trie.trie.root());

            for (key, slot) in account.storage.iter() {
                let key_hash = keccak256(key.to_be_bytes::<{ U256::BYTES }>());
                let path = Nibbles::unpack(key_hash);

                dev_trace!(
                    "update storage of {address}: {key:#064X}={:#064X}, key_hash={key_hash}",
                    slot.present_value
                );

                if slot.present_value.is_zero() {
                    trie.remove_leaf(&path);
                } else {
                    trie.update_leaf(path, slot.present_value, |value| {
                        self.rlp_buffer.clear();
                        value.encode(&mut self.rlp_buffer);
                        self.rlp_buffer.clone()
                    });
                }
            }

            let storage_root = trie.trie.root();
            dev_trace!("new storage root: {storage_root}");
            let info = account.info.as_ref().unwrap();
            let account = TrieAccount {
                nonce: info.nonce,
                balance: info.balance,
                storage_root,
                code_hash: info.code_hash,
            };
            dev_trace!("update account: {address} {:?}", account);
            self.update_account(hashed_address, account);
        }
    }

    /// Get the hashed address with memoization
    #[inline(always)]
    fn hashed_address(&self, address: Address) -> B256 {
        *self
            .address_hashes
            .borrow_mut()
            .entry(address)
            .or_insert_with(|| cycle_track!(keccak256(address), "keccak256"))
    }

    /// Update the account
    #[inline(always)]
    fn update_account(&mut self, hashed_address: B256, account: TrieAccount) {
        let account_path = Nibbles::unpack(hashed_address);

        self.state.update_leaf(account_path, account, |account| {
            self.rlp_buffer.clear();
            account.encode(&mut self.rlp_buffer);
            self.rlp_buffer.clone()
        });
    }
}

/// A partial trie that can be updated
#[derive(Debug, Default)]
struct PartialTrie<T> {
    trie: RevealedSparseTrie,
    /// FIXME: `RevealedSparseTrie` did not expose API to get the leafs
    leafs: HashMap<Nibbles, T>,
}

impl<T: Default> PartialTrie<T> {
    /// Open a partial trie from a root node
    fn open<P: sbv_kv::KeyValueStoreGet<B256, TrieNode> + Copy, F: FnOnce(&[u8]) -> T + Copy>(
        nodes_provider: P,
        root: B256,
        parse_leaf: F,
    ) -> Self {
        if root == EMPTY_ROOT_HASH {
            return Self::default();
        }
        let root = nodes_provider.get(&root).unwrap().into_owned();
        let mut state = cycle_track!(
            RevealedSparseTrie::from_root(root.clone(), None, true).unwrap(),
            "RevealedSparseTrie::from_root"
        );
        let mut leafs = HashMap::default();
        // traverse the partial trie
        cycle_track!(
            traverse_import_partial_trie(
                &Nibbles::default(),
                root,
                nodes_provider,
                &mut state,
                &mut |path, value| {
                    leafs.insert(path, parse_leaf(value));
                },
            ),
            "traverse_import_partial_trie"
        );

        Self { trie: state, leafs }
    }

    fn get(&self, path: &Nibbles) -> Option<&T> {
        self.leafs.get(path)
    }

    fn update_leaf<F: FnMut(&T) -> Vec<u8>>(&mut self, path: Nibbles, value: T, encode: F) {
        cycle_track!(
            self.update_leaf_inner(path, value, encode),
            "PartialTrie::update_leaf"
        )
    }

    fn remove_leaf(&mut self, path: &Nibbles) {
        cycle_track!(self.remove_leaf_inner(path), "PartialTrie::remove_leaf")
    }

    fn update_leaf_inner<F: FnMut(&T) -> Vec<u8>>(
        &mut self,
        path: Nibbles,
        value: T,
        mut encode: F,
    ) {
        self.trie
            .update_leaf(path.clone(), encode(&value))
            .expect("update leaf");
        self.leafs.insert(path, value);
    }

    fn remove_leaf_inner(&mut self, path: &Nibbles) {
        self.trie.remove_leaf(path).expect("remove leaf");
        self.leafs.remove(path);
    }
}

fn traverse_import_partial_trie<
    P: sbv_kv::KeyValueStoreGet<B256, TrieNode> + Copy,
    F: FnMut(Nibbles, &Vec<u8>),
>(
    path: &Nibbles,
    node: TrieNode,
    nodes: P,
    trie: &mut RevealedSparseTrie,
    store_leaf: &mut F,
) -> Option<TrieMask> {
    let trie_mask = match node {
        TrieNode::EmptyRoot => None,
        TrieNode::Branch(ref branch) => {
            let mut trie_mask = TrieMask::default();

            let mut stack_ptr = branch.as_ref().first_child_index();
            for idx in CHILD_INDEX_RANGE {
                if branch.state_mask.is_bit_set(idx) {
                    trie_mask.set_bit(idx);
                    let mut child_path = path.clone();
                    child_path.push(idx);
                    let child_node = decode_rlp_node(nodes, &branch.stack[stack_ptr]);
                    stack_ptr += 1;

                    if let Some(child_node) = child_node {
                        traverse_import_partial_trie(
                            &child_path,
                            child_node,
                            nodes,
                            trie,
                            store_leaf,
                        );
                    }
                }
            }
            Some(trie_mask)
        }
        TrieNode::Leaf(ref leaf) => {
            let mut full = path.clone();
            full.extend_from_slice_unchecked(&leaf.key);
            store_leaf(full, &leaf.value);
            None
        }
        TrieNode::Extension(ref extension) => {
            let mut child_path = path.clone();
            child_path.extend_from_slice_unchecked(&extension.key);

            if let Some(child_node) = decode_rlp_node(nodes, &extension.child) {
                traverse_import_partial_trie(&child_path, child_node, nodes, trie, store_leaf);
            }

            None
        }
    };

    trie.reveal_node(path.clone(), node, trie_mask).unwrap();

    trie_mask
}

fn decode_trie_account(mut buf: &[u8]) -> TrieAccount {
    let acc = cycle_track!(TrieAccount::decode(&mut buf), "TrieAccount::decode").unwrap();
    assert!(buf.is_empty(), "the leaf should only contains the account");
    acc
}

fn decode_u256_rlp(mut buf: &[u8]) -> U256 {
    let value = cycle_track!(U256::decode(&mut buf), "U256::decode").unwrap();
    assert!(buf.is_empty(), "the leaf should only contains the value");
    value
}

fn decode_rlp_node<P: sbv_kv::KeyValueStoreGet<B256, TrieNode>>(
    nodes_provider: P,
    node: &RlpNode,
) -> Option<TrieNode> {
    if node.len() == B256::len_bytes() + 1 {
        let hash = B256::from_slice(&node[1..]);

        nodes_provider.get(&hash).map(|node| node.into_owned())
    } else {
        let mut buf = node.as_ref();
        let child = cycle_track!(TrieNode::decode(&mut buf), "TrieNode::decode").unwrap();
        assert!(buf.is_empty());

        Some(child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbv_kv::nohash::NoHashMap;
    use sbv_primitives::types::BlockWitness;

    const BLOCK: &str = include_str!("../../../testdata/holesky_witness/2971844.json");

    #[test]
    fn test() {
        let block = serde_json::from_str::<BlockWitness>(BLOCK).unwrap();

        let mut store = NoHashMap::default();
        block.import_nodes(&mut store).unwrap();

        let trie = PartialStateTrie::open(&store, block.pre_state_root);
        for tx in block.transaction.iter() {
            let _ = trie.get_account(tx.from).unwrap();
            let _ = trie.get_storage(&store, tx.from, U256::ZERO);
            if let Some(to) = tx.to {
                let _ = trie.get_account(to);
            }
        }
    }
}
