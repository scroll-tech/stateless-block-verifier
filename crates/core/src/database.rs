use sbv_kv::{HashMap, KeyValueStoreGet};
pub use sbv_primitives::types::revm::database::DatabaseRef;
use sbv_primitives::{
    Address, B256, Bytes, U256,
    types::revm::{
        AccountInfo, Bytecode,
        database::{BundleAccount, DBErrorMarker},
    },
};
use sbv_trie::PartialStateTrie;
use std::{cell::RefCell, collections::BTreeMap, fmt};

/// A database that consists of account and storage information.
pub struct EvmDatabase<CodeDb, BlockHashProvider> {
    /// Map of code hash to bytecode.
    pub(crate) code_db: CodeDb,
    /// Cache of analyzed code
    analyzed_code_cache: RefCell<HashMap<B256, Option<Bytecode>>>,
    /// partial merkle patricia trie
    pub(crate) state: PartialStateTrie,
    /// Provider of block hashes
    block_hashes: BlockHashProvider,
}

/// Database error.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Missing L2 message queue witness
    #[cfg(feature = "scroll")]
    #[error("missing L2 message queue witness")]
    MissingL2MessageQueueWitness,
    /// Partial state trie error
    #[error(transparent)]
    PartialStateTrie(#[from] sbv_trie::PartialStateTrieError),
    /// Requested code not loaded
    #[error("requested code({0}) not loaded")]
    CodeNotLoaded(B256),
}

type Result<T, E = DatabaseError> = std::result::Result<T, E>;

impl<CodeDb, BlockHashProvider> fmt::Debug for EvmDatabase<CodeDb, BlockHashProvider> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmDatabase").finish()
    }
}

impl<CodeDb: KeyValueStoreGet<B256, Bytes>, BlockHashProvider: KeyValueStoreGet<u64, B256>>
    EvmDatabase<CodeDb, BlockHashProvider>
{
    /// Initialize an EVM database from a zkTrie root.
    pub fn new(code_db: CodeDb, state: PartialStateTrie, block_hashes: BlockHashProvider) -> Self {
        EvmDatabase {
            code_db,
            analyzed_code_cache: Default::default(),
            block_hashes,
            state,
        }
    }

    /// Update changes to the database.
    pub fn commit(&mut self, post_state: BTreeMap<Address, BundleAccount>) -> Result<B256> {
        Ok(self.state.update(post_state)?)
    }

    /// Get the withdrawal trie root of scroll.
    ///
    /// Note: this should not be confused with the withdrawal of the beacon chain.
    #[cfg(feature = "scroll")]
    pub fn withdraw_root(&self) -> Result<B256, DatabaseError> {
        /// L2MessageQueue pre-deployed address
        pub const ADDRESS: Address =
            sbv_primitives::address!("5300000000000000000000000000000000000000");
        /// the slot of withdraw root in L2MessageQueue
        pub const WITHDRAW_TRIE_ROOT_SLOT: U256 = U256::ZERO;

        self.basic_ref(ADDRESS)?
            .ok_or(DatabaseError::MissingL2MessageQueueWitness)?;
        let withdraw_root = self.storage_ref(ADDRESS, WITHDRAW_TRIE_ROOT_SLOT)?;
        Ok(withdraw_root.into())
    }

    fn load_code(&self, hash: B256) -> Option<Bytecode> {
        let mut code_cache = self.analyzed_code_cache.borrow_mut();
        if let Some(code) = code_cache.get(&hash) {
            code.clone()
        } else {
            let code = self.code_db.get(&hash).cloned().map(Bytecode::new_raw);
            code_cache.insert(hash, code.clone());
            code
        }
    }
}

impl<CodeDb: KeyValueStoreGet<B256, Bytes>, BlockHashProvider: KeyValueStoreGet<u64, B256>>
    DatabaseRef for EvmDatabase<CodeDb, BlockHashProvider>
{
    type Error = DatabaseError;

    /// Get basic account information.
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let Some(account) = self.state.get_account(address)? else {
            return Ok(None);
        };
        dev_trace!("load trie account of {address:?}: {account:?}");
        let code = self.load_code(account.code_hash);
        let info = AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            code_hash: account.code_hash,
            code,
        };

        #[cfg(debug_assertions)]
        if let Some(ref code) = info.code {
            assert_eq!(
                info.code_hash,
                code.hash_slow(),
                "code hash mismatch for account {address:?}",
            );
        }

        Ok(Some(info))
    }

    /// Get account code by its code hash.
    fn code_by_hash_ref(&self, hash: B256) -> Result<Bytecode, Self::Error> {
        self.load_code(hash)
            .ok_or(DatabaseError::CodeNotLoaded(hash))
    }

    /// Get storage value of address at index.
    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        dev_trace!("get storage of {:?} at index {:?}", address, index);
        Ok(self.state.get_storage(address, index)?)
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        Ok(*self
            .block_hashes
            .get(&number)
            .unwrap_or_else(|| panic!("block hash of number {number} not found")))
    }
}

impl DBErrorMarker for DatabaseError {}
