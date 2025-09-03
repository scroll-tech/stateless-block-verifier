//! Partial Merkle Patricia Trie
#[macro_use]
extern crate sbv_helpers;

use alloy_trie::{EMPTY_ROOT_HASH, Nibbles, TrieAccount};

use sbv_kv::{HashMap, nohash::NoHashMap};
use sbv_primitives::{Address, B256, Bytes, U256, keccak256, types::revm::database::BundleAccount};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "sanity-check")]
use alloy_trie::{
    TrieMask,
    nodes::{CHILD_INDEX_RANGE, RlpNode, TrieNode},
};
#[cfg(feature = "sanity-check")]
use reth_trie_sparse::{
    SerialSparseTrie, SparseTrieInterface, TrieMasks, provider::DefaultTrieNodeProvider,
};

mod mpt;

/// A partial trie that can be updated
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartialStateTrie<'a> {
    state_trie: mpt::MptNode<'a>,
    storage_tries: NoHashMap<B256, mpt::MptNode<'a>>,
    #[cfg(feature = "sanity-check")]
    reth_state_trie: SerialSparseTrie,
    #[cfg(feature = "sanity-check")]
    reth_storage_tries: NoHashMap<B256, SerialSparseTrie>,
}

/// Partial state trie error
#[derive(thiserror::Error, Debug)]
pub enum PartialStateTrieError {
    /// mpt error
    #[error("error occurred in reth_trie_sparse: {0}")]
    Impl(#[from] mpt::Error),
}

impl<'a> PartialStateTrie<'a> {
    /// Create a partial state trie from a previous state root and a list of RLP-encoded MPT nodes
    pub fn new<I>(prev_state_root: B256, states: I) -> Self
    where
        I: IntoIterator<Item = &'a Bytes>,
    {
        let mut root_node: Option<mpt::MptNode> = None;

        #[cfg(feature = "sanity-check")]
        let mut states_by_hash = NoHashMap::default();

        let mut node_by_hash = NoHashMap::default();
        let mut node_map = HashMap::default();

        for encoded in states.into_iter() {
            let node =
                mpt::MptNode::decode(&mut encoded.as_ref()).expect("Valid MPT node in witness");
            let hash = keccak256(encoded);
            if hash == prev_state_root {
                root_node = Some(node.clone());
            }

            #[cfg(feature = "sanity-check")]
            states_by_hash.insert(hash, encoded.clone());

            node_by_hash.insert(hash, node.clone());
            node_map.insert(node.reference(), node);
        }

        let root = root_node.unwrap_or_else(|| mpt::MptNodeData::Digest(prev_state_root).into());

        let mut storage_roots = Vec::new();
        let state_trie = mpt::resolve_nodes_detect_storage_roots(
            &root,
            &node_map,
            Some(&mut storage_roots),
            Nibbles::default(),
        );

        let mut storage_tries =
            NoHashMap::with_capacity_and_hasher(storage_roots.len(), Default::default());

        #[cfg(feature = "sanity-check")]
        let mut reth_storage_tries =
            NoHashMap::with_capacity_and_hasher(storage_roots.len(), Default::default());
        #[cfg(feature = "sanity-check")]
        let mut reth_state_trie =
            open_trie(&states_by_hash, prev_state_root).expect("Can open state trie");

        for (hashed_address, storage_root) in storage_roots {
            let Some(root_node) = node_by_hash.get(&storage_root) else {
                // An execution witness can include an account leaf (with non-empty storageRoot), but omit
                // its entire storage trie when that account's storage was NOT touched during the block.
                continue;
            };
            let storage_trie = mpt::resolve_nodes(&root_node, &node_map);
            assert_eq!(storage_trie.hash(), storage_root);
            assert!(
                !storage_trie.is_digest(),
                "could not resolve storage trie for {storage_root}"
            );

            #[cfg(feature = "sanity-check")]
            let mut reth_storage_trie =
                open_trie(&states_by_hash, storage_root).expect("Can open storage trie");
            #[cfg(feature = "sanity-check")]
            assert_eq!(reth_storage_trie.root(), storage_root);

            storage_tries.insert(hashed_address, storage_trie);

            #[cfg(feature = "sanity-check")]
            reth_storage_tries.insert(hashed_address, reth_storage_trie);
        }

        #[cfg(feature = "sanity-check")]
        assert_eq!(reth_state_trie.root(), prev_state_root);
        assert_eq!(state_trie.hash(), prev_state_root);

        Self {
            state_trie,
            storage_tries,

            #[cfg(feature = "sanity-check")]
            reth_state_trie,
            #[cfg(feature = "sanity-check")]
            reth_storage_tries,
        }
    }

    /// Get account by address
    #[inline]
    pub fn get_account(
        &self,
        address: Address,
    ) -> Result<Option<TrieAccount>, PartialStateTrieError> {
        let hashed_address = keccak256(address);
        let account = self.state_trie.get_rlp::<TrieAccount>(&*hashed_address)?;

        #[cfg(feature = "sanity-check")]
        {
            let reth_account = self
                .reth_state_trie
                .get_leaf_value(&Nibbles::unpack(&*hashed_address))
                .map(|value| TrieAccount::decode(&mut &**value).unwrap());
            assert_eq!(reth_account, account);
        }

        Ok(account)
    }

    /// Get storage value of an account at a specific slot.
    pub fn get_storage(
        &self,
        address: Address,
        index: U256,
    ) -> Result<U256, PartialStateTrieError> {
        let hashed_address = keccak256(address);

        // Usual case, where given storage slot is present.
        if let Some(storage_trie) = self.storage_tries.get(&hashed_address) {
            let key = keccak256(index.to_be_bytes::<32>());
            let value = storage_trie.get_rlp::<U256>(&*key)?.unwrap_or_default();

            #[cfg(feature = "sanity-check")]
            {
                let reth_storage_trie = self
                    .reth_storage_tries
                    .get(&hashed_address)
                    .expect("reth storage trie must exist if mpt storage trie exists");
                assert_eq!(storage_trie.hash(), reth_storage_trie.clone().root());

                let reth_value = reth_storage_trie
                    .get_leaf_value(&Nibbles::unpack(&*key))
                    .map(|v| U256::decode(&mut &**v).unwrap())
                    .unwrap_or_default();
                assert_eq!(reth_value, value);
            }

            return Ok(value);
        }

        // Storage slot value is not present in the trie, validate that the witness is complete.
        // TODO: Implement witness checks like in reth - https://github.com/paradigmxyz/reth/blob/127595e23079de2c494048d0821ea1f1107eb624/crates/stateless/src/trie.rs#L68C9-L87.
        let account = self.state_trie.get_rlp::<TrieAccount>(&*hashed_address)?;

        #[cfg(feature = "sanity-check")]
        {
            let reth_account = self
                .reth_state_trie
                .get_leaf_value(&Nibbles::unpack(&*hashed_address))
                .map(|value| TrieAccount::decode(&mut &**value).unwrap());
            assert_eq!(reth_account, account);
        }

        match account {
            Some(account) => {
                if account.storage_root != EMPTY_ROOT_HASH {
                    todo!("Validate that storage witness is valid");
                }
            }
            None => {
                todo!("Validate that account witness is valid");
            }
        }

        // Account doesn't exist or has empty storage root.
        Ok(U256::ZERO)
    }

    /// Mutates state based on diffs provided in [`HashedPostState`].
    pub fn update(
        &mut self,
        post_state: BTreeMap<Address, BundleAccount>,
    ) -> Result<B256, PartialStateTrieError> {
        for (address, account) in post_state.into_iter() {
            dev_trace!("update account: {address} {:?}", account.info);
            let address_hash = keccak256(address);

            #[cfg(feature = "sanity-check")]
            let address_path = Nibbles::unpack(&*address_hash);

            if account.was_destroyed() {
                self.state_trie.delete(&*address_hash)?;

                #[cfg(feature = "sanity-check")]
                self.reth_state_trie
                    .remove_leaf(&address_path, DefaultTrieNodeProvider)
                    .unwrap();

                continue;
            }

            let original_account = self.state_trie.get_rlp::<TrieAccount>(&*address_hash)?;
            let original_storage_root = original_account
                .as_ref()
                .map(|acc| acc.storage_root)
                .unwrap_or(EMPTY_ROOT_HASH);

            let storage_root = if !account.storage.is_empty() {
                dev_trace!("non-empty storage, trie needs to be updated");

                let storage_trie = self.storage_tries.entry(address_hash).or_default();
                debug_assert_eq!(storage_trie.hash(), original_storage_root);

                #[cfg(feature = "sanity-check")]
                let reth_storage_trie = self.reth_storage_tries.entry(address_hash).or_default();
                #[cfg(feature = "sanity-check")]
                assert_eq!(storage_trie.hash(), reth_storage_trie.root());

                dev_trace!(
                    "opened storage trie of {address} at {}",
                    storage_trie.hash()
                );

                for (key, slot) in BTreeMap::from_iter(account.storage.clone()) {
                    let key_hash = keccak256(key.to_be_bytes::<{ U256::BYTES }>());
                    dev_trace!(
                        "update storage of {address}: {key:#064X}={:#064X}, key_hash={key_hash}",
                        slot.present_value
                    );

                    #[cfg(feature = "sanity-check")]
                    let key_path = Nibbles::unpack(&*key_hash);

                    if slot.present_value.is_zero() {
                        storage_trie.delete(&*key_hash)?;

                        #[cfg(feature = "sanity-check")]
                        reth_storage_trie
                            .remove_leaf(&key_path, DefaultTrieNodeProvider)
                            .unwrap();
                    } else {
                        storage_trie.insert_rlp(&*key_hash, slot.present_value)?;

                        #[cfg(feature = "sanity-check")]
                        reth_storage_trie
                            .update_leaf(
                                key_path,
                                slot.present_value.to_rlp(),
                                DefaultTrieNodeProvider,
                            )
                            .unwrap();
                    }

                    #[cfg(feature = "sanity-check")]
                    assert_eq!(storage_trie.hash(), reth_storage_trie.root());
                }
                storage_trie.hash()
            } else {
                original_storage_root
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
            self.state_trie.insert_rlp(&*address_hash, account)?;

            #[cfg(feature = "sanity-check")]
            self.reth_state_trie
                .update_leaf(address_path, account.to_rlp(), DefaultTrieNodeProvider)
                .unwrap();
        }

        #[cfg(feature = "sanity-check")]
        assert_eq!(self.state_trie.hash(), self.reth_state_trie.root());

        Ok(self.state_trie.hash())
    }
}

#[inline(always)]
#[cfg(feature = "sanity-check")]
fn open_trie<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
    nodes_provider: &P,
    root: B256,
) -> Result<SerialSparseTrie, PartialStateTrieError> {
    if root == EMPTY_ROOT_HASH {
        return Ok(SerialSparseTrie::default());
    }
    let root_node = nodes_provider.get(&root).unwrap();
    let root = TrieNode::decode(&mut root_node.as_ref()).unwrap();
    let mut trie = SerialSparseTrie::from_root(root.clone(), TrieMasks::none(), false).unwrap();
    cycle_track!(
        traverse_import_partial_trie(Nibbles::default(), root, nodes_provider, &mut trie),
        "traverse_import_partial_trie"
    )?;
    Ok(trie)
}

#[inline(always)]
#[cfg(feature = "sanity-check")]
fn traverse_import_partial_trie<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
    path: Nibbles,
    node: TrieNode,
    nodes: &P,
    trie: &mut SerialSparseTrie,
) -> Result<(), PartialStateTrieError> {
    match node {
        TrieNode::EmptyRoot => trie.reveal_node(path, node, TrieMasks::none()).unwrap(),
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
            trie.reveal_node(path, node, trie_mask).unwrap();
        }
        TrieNode::Leaf(_) => trie.reveal_node(path, node, TrieMasks::none()).unwrap(),
        TrieNode::Extension(ref extension) => {
            let mut child_path = path;
            child_path.extend(&extension.key);

            if let Some(child_node) = decode_rlp_node(nodes, &extension.child)? {
                traverse_import_partial_trie(child_path, child_node, nodes, trie)?;
            }
            trie.reveal_node(path, node, TrieMasks::none()).unwrap();
        }
    };

    Ok(())
}

#[inline(always)]
#[cfg(feature = "sanity-check")]
fn decode_rlp_node<P: sbv_kv::KeyValueStoreGet<B256, Bytes>>(
    nodes_provider: P,
    node: &RlpNode,
) -> Result<Option<TrieNode>, PartialStateTrieError> {
    if node.len() == B256::len_bytes() + 1 {
        let hash = B256::from_slice(&node[1..]);
        let Some(node_bytes) = nodes_provider.get(&hash) else {
            return Ok(None);
        };
        Ok(Some(TrieNode::decode(&mut node_bytes.as_ref()).unwrap()))
    } else {
        let mut buf = node.as_ref();
        Ok(Some(TrieNode::decode(&mut buf).unwrap()))
    }
}

#[cfg(test)]
#[cfg(feature = "sanity-check")]
mod tests {
    use super::*;
    use crate::mpt::{MptNode, RlpBytes};
    use reth_trie_sparse::provider::DefaultTrieNodeProvider;
    use reth_trie_sparse::{SerialSparseTrie, SparseTrieInterface};
    use sbv_primitives::address;

    #[test]
    fn test_storage_trie() {
        let mut rsp_trie = MptNode::default();
        let mut reth_trie = SerialSparseTrie::default();

        assert_eq!(rsp_trie.hash(), reth_trie.root());

        let index = U256::from(0x01);
        let key_hash = keccak256(index.to_be_bytes::<32>());
        let value = U256::from(0xdeadbeefu64);

        rsp_trie.insert_rlp(&*key_hash, value).unwrap();
        reth_trie
            .update_leaf(
                Nibbles::unpack(key_hash),
                value.to_rlp(),
                DefaultTrieNodeProvider,
            )
            .unwrap();
        assert_eq!(rsp_trie.hash(), reth_trie.root());
    }

    #[test]
    fn test_state_trie() {
        let mut rsp_trie = MptNode::default();
        let mut reth_trie = SerialSparseTrie::default();

        assert_eq!(rsp_trie.hash(), reth_trie.root());

        let address = address!("deadbeef00000000000000000000000000000000");
        let addr_hash = keccak256(address);
        let account = TrieAccount {
            nonce: 1u64,
            balance: U256::from(0xdeadbeefu64),
            storage_root: mpt::EMPTY_ROOT_HASH,
            code_hash: B256::ZERO,
        };
        rsp_trie.insert_rlp(&*addr_hash, account.clone()).unwrap();
        reth_trie
            .update_leaf(
                Nibbles::unpack(addr_hash),
                account.to_rlp(),
                DefaultTrieNodeProvider,
            )
            .unwrap();
        assert_eq!(rsp_trie.hash(), reth_trie.root());
    }

    #[test]
    fn test_state_trie_random() {
        let mut rsp_trie = MptNode::default();
        let mut reth_trie = SerialSparseTrie::default();

        assert_eq!(rsp_trie.hash(), reth_trie.root());

        for i in 0..10000u64 {
            let address = Address::left_padding_from(&i.to_be_bytes());
            let addr_hash = keccak256(address);
            let account = TrieAccount {
                nonce: i,
                balance: U256::from(i),
                storage_root: B256::from(U256::from(i)),
                code_hash: B256::from(U256::from(i)),
            };
            rsp_trie.insert_rlp(&*addr_hash, account.clone()).unwrap();
            reth_trie
                .update_leaf(
                    Nibbles::unpack(addr_hash),
                    account.to_rlp(),
                    DefaultTrieNodeProvider,
                )
                .unwrap();
            assert_eq!(rsp_trie.hash(), reth_trie.root());
        }
        assert_eq!(rsp_trie.hash(), reth_trie.root());
    }
}
