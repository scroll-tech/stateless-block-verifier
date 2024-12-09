//! Partial Merkle Patricia Trie

extern crate core;

use alloy_rlp::{Decodable, Encodable};
use alloy_trie::{
    nodes::{TrieNode, CHILD_INDEX_RANGE},
    Nibbles, TrieAccount, TrieMask, EMPTY_ROOT_HASH,
};
use reth_trie_sparse::RevealedSparseTrie;
use sbv_kv::{KeyValueStoreGet, KeyValueStoreInsert};
use sbv_primitives::{keccak256, Address, B256, U256};
use std::cell::RefCell;
use std::collections::HashMap;

/// Fill a KeyValueStore<B256, TrieNode> from a list of nodes
pub fn decode_nodes<
    B: AsRef<[u8]>,
    S: KeyValueStoreInsert<B256, TrieNode>,
    I: Iterator<Item = B>,
>(
    store: &mut S,
    iter: I,
) -> Result<(), alloy_rlp::Error> {
    for byte in iter {
        let mut buf = byte.as_ref();
        let node_hash = keccak256(buf);
        let node = TrieNode::decode(&mut buf)?;
        assert!(
            buf.is_empty(),
            "the rlp buffer should only contains the node"
        );
        store.insert(node_hash, node);
    }
    Ok(())
}

/// A partial trie that can be updated
#[derive(Debug)]
pub struct PartialStateTrie {
    state: PartialTrie<TrieAccount>,
    storage_roots: RefCell<HashMap<Address, B256>>,
    storage_tries: RefCell<HashMap<Address, PartialTrie<U256>>>,
    /// shared rlp buffer
    rlp_buffer: Vec<u8>,
}

impl PartialStateTrie {
    /// Open a partial trie from a root node
    pub fn open<S: KeyValueStoreGet<B256, TrieNode>>(node_store: &S, root: B256) -> Self {
        let state = PartialTrie::open(node_store, root, |mut value| {
            let account = TrieAccount::decode(&mut value).unwrap();
            assert!(
                value.is_empty(),
                "the leaf should only contains the account"
            );
            account
        });

        PartialStateTrie {
            state,
            storage_roots: Default::default(),
            storage_tries: Default::default(),
            rlp_buffer: Vec::new(),
        }
    }

    /// Get account
    #[must_use]
    pub fn get_account(&self, address: Address) -> Option<&TrieAccount> {
        let path = Nibbles::unpack(keccak256(address));
        self.state.get(&path).inspect(|account| {
            self.storage_roots
                .borrow_mut()
                .insert(address, account.storage_root);
        })
    }

    /// Update account
    pub fn update_account(&mut self, address: Address, account: TrieAccount) {
        let path = Nibbles::unpack(keccak256(address));
        self.state.update_leaf(path, account, |account| {
            self.rlp_buffer.clear();
            account.encode(&mut self.rlp_buffer);
            self.rlp_buffer.clone()
        });
    }

    /// Get storage
    #[must_use]
    pub fn get_storage<S: KeyValueStoreGet<B256, TrieNode>>(
        &self,
        node_store: &S,
        address: Address,
        index: U256,
    ) -> Option<U256> {
        let storage_root = *self.storage_roots.borrow().get(&address)?;
        let path = Nibbles::unpack(keccak256(index.to_be_bytes::<32>()));

        self.storage_tries
            .borrow_mut()
            .entry(address)
            .or_insert_with(|| PartialTrie::open(node_store, storage_root, U256::from_be_slice))
            .get(&path)
            .copied()
    }

    /// Update storage
    pub fn update_storage<S: KeyValueStoreGet<B256, TrieNode>>(
        &mut self,
        node_store: &S,
        address: Address,
        index: U256,
        value: U256,
    ) {
        let storage_root = self
            .storage_roots
            .get_mut()
            .get(&address)
            .copied()
            .unwrap_or(EMPTY_ROOT_HASH);
        let path = Nibbles::unpack(keccak256(index.to_be_bytes::<32>()));

        let trie = self
            .storage_tries
            .get_mut()
            .entry(address)
            .or_insert_with(|| PartialTrie::open(node_store, storage_root, U256::from_be_slice));

        if value.is_zero() {
            trie.remove_leaf(&path);
        } else {
            trie.update_leaf(path, value, |value| value.to_be_bytes_trimmed_vec());
        }
    }

    /// Commit storage changes and calculate the new storage root
    #[must_use]
    pub fn commit_storage<S: KeyValueStoreGet<B256, TrieNode>>(
        &mut self,
        node_store: &S,
        address: Address,
    ) -> B256 {
        let storage_roots = self.storage_roots.get_mut();
        let storage_root = storage_roots.entry(address).or_insert(EMPTY_ROOT_HASH);

        let trie = self
            .storage_tries
            .get_mut()
            .entry(address)
            .or_insert_with(|| PartialTrie::open(node_store, *storage_root, U256::from_be_slice));

        *storage_root = trie.trie.root();
        *storage_root
    }

    /// Commit state changes and calculate the new state root
    #[must_use]
    pub fn commit_state(&mut self) -> B256 {
        self.state.trie.root()
    }
}

/// A partial trie that can be updated
#[derive(Debug, Default)]
struct PartialTrie<T> {
    trie: RevealedSparseTrie,
    leafs: HashMap<Nibbles, T>,
}

impl<T: Default> PartialTrie<T> {
    /// Open a partial trie from a root node
    fn open<S: KeyValueStoreGet<B256, TrieNode>, F: FnOnce(&[u8]) -> T + Copy>(
        node_store: &S,
        root: B256,
        parse_leaf: F,
    ) -> Self {
        if root == EMPTY_ROOT_HASH {
            return Self::default();
        }
        let root = node_store.get(&root).unwrap().into_owned();
        let mut state = RevealedSparseTrie::from_root(root.clone(), None, true).unwrap();
        let mut leafs = HashMap::new();
        // traverse the partial trie
        traverse_import_partial_trie(
            &Nibbles::default(),
            &root,
            node_store,
            &mut state,
            &mut |path, value| {
                leafs.insert(path, parse_leaf(value));
            },
        );

        Self { trie: state, leafs }
    }

    fn get(&self, path: &Nibbles) -> Option<&T> {
        self.leafs.get(path)
    }

    fn update_leaf<F: FnMut(&T) -> Vec<u8>>(&mut self, path: Nibbles, value: T, mut encode: F) {
        self.trie
            .update_leaf(path.clone(), encode(&value))
            .expect("update leaf");
        self.leafs.insert(path, value);
    }

    fn remove_leaf(&mut self, path: &Nibbles) {
        self.trie.remove_leaf(path).expect("remove leaf");
        self.leafs.remove(path);
    }
}

fn traverse_import_partial_trie<
    S: KeyValueStoreGet<B256, TrieNode>,
    F: FnMut(Nibbles, &Vec<u8>),
>(
    path: &Nibbles,
    node: &TrieNode,
    nodes: &S,
    trie: &mut RevealedSparseTrie,
    store_leaf: &mut F,
) -> Option<TrieMask> {
    let trie_mask = match node {
        TrieNode::Branch(branch) => {
            let mut trie_mask = TrieMask::default();

            let mut stack_ptr = branch.as_ref().first_child_index();
            for idx in CHILD_INDEX_RANGE {
                if branch.state_mask.is_bit_set(idx) {
                    trie_mask.set_bit(idx);
                    let mut child_path = path.clone();
                    child_path.push(idx);
                    let child = &branch.stack[stack_ptr];
                    stack_ptr += 1;

                    let child_node = if child.len() == B256::len_bytes() + 1 {
                        let hash = B256::from_slice(&child[1..]);

                        match nodes.get(&hash) {
                            Some(node) => node.into_owned(),
                            // the node is not in the witness
                            None => continue,
                        }
                    } else {
                        let mut buf = child.as_ref();
                        let child = TrieNode::decode(&mut buf).unwrap();
                        assert!(buf.is_empty());

                        child
                    };

                    let mask = traverse_import_partial_trie(
                        &child_path,
                        &child_node,
                        nodes,
                        trie,
                        store_leaf,
                    );

                    trie.reveal_node(child_path, child_node, mask).unwrap();
                }
            }
            Some(trie_mask)
        }
        TrieNode::Leaf(leaf) => {
            let mut full = path.clone();
            full.extend_from_slice_unchecked(&leaf.key);
            store_leaf(full, &leaf.value);
            None
        }
        _ => None,
    };

    trie.reveal_node(path.clone(), node.clone(), trie_mask)
        .unwrap();

    trie_mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rpc_types_debug::ExecutionWitness;
    use alloy_rpc_types_eth::Block;
    use std::collections::HashMap;

    const PREV_BLOCK: &str = include_str!("../../../testdata/holesky_witness/0x2ba60c/block.json");
    const BLOCK: &str = include_str!("../../../testdata/holesky_witness/0x2ba60d/block.json");
    const WITNESS: &str = include_str!("../../../testdata/holesky_witness/0x2ba60d/witness.json");

    #[test]
    fn test() {
        let state_root = serde_json::from_str::<Block>(PREV_BLOCK)
            .unwrap()
            .header
            .state_root;

        let block = serde_json::from_str::<Block>(BLOCK).unwrap();

        let state = serde_json::from_str::<ExecutionWitness>(WITNESS)
            .unwrap()
            .state;

        let mut store = HashMap::new();
        decode_nodes(&mut store, state.into_iter().map(|(_, node)| node.0)).unwrap();

        let trie = PartialStateTrie::open(&store, state_root);
        for tx in block.transactions.into_transactions() {
            let _ = trie.get_account(tx.from).unwrap();
            let _ = trie.get_storage(&store, tx.from, U256::ZERO);
            if let Some(to) = tx.to {
                let _ = trie.get_account(to);
            }
        }
    }
}
