//! Partial Merkle Patricia Trie
#[macro_use]
extern crate sbv_helpers;

use crate::mpt::MptNode;
use alloy_trie::{EMPTY_ROOT_HASH, TrieAccount};
use sbv_primitives::{Address, B256, Bytes, U256, keccak256, types::revm::database::BundleAccount};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod execution_witness;
mod mpt;
pub mod r0;

pub use execution_witness::FromWitnessError;
pub use reth_trie::{HashedPostState, KeccakKeyHasher};
use sbv_primitives::alloy_primitives::map::B256Map;

/// A partial trie that can be updated
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartialStateTrie {
    state_trie: MptNode,
    storage_tries: B256Map<MptNode>,
}

/// Partial state trie error
#[derive(thiserror::Error, Debug)]
pub enum PartialStateTrieError {
    /// mpt error
    #[error("error occurred in reth_trie_sparse: {0}")]
    Impl(#[from] mpt::Error),
}

impl PartialStateTrie {
    /// Create a partial state trie from a previous state root and a list of RLP-encoded MPT nodes
    pub fn new<'a, I>(
        prev_state_root: B256,
        states: I,
    ) -> Result<PartialStateTrie, FromWitnessError>
    where
        I: IntoIterator<Item = &'a Bytes>,
    {
        let (state_trie, storage_tries) =
            execution_witness::build_validated_tries(prev_state_root, states)?;

        Ok(PartialStateTrie {
            state_trie,
            storage_tries,
        })
    }

    /// Get account by address
    #[inline]
    pub fn get_account(
        &self,
        address: Address,
    ) -> Result<Option<TrieAccount>, PartialStateTrieError> {
        let hashed_address = keccak256(address);
        let account = self.state_trie.get_rlp::<TrieAccount>(&*hashed_address)?;

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
        let account = self.state_trie.get_rlp::<TrieAccount>(&*hashed_address)?;
        match account {
            Some(account) => {
                if account.storage_root != EMPTY_ROOT_HASH {
                    unreachable!("pre-built storage trie shall be present");
                }
            }
            None => {
                println!("[TRIGGERED] CASE 2. STATE TRIE EMPTY NODE PROOF");
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
