//! Partial Merkle Patricia Trie

extern crate core;

use alloy_rlp::{Decodable, Encodable};
use alloy_trie::{nodes::CHILD_INDEX_RANGE, Nibbles, TrieMask, EMPTY_ROOT_HASH};
use reth_trie_sparse::RevealedSparseTrie;
use revm::db::BundleAccount;
use sbv_kv::{KeyValueStoreGet, KeyValueStoreInsert};
use sbv_primitives::{keccak256, Address, B256, U256};
use std::cell::RefCell;
use std::collections::HashMap;

pub use alloy_trie::{nodes::TrieNode, TrieAccount};
pub use reth_trie::{KeccakKeyHasher, KeyHasher};
use sbv_helpers::dev_trace;

/// Fill a KeyValueStore<B256, TrieNode> from a list of nodes
pub fn decode_nodes<
    B: AsRef<[u8]>,
    P: KeyValueStoreInsert<B256, TrieNode>,
    I: Iterator<Item = B>,
>(
    provider: &mut P,
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
    storage_roots: RefCell<HashMap<B256, B256>>,
    /// hashed address -> storage tire
    storage_tries: RefCell<HashMap<B256, PartialTrie<U256>>>,
    /// shared rlp buffer
    rlp_buffer: Vec<u8>,
}

impl PartialStateTrie {
    /// Open a partial trie from a root node
    pub fn open<P: KeyValueStoreGet<B256, TrieNode> + Copy>(nodes_provider: P, root: B256) -> Self {
        let state = PartialTrie::open(nodes_provider, root, decode_trie_account);

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
    pub fn get_storage<P: KeyValueStoreGet<B256, TrieNode> + Copy>(
        &self,
        nodes_provider: P,
        address: Address,
        index: U256,
    ) -> Option<U256> {
        let hashed_address = self.hashed_address(address);
        let storage_root = *self.storage_roots.borrow().get(&hashed_address)?;
        let path = Nibbles::unpack(keccak256(index.to_be_bytes::<32>()));

        self.storage_tries
            .borrow_mut()
            .entry(hashed_address)
            .or_insert_with(|| PartialTrie::open(nodes_provider, storage_root, decode_u256_rlp))
            .get(&path)
            .copied()
    }

    /// Commit state changes and calculate the new state root
    #[must_use]
    pub fn commit_state(&mut self) -> B256 {
        self.state.trie.root()
    }

    /// Update the trie with the new state
    pub fn update<'a, P: KeyValueStoreGet<B256, TrieNode> + Copy>(
        &mut self,
        nodes_provider: P,
        post_state: impl IntoIterator<Item = (&'a Address, &'a BundleAccount)>,
    ) {
        for (address, account) in post_state.into_iter() {
            dev_trace!("update account: {address} {:?}", account.info);
            let hashed_address = self.hashed_address(*address);
            let account_path = Nibbles::unpack(&hashed_address);

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
                    PartialTrie::open(nodes_provider, storage_root, decode_u256_rlp)
                });

            for (key, slot) in account.storage.iter() {
                let path = Nibbles::unpack(keccak256(key.to_be_bytes::<32>()));

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
            .or_insert_with(|| keccak256(address))
    }

    /// Update the account
    #[inline(always)]
    fn update_account(&mut self, hashed_address: B256, account: TrieAccount) {
        let account_path = Nibbles::unpack(&hashed_address);
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
    fn open<P: KeyValueStoreGet<B256, TrieNode> + Copy, F: FnOnce(&[u8]) -> T + Copy>(
        nodes_provider: P,
        root: B256,
        parse_leaf: F,
    ) -> Self {
        if root == EMPTY_ROOT_HASH {
            return Self::default();
        }
        let root = nodes_provider.get(&root).unwrap().into_owned();
        let mut state = RevealedSparseTrie::from_root(root.clone(), None, true).unwrap();
        let mut leafs = HashMap::new();
        // traverse the partial trie
        traverse_import_partial_trie(
            &Nibbles::default(),
            &root,
            nodes_provider,
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
    P: KeyValueStoreGet<B256, TrieNode> + Copy,
    F: FnMut(Nibbles, &Vec<u8>),
>(
    path: &Nibbles,
    node: &TrieNode,
    nodes: P,
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

fn decode_trie_account(mut buf: &[u8]) -> TrieAccount {
    let acc = TrieAccount::decode(&mut buf).unwrap();
    assert!(buf.is_empty(), "the leaf should only contains the account");
    acc
}

fn decode_u256_rlp(mut buf: &[u8]) -> U256 {
    let value = U256::decode(&mut buf).unwrap();
    assert!(buf.is_empty(), "the leaf should only contains the value");
    value
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
