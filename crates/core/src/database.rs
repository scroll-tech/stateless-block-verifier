use sbv_kv::{HashMap, KeyValueStoreGet};
use sbv_primitives::{
    Address, B256, Bytes, U256,
    types::revm::{
        AccountInfo, Bytecode,
        database::{BundleAccount, DBErrorMarker},
    },
};
use sbv_trie::{PartialStateTrie, TrieNode};
use std::{cell::RefCell, fmt};

pub use sbv_primitives::types::revm::database::DatabaseRef;

/// A database that consists of account and storage information.
pub struct EvmDatabase<CodeDb, NodesProvider, BlockHashProvider> {
    /// Map of code hash to bytecode.
    pub(crate) code_db: CodeDb,
    /// Cache of analyzed code
    analyzed_code_cache: RefCell<HashMap<B256, Option<Bytecode>>>,
    /// Provider of trie nodes
    pub(crate) nodes_provider: NodesProvider,
    /// Provider of block hashes
    block_hashes: BlockHashProvider,
    /// partial merkle patricia trie
    pub(crate) state: PartialStateTrie,
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

impl<CodeDb, NodesProvider, BlockHashProvider> fmt::Debug
    for EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmDatabase").finish()
    }
}

impl<
    CodeDb: KeyValueStoreGet<B256, Bytes>,
    NodesProvider: KeyValueStoreGet<B256, TrieNode>,
    BlockHashProvider: KeyValueStoreGet<u64, B256>,
> EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>
{
    /// Initialize an EVM database from a zkTrie root.
    pub fn new_from_root(
        code_db: CodeDb,
        state_root_before: B256,
        nodes_provider: NodesProvider,
        block_hashes: BlockHashProvider,
    ) -> Result<Self> {
        dev_trace!("open trie from root {:?}", state_root_before);

        let state = cycle_track!(
            PartialStateTrie::open(&nodes_provider, state_root_before),
            "PartialStateTrie::open"
        )?;

        Ok(EvmDatabase {
            code_db,
            analyzed_code_cache: Default::default(),
            nodes_provider,
            block_hashes,
            state,
        })
    }

    /// Update changes to the database.
    pub fn update<'a, P: KeyValueStoreGet<B256, TrieNode>>(
        &mut self,
        nodes_provider: P,
        post_state: impl IntoIterator<Item = (&'a Address, &'a BundleAccount)>,
    ) -> Result<()> {
        self.state.update(nodes_provider, post_state)?;
        Ok(())
    }

    /// Commit changes and return the new state root.
    pub fn commit_changes(&mut self) -> B256 {
        self.state.commit_state()
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

impl<
    CodeDb: KeyValueStoreGet<B256, Bytes>,
    NodesProvider: KeyValueStoreGet<B256, TrieNode>,
    BlockHashProvider: KeyValueStoreGet<u64, B256>,
> DatabaseRef for EvmDatabase<CodeDb, NodesProvider, BlockHashProvider>
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
        Ok(self
            .state
            .get_storage(&self.nodes_provider, address, index)?
            .unwrap_or(U256::ZERO))
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
