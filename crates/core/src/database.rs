use sbv_kv::{HashMap, KeyValueStoreGet};
use sbv_primitives::{
    AccountInfo, Address, B256, Bytecode, Bytes, U256,
    revm::{db::BundleAccount, interpreter::analysis::to_analysed},
};
use sbv_trie::{PartialStateTrie, TrieNode};
use std::{cell::RefCell, fmt};

pub use sbv_primitives::revm::db::DatabaseRef;

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
    pub fn update<'a, P: KeyValueStoreGet<B256, TrieNode> + Copy>(
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
        use sbv_primitives::predeployed::message_queue;
        self.basic_ref(message_queue::ADDRESS)?
            .ok_or(DatabaseError::MissingL2MessageQueueWitness)?;
        let withdraw_root = self.storage_ref(
            message_queue::ADDRESS,
            message_queue::WITHDRAW_TRIE_ROOT_SLOT,
        )?;
        Ok(withdraw_root.into())
    }

    fn load_code(&self, hash: B256) -> Option<Bytecode> {
        let mut code_cache = self.analyzed_code_cache.borrow_mut();
        if let Some(code) = code_cache.get(&hash) {
            code.clone()
        } else {
            let code = self
                .code_db
                .get(&hash)
                .cloned()
                .map(Bytecode::new_legacy)
                .map(to_analysed);
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
        let Some(account) = self.state.get_account(address) else {
            return Ok(None);
        };
        dev_trace!("load trie account of {address:?}: {account:?}");
        let code = self.load_code(account.code_hash);
        let info = AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            #[cfg(feature = "scroll")]
            code_size: code.as_ref().map(|c| c.len()).unwrap_or(0), // FIXME: this should be remove
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
        // Sometimes the code in previous account info is not contained,
        // and the CacheDB has already loaded the previous account info,
        // then the upcoming trace contains code (meaning the code is used in this new block),
        // we can't directly update the CacheDB, so we offer the code by hash here.
        // However, if the code still cannot be found, this is an error.
        self.load_code(hash).ok_or_else(|| {
            unreachable!(
                "Code is either loaded or not needed (like EXTCODESIZE), code hash: {:?}",
                hash
            );
        })
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

impl From<DatabaseError> for reth_storage_errors::provider::ProviderError {
    fn from(e: DatabaseError) -> Self {
        reth_storage_errors::provider::ProviderError::TrieWitnessError(e.to_string())
    }
}
