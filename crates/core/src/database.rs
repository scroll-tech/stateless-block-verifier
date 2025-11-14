//! Most copied from <https://github.com/paradigmxyz/reth/blob/5c18df9889941837e61929be4b51abb75f07f152/crates/stateless/src/witness_db.rs>
//! Under MIT license

use reth_stateless::StatelessTrie;
pub use sbv_primitives::types::revm::database::Database;
use sbv_primitives::{
    Address, B256, U256,
    alloy_primitives::map::B256Map,
    types::{
        reth::evm::execute::ProviderError,
        revm::{AccountInfo, Bytecode},
    },
};
use sbv_trie::SparseState;
use std::collections::BTreeMap;

/// A database that consists of account and storage information.
#[derive(Debug)]
pub struct WitnessDatabase<'a> {
    /// Map of block numbers to block hashes.
    /// This is used to service the `BLOCKHASH` opcode.
    block_hashes_by_block_number: &'a BTreeMap<u64, B256>,
    /// Map of code hashes to bytecode.
    /// Used to fetch contract code needed during execution.
    bytecode: &'a B256Map<Bytecode>,
    /// The sparse Merkle Patricia Trie containing account and storage state.
    /// This is used to provide account/storage values during EVM execution.
    pub(crate) trie: &'a SparseState,
}

impl<'a> WitnessDatabase<'a> {
    /// Creates a new [`WitnessDatabase`] instance.
    ///
    /// # Assumptions
    ///
    /// This function assumes:
    /// 1. The provided `trie` has been populated with state data consistent with a known state root
    ///    (e.g., using witness data and verifying against a parent block's state root).
    /// 2. The `bytecode` map contains all bytecode corresponding to code hashes present in the
    ///    account data within the `trie`.
    /// 3. The `ancestor_hashes` map contains the block hashes for the relevant ancestor blocks (up
    ///    to 256 including the current block number). It assumes these hashes correspond to a
    ///    contiguous chain of blocks. The caller is responsible for verifying the contiguity and
    ///    the block limit.
    pub(crate) const fn new(
        trie: &'a SparseState,
        bytecode: &'a B256Map<Bytecode>,
        ancestor_hashes: &'a BTreeMap<u64, B256>,
    ) -> Self {
        Self {
            trie,
            block_hashes_by_block_number: ancestor_hashes,
            bytecode,
        }
    }
}

impl Database for WitnessDatabase<'_> {
    /// The database error type.
    type Error = ProviderError;

    /// Get basic account information by hashing the address and looking up the account RLP
    /// in the underlying [`StatelessTrie`] implementation.
    ///
    /// Returns `Ok(None)` if the account is not found in the trie.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.trie.account(address).map(|opt| {
            opt.map(|account| AccountInfo {
                balance: account.balance,
                nonce: account.nonce,
                code_hash: account.code_hash,
                code: None,
            })
        })
    }

    /// Get account code by its hash from the provided bytecode map.
    ///
    /// Returns an error if the bytecode for the given hash is not found in the map.
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.bytecode.get(&code_hash).cloned().ok_or_else(|| {
            ProviderError::TrieWitnessError(format!("bytecode for {code_hash} not found"))
        })
    }

    /// Get storage value of an account at a specific slot.
    ///
    /// Returns `U256::ZERO` if the slot is not found in the trie.
    fn storage(&mut self, address: Address, slot: U256) -> Result<U256, Self::Error> {
        self.trie.storage(address, slot)
    }

    /// Get block hash by block number from the provided ancestor hashes map.
    ///
    /// Returns an error if the hash for the given block number is not found in the map.
    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> {
        self.block_hashes_by_block_number
            .get(&block_number)
            .copied()
            .ok_or(ProviderError::StateForNumberNotFound(block_number))
    }
}
