use crate::error::DatabaseError;
use revm::interpreter::analysis::to_analysed;
use revm::{
    db::DatabaseRef,
    primitives::{AccountInfo, Address, Bytecode, B256, U256},
};
use sbv_primitives::{
    zk_trie::{
        db::{
            kv::{KVDatabase, KVDatabaseItem},
            NodeDb,
        },
        hash::{key_hasher::NoCacheHasher, HashScheme, ZkHash},
        scroll_types::Account,
        trie::ZkTrie,
    },
    Block,
};
use std::{cell::RefCell, collections::HashMap, fmt};

type Result<T, E = DatabaseError> = std::result::Result<T, E>;

/// A database that consists of account and storage information.
pub struct EvmDatabase<'a, CodeDb, ZkDb, H> {
    /// Map of code hash to bytecode.
    pub(crate) code_db: &'a mut CodeDb,
    /// Cache of analyzed code
    analyzed_code_cache: RefCell<HashMap<B256, Option<Bytecode>>>,
    /// Storage root cache, avoid re-query account when storage root is needed
    storage_root_caches: RefCell<HashMap<Address, ZkHash>>,
    /// Storage trie cache, avoid re-creating trie for the same account.
    /// Need to invalidate before `update`, otherwise the trie root may be outdated
    storage_trie_caches: RefCell<HashMap<ZkHash, Option<ZkTrie<H>>>>,
    /// Current uncommitted zkTrie root based on the block trace.
    committed_zktrie_root: B256,
    /// The underlying zkTrie database.
    pub(crate) zktrie_db: &'a mut NodeDb<ZkDb>,
    /// Current view of zkTrie database.
    zktrie: ZkTrie<H>,
}

impl<CodeDb, Db, HashScheme> fmt::Debug for EvmDatabase<'_, CodeDb, Db, HashScheme> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EvmDatabase")
            .field("committed_zktrie_root", &self.committed_zktrie_root)
            .finish()
    }
}

impl<'a, CodeDb: KVDatabase, ZkDb: KVDatabase + 'static, H: HashScheme>
    EvmDatabase<'a, CodeDb, ZkDb, H>
{
    /// Initialize an EVM database from a zkTrie root.
    pub fn new_from_root(
        committed_zktrie_root: B256,
        code_db: &'a mut CodeDb,
        zktrie_db: &'a mut NodeDb<ZkDb>,
    ) -> Result<Self> {
        let zktrie = ZkTrie::new_with_root(zktrie_db, NoCacheHasher, committed_zktrie_root)
            .map_err(DatabaseError::zk_trie)?;

        Ok(EvmDatabase {
            code_db,
            analyzed_code_cache: Default::default(),
            storage_root_caches: Default::default(),
            storage_trie_caches: Default::default(),
            committed_zktrie_root,
            zktrie_db,
            zktrie,
        })
    }

    /// Get the previous storage root of an account.
    #[inline]
    pub(crate) fn prev_storage_root(&self, address: &Address) -> B256 {
        self.storage_root_caches
            .borrow()
            .get(address)
            .copied()
            .unwrap_or_default()
    }

    #[inline]
    pub(crate) fn update_storage_root_cache(&self, address: Address, storage_root: ZkTrie<H>) {
        let new_root = *storage_root.root().unwrap_ref();
        let old = self
            .storage_root_caches
            .borrow_mut()
            .insert(address, new_root);

        let mut storage_trie_caches = self.storage_trie_caches.borrow_mut();
        if let Some(old) = old {
            storage_trie_caches.remove(&old);
        }

        storage_trie_caches.insert(new_root, Some(storage_root));
    }

    /// Get the committed zkTrie root.
    #[inline]
    pub(crate) fn committed_zktrie_root(&self) -> B256 {
        self.committed_zktrie_root
    }

    /// Set the committed zkTrie root.
    #[inline]
    pub(crate) fn updated_committed_zktrie_root(&mut self, new_root: B256) {
        self.committed_zktrie_root = new_root;
    }

    /// Update the database with a new block trace.
    pub fn insert_codes<T: Block>(&mut self, l2_trace: T) -> Result<()> {
        measure_duration_millis!(
            update_db_duration_milliseconds,
            self.insert_codes_inner(l2_trace)
        )
    }

    fn insert_codes_inner<T: Block>(&mut self, l2_trace: T) -> Result<()> {
        cycle_tracker_start!("insert CodeDB");
        for code in l2_trace.codes() {
            let hash = revm::primitives::keccak256(code);
            self.code_db
                .or_put(hash.as_slice(), code)
                .map_err(DatabaseError::code_db)?;
        }
        cycle_tracker_end!("insert CodeDB");
        Ok(())
    }

    fn load_code(&self, hash: B256) -> Result<Option<Bytecode>> {
        let mut code_cache = self.analyzed_code_cache.borrow_mut();
        if let Some(code) = code_cache.get(&hash) {
            Ok(code.clone())
        } else {
            let code = self
                .code_db
                .get(&hash)
                .map_err(DatabaseError::code_db)?
                .map(|v| to_analysed(Bytecode::new_legacy(v.into_bytes().into())));
            code_cache.insert(hash, code.clone());
            Ok(code)
        }
    }
}

impl<CodeDb: KVDatabase, ZkDb: KVDatabase + 'static, H: HashScheme> DatabaseRef
    for EvmDatabase<'_, CodeDb, ZkDb, H>
{
    type Error = DatabaseError;

    /// Get basic account information.
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>> {
        let Some(account) = measure_duration_micros!(
            zktrie_get_duration_microseconds,
            self.zktrie.get::<_, Account, _>(self.zktrie_db, address)
        )
        .map_err(DatabaseError::zk_trie)?
        else {
            return Ok(None);
        };

        self.storage_root_caches
            .borrow_mut()
            .insert(address, account.storage_root);

        let mut info = AccountInfo::from(account);
        info.code = self.load_code(account.code_hash)?;

        if let Some(ref code) = info.code {
            debug_assert_eq!(
                info.code_hash,
                code.hash_slow(),
                "code hash mismatch for account {address:?}",
            );
            assert_eq!(
                info.code_size,
                code.original_bytes().len(),
                "code size mismatch for account {address:?}",
            );
        }

        Ok(Some(info))
    }

    /// Get account code by its code hash.
    fn code_by_hash_ref(&self, hash: B256) -> Result<Bytecode> {
        // Sometimes the code in previous account info is not contained,
        // and the CacheDB has already loaded the previous account info,
        // then the upcoming trace contains code (meaning the code is used in this new block),
        // we can't directly update the CacheDB, so we offer the code by hash here.
        // However, if the code still cannot be found, this is an error.
        self.load_code(hash)?.ok_or_else(|| {
            unreachable!(
                "Code is either loaded or not needed (like EXTCODESIZE), code hash: {:?}",
                hash
            );
        })
    }

    /// Get storage value of address at index.
    fn storage_ref(&self, address: Address, index: U256) -> Result<U256> {
        dev_trace!("get storage of {:?} at index {:?}", address, index);
        let storage_root = *self
            .storage_root_caches
            .borrow_mut()
            .entry(address)
            .or_insert_with_key(|address| {
                self.zktrie
                    .get::<_, Account, _>(self.zktrie_db, address)
                    .expect("unexpected zktrie error")
                    .map(|acc| acc.storage_root)
                    .unwrap_or_default()
            });

        let mut storage_trie_caches = self.storage_trie_caches.borrow_mut();

        let trie = storage_trie_caches
            .entry(storage_root)
            .or_insert_with_key(|storage_root| {
                dev_debug!("storage root of {:?} is {:?}", address, storage_root);

                ZkTrie::new_with_root(self.zktrie_db, NoCacheHasher, *storage_root)
                    .inspect_err(|_e| {
                        dev_warn!(
                            "storage trie associated with account({address}) not found: {_e}\n{}",
                            std::backtrace::Backtrace::force_capture()
                        );
                    })
                    .ok()
            });
        if trie.is_none() {
            return Err(DatabaseError::NotIncluded);
        }
        let trie = trie.as_mut().unwrap();

        #[cfg(debug_assertions)]
        {
            let current_root = trie.root().unwrap_ref();
            let expected_root = self
                .zktrie
                .get::<_, Account, _>(self.zktrie_db, address)
                .expect("unexpected zktrie error")
                .map(|acc| acc.storage_root)
                .unwrap_or_default();
            assert_eq!(*current_root, expected_root);
        }

        Ok(measure_duration_micros!(
            zktrie_get_duration_microseconds,
            trie.get::<_, U256, _>(self.zktrie_db, index.to_be_bytes::<32>())
        )
        .map_err(DatabaseError::zk_trie)?
        .unwrap_or_default())
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, _: u64) -> Result<B256> {
        unreachable!("BLOCKHASH is disabled")
    }
}
