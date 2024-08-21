use eth_types::{
    state_db::{CodeDB, StateDB},
    ToWord, H160, H256,
};
use mpt_zktrie::ZktrieState;
use revm::{
    db::DatabaseRef,
    primitives::{AccountInfo, Address, Bytecode, B256, U256},
    Database,
};
use std::{convert::Infallible, fmt::Debug};

use crate::{cycle_tracker_end, cycle_tracker_start, dev_trace, utils::ext::BlockRevmDbExt};

/// A read-only in-memory database that consists of account and storage information.
#[derive(Debug)]
pub struct ReadOnlyDB {
    /// In-memory map of code hash to bytecode. The code hash is a poseidon hash of the bytecode if
    /// the "scroll" feature is enabled, otherwise by default it is the keccak256 hash.
    code_db: CodeDB,
    /// In-memory key-value database representing the state trie.
    pub(crate) sdb: StateDB,
}

impl Default for ReadOnlyDB {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadOnlyDB {
    /// Initialize an EVM database from a block trace.
    pub fn new() -> Self {
        ReadOnlyDB {
            code_db: CodeDB::new(),
            sdb: StateDB::new(),
        }
    }

    /// Update the database with a new block trace.
    pub fn update<T: BlockRevmDbExt>(&mut self, l2_trace: T, zktrie_state: &ZktrieState) {
        dev_trace!(
            "update ReadOnlyDB with trie root: {:?}",
            l2_trace.root_before()
        );

        cycle_tracker_start!("insert StateDB account");
        for (addr, account) in l2_trace.accounts(zktrie_state) {
            dev_trace!("insert account {:?} {:?}", addr, account);
            let (exist, _) = self.sdb.get_account(&addr);
            // won't update exist value, those should be already updated in upper CacheDB
            if !exist {
                self.sdb.set_account(&addr, account);
            }
        }
        cycle_tracker_end!("insert StateDB account");

        cycle_tracker_start!("insert StateDB storage");
        for ((addr, key), val) in l2_trace.storages(zktrie_state) {
            dev_trace!("insert storage {:?} {:?} {:?}", addr, key, val);
            let key = key.to_word();
            let (exist, _) = self.sdb.get_committed_storage(&addr, &key);
            // won't update exist value, those should be already updated in upper CacheDB
            if !exist {
                *self.sdb.get_storage_mut(&addr, &key).1 = val;
            }
        }
        cycle_tracker_end!("insert StateDB storage");

        cycle_tracker_start!("insert CodeDB");
        for code in l2_trace.codes() {
            let hash = revm::primitives::keccak256(code);
            dev_trace!("insert code {:?}", hash);
            // save a `to_vec` call if exists
            if self.code_db.0.contains_key(&H256(hash.0)) {
                continue;
            }
            self.code_db.insert_with_hash(H256(hash.0), code.to_vec());
        }
        cycle_tracker_end!("insert CodeDB");
    }
}

impl DatabaseRef for ReadOnlyDB {
    type Error = Infallible;

    /// Get basic account information.
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let (exist, acc) = self.sdb.get_account(&H160::from(**address));

        dev_trace!("loaded account: {address:?}, exist: {exist}, acc: {acc:?}");
        if exist {
            let acc = AccountInfo {
                balance: U256::from_limbs(acc.balance.0),
                nonce: acc.nonce.as_u64(),
                code_size: acc.code_size.as_usize(),
                // revm code hash is keccak256 of bytecode
                code_hash: B256::from(acc.keccak_code_hash.to_fixed_bytes()),
                // we also need poseidon code hash which is [eth_types::Account::code_hash]
                poseidon_code_hash: B256::from(acc.code_hash.to_fixed_bytes()),
                // if None, means CodeDB did not include the code, could cause by: EXTCODESIZE, EXTCODEHASH
                code: self
                    .code_db
                    .0
                    .get(&acc.keccak_code_hash)
                    .map(|vec| Bytecode::new_raw(revm::primitives::Bytes::from(vec.clone()))),
            };
            Ok(Some(acc))
        } else {
            Ok(None)
        }
    }

    /// Get account code by its code hash.
    fn code_by_hash_ref(&self, hash: B256) -> Result<Bytecode, Self::Error> {
        // Sometimes the code in previous account info is not contained,
        // and the CacheDB has already loaded the previous account info,
        // then the upcoming trace contains code (meaning the code is used in this new block),
        // we can't directly update the CacheDB, so we offer the code by hash here.
        // However, if the code still cannot be found, this is an error.
        self.code_db
            .0
            .get(&H256(hash.0))
            .map(|vec| Bytecode::new_raw(revm::primitives::Bytes::from(vec.clone())))
            .ok_or_else(|| {
                unreachable!(
                    "Code is either loaded or not needed (like EXTCODESIZE), code hash: {:?}",
                    hash
                );
            })
    }

    /// Get storage value of address at index.
    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let (_, val) = self
            .sdb
            .get_storage(&H160::from(**address), &eth_types::U256(*index.as_limbs()));
        Ok(U256::from_limbs(val.0))
    }

    /// Get block hash by block number.
    fn block_hash_ref(&self, _: u64) -> Result<B256, Self::Error> {
        unreachable!("BLOCKHASH is disabled")
    }
}

impl Database for ReadOnlyDB {
    type Error = Infallible;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        DatabaseRef::basic_ref(self, address)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        DatabaseRef::code_by_hash_ref(self, code_hash)
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        DatabaseRef::storage_ref(self, address, index)
    }

    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> {
        DatabaseRef::block_hash_ref(self, block_number)
    }
}
