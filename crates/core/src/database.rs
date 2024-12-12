use revm::interpreter::analysis::to_analysed;
use revm::{
    db::DatabaseRef,
    primitives::{AccountInfo, Address, Bytecode, Bytes, B256, U256},
};
use sbv_kv::KeyValueStoreGet;
use sbv_trie::{PartialStateTrie, TrieNode};
use std::convert::Infallible;
use std::{cell::RefCell, collections::HashMap, fmt};

/// A database that consists of account and storage information.
pub struct EvmDatabase<CodeDb, NodesProvider> {
    /// Map of code hash to bytecode.
    pub(crate) code_db: CodeDb,
    /// Cache of analyzed code
    analyzed_code_cache: RefCell<HashMap<B256, Option<Bytecode>>>,
    /// Provider of trie nodes
    pub(crate) nodes_provider: NodesProvider,
    /// partial merkle patricia trie
    pub(crate) state: PartialStateTrie,
}

impl<CodeDb, Db> fmt::Debug for EvmDatabase<CodeDb, Db> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmDatabase").finish()
    }
}

impl<CodeDb: KeyValueStoreGet<B256, Bytes>, NodesProvider: KeyValueStoreGet<B256, TrieNode>>
    EvmDatabase<CodeDb, NodesProvider>
{
    /// Initialize an EVM database from a zkTrie root.
    pub fn new_from_root(
        code_db: CodeDb,
        state_root_before: B256,
        nodes_provider: NodesProvider,
    ) -> Self {
        let state = PartialStateTrie::open(&nodes_provider, state_root_before);

        EvmDatabase {
            code_db,
            analyzed_code_cache: Default::default(),
            nodes_provider,
            state,
        }
    }

    fn load_code(&self, hash: B256) -> Option<Bytecode> {
        let mut code_cache = self.analyzed_code_cache.borrow_mut();
        if let Some(code) = code_cache.get(&hash) {
            code.clone()
        } else {
            let code = self
                .code_db
                .get(&hash)
                .map(|v| to_analysed(Bytecode::new_legacy(v.into_owned())));
            code_cache.insert(hash, code.clone());
            code
        }
    }
}

impl<CodeDb: KeyValueStoreGet<B256, Bytes>, NodesProvider: KeyValueStoreGet<B256, TrieNode>>
    DatabaseRef for EvmDatabase<CodeDb, NodesProvider>
{
    type Error = Infallible;

    /// Get basic account information.
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let Some(account) = self.state.get_account(address) else {
            return Ok(None);
        };
        dev_trace!("load trie account of {:?}", address);

        let info = AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            code_hash: account.code_hash,
            code: self.load_code(account.code_hash),
            ..Default::default()
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
            .get_storage(&self.nodes_provider, address, index)
            .unwrap_or(U256::ZERO))
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, _: u64) -> Result<B256, Self::Error> {
        unreachable!("BLOCKHASH is disabled")
    }
}
