//! This is copied and modified from https://github.com/succinctlabs/rsp
//! crates/mpt/src/execution_witness.rs rev@2a99f35a9b81452eb53af3848e50addfd481363c
//! Under MIT license
use crate::mpt::{MptNode, MptNodeData, MptNodeReference, resolve_nodes};
use alloy_rlp::Decodable;
use reth_trie::TrieAccount;
use sbv_kv::{HashMap, nohash::NoHashMap};
use sbv_primitives::{B256, Bytes, keccak256};

/// Partial state trie error
#[derive(thiserror::Error, Debug)]
pub enum FromWitnessError {
    /// rlp error
    #[error("rlp error: {0}")]
    Rlp(#[from] alloy_rlp::Error),
    /// trie error
    #[error(transparent)]
    Trie(#[from] crate::mpt::Error),
    /// missing storage trie witness
    #[error(
        "missing storage trie witness for {hashed_address:?} with storage root {storage_root:?}"
    )]
    MissingStorageTrie {
        /// The keccak256 hash of the account address
        hashed_address: B256,
        /// The storage root of the account
        storage_root: B256,
    },
    /// state trie validation error
    #[error("mismatched state root: expected {expected:?}, got {actual:?}")]
    StateTrieValidation {
        /// The expected state root hash
        expected: B256,
        /// The actual computed state root hash
        actual: B256,
    },
    /// missing account in state trie
    #[error("account not found in state trie")]
    MissingAccount,
    /// storage trie validation error
    #[error(
        "mismatched storage root for address hash {hashed_address:?}: expected {expected_hash:?}, got {actual_hash:?}"
    )]
    StorageTrieValidation {
        /// The keccak256 hash of the account address
        hashed_address: B256,
        /// The expected storage root hash from the account
        expected_hash: B256,
        /// The actual computed storage root hash
        actual_hash: B256,
    },
}

// Builds tries from the witness state.
//
// NOTE: This method should be called outside zkVM! In general, you construct tries, then
// validate them inside zkVM.
pub(crate) fn build_validated_tries<'a, I>(
    prev_state_root: B256,
    states: I,
) -> Result<(MptNode, NoHashMap<B256, MptNode>), FromWitnessError>
where
    I: IntoIterator<Item = &'a Bytes>,
{
    // Step 1: Decode all RLP-encoded trie nodes and index by hash
    // IMPORTANT: Witness state contains both *state trie* nodes and *storage tries* nodes!
    let mut node_map = HashMap::<MptNodeReference, MptNode>::default();
    let mut node_by_hash = NoHashMap::<B256, MptNode>::default();
    let mut root_node: Option<MptNode> = None;

    for encoded in states.into_iter() {
        let node = MptNode::decode(&mut encoded.as_ref())?;
        let hash = keccak256(encoded);
        if hash == prev_state_root {
            root_node = Some(node.clone());
        }
        node_by_hash.insert(hash, node.clone());
        node_map.insert(node.reference(), node);
    }

    // Step 2: Use root_node or fallback to Digest
    let root = root_node.unwrap_or_else(|| MptNodeData::Digest(prev_state_root).into());

    // Build state trie.
    let mut raw_storage_tries = Vec::with_capacity(node_by_hash.len());
    let state_trie = resolve_nodes(&root, &node_map);

    state_trie.for_each_leaves(|key, mut value| {
        let account = TrieAccount::decode(&mut value).unwrap();
        let hashed_address = B256::from_slice(key);
        raw_storage_tries.push((hashed_address, account.storage_root));
    });

    // Step 3: Build storage tries per account efficiently
    let mut storage_tries = NoHashMap::<B256, MptNode>::with_capacity_and_hasher(
        raw_storage_tries.len(),
        Default::default(),
    );

    for (hashed_address, storage_root) in raw_storage_tries {
        let root_node = match node_by_hash.get(&storage_root).cloned() {
            Some(node) => node,
            None => {
                // An execution witness can include an account leaf (with non-empty storageRoot),
                // but omit its entire storage trie when that account's storage was
                // NOT touched during the block.
                continue;
            }
        };
        let storage_trie = resolve_nodes(&root_node, &node_map);

        if storage_trie.is_digest() {
            return Err(FromWitnessError::MissingStorageTrie {
                hashed_address,
                storage_root,
            });
        }

        // Insert resolved storage trie.
        storage_tries.insert(hashed_address, storage_trie);
    }

    // Step 3a: Verify that state_trie was built correctly - confirm tree hash with pre state root.
    validate_state_trie(&state_trie, prev_state_root)?;

    // Step 3b: Verify that each storage trie matches the declared storage_root in the state trie.
    validate_storage_tries(&state_trie, &storage_tries)?;

    Ok((state_trie, storage_tries))
}

// Validate that state_trie was built correctly - confirm tree hash with prev state root.
fn validate_state_trie(state_trie: &MptNode, pre_state_root: B256) -> Result<(), FromWitnessError> {
    if state_trie.hash() != pre_state_root {
        return Err(FromWitnessError::StateTrieValidation {
            expected: pre_state_root,
            actual: state_trie.hash(),
        });
    }
    Ok(())
}

// Validates that each storage trie matches the declared storage_root in the state trie.
fn validate_storage_tries(
    state_trie: &MptNode,
    storage_tries: &NoHashMap<B256, MptNode>,
) -> Result<(), FromWitnessError> {
    for (hashed_address, storage_trie) in storage_tries.iter() {
        let account = state_trie
            .get_rlp::<TrieAccount>(hashed_address.as_slice())?
            .ok_or(FromWitnessError::MissingAccount)?;

        let storage_root = account.storage_root;
        let actual_hash = storage_trie.hash();

        if storage_root != actual_hash {
            return Err(FromWitnessError::StorageTrieValidation {
                hashed_address: *hashed_address,
                expected_hash: storage_root,
                actual_hash,
            });
        }
    }

    Ok(())
}
