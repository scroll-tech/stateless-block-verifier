//! Partial Merkle Patricia Trie
#[macro_use]
extern crate sbv_helpers;

use alloy_trie::{EMPTY_ROOT_HASH, Nibbles, TrieAccount};
use sbv_kv::{HashMap, nohash::NoHashMap};
use sbv_primitives::{Address, B256, Bytes, U256, keccak256, types::revm::database::BundleAccount};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod mpt;

/// A partial trie that can be updated
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartialStateTrie<'a> {
    state_trie: mpt::MptNode<'a>,
    storage_tries: NoHashMap<B256, mpt::MptNode<'a>>,
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
        let mut node_by_hash = NoHashMap::default();
        let mut node_map = HashMap::default();

        for encoded in states.into_iter() {
            let node =
                mpt::MptNode::decode(&mut encoded.as_ref()).expect("Valid MPT node in witness");
            let hash = keccak256(encoded);
            if hash == prev_state_root {
                root_node = Some(node.clone());
            }

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

            storage_tries.insert(hashed_address, storage_trie);
        }
        assert_eq!(state_trie.hash(), prev_state_root);

        Self {
            state_trie,
            storage_tries,
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
            return Ok(value);
        }

        // Storage slot value is not present in the trie, validate that the witness is complete.
        // TODO: Implement witness checks like in reth - https://github.com/paradigmxyz/reth/blob/127595e23079de2c494048d0821ea1f1107eb624/crates/stateless/src/trie.rs#L68C9-L87.
        let account = self.state_trie.get_rlp::<TrieAccount>(&*hashed_address)?;

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

            if account.was_destroyed() {
                self.state_trie.delete(&*address_hash)?;
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

                    if slot.present_value.is_zero() {
                        storage_trie.delete(&*key_hash)?;
                    } else {
                        storage_trie.insert_rlp(&*key_hash, slot.present_value)?;
                    }
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
        }

        Ok(self.state_trie.hash())
    }
}
