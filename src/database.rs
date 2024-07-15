use crate::utils::{collect_account_proofs, collect_storage_proofs};
use eth_types::{
    l2_types::{trace::collect_codes, BlockTrace},
    state_db::{self, CodeDB, StateDB},
    ToWord, H160,
};
use mpt_zktrie::state::ZktrieState;
use revm::{
    db::DatabaseRef,
    primitives::{AccountInfo, Address, Bytecode, B256, U256},
};
use std::{convert::Infallible, fmt::Debug};

/// EVM database that stores account and storage information.
#[derive(Debug)]
pub struct ReadOnlyDB {
    code_db: CodeDB,
    pub(crate) sdb: StateDB,
}

impl ReadOnlyDB {
    /// Initialize an EVM database from a block trace.
    pub fn new(l2_trace: &BlockTrace) -> Self {
        let mut sdb = StateDB::new();
        for parsed in
            ZktrieState::parse_account_from_proofs(collect_account_proofs(&l2_trace.storage_trace))
        {
            let (addr, acc) = parsed.unwrap();
            trace!("insert account {:?} {:?}", addr, acc);
            sdb.set_account(&addr, state_db::Account::from(&acc));
        }

        for parsed in
            ZktrieState::parse_storage_from_proofs(collect_storage_proofs(&l2_trace.storage_trace))
        {
            let ((addr, key), val) = parsed.unwrap();
            let key = key.to_word();
            *sdb.get_storage_mut(&addr, &key).1 = val.into();
        }

        let mut code_db = CodeDB::new();
        for (hash, code) in collect_codes(l2_trace, Some(&sdb)).unwrap() {
            code_db.insert_with_hash(hash, code);
        }

        ReadOnlyDB { code_db, sdb }
    }
}

impl DatabaseRef for ReadOnlyDB {
    type Error = Infallible;

    /// Get basic account information.
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let (exist, acc) = self.sdb.get_account(&H160::from(**address));
        trace!("loaded account: {address:?}, exist: {exist}, acc: {acc:?}");
        if exist {
            let acc = AccountInfo {
                balance: U256::from_limbs(acc.balance.0),
                nonce: acc.nonce.as_u64(),
                code_size: acc.code_size.as_usize(),
                code_hash: B256::from(acc.code_hash.to_fixed_bytes()),
                keccak_code_hash: B256::from(acc.keccak_code_hash.to_fixed_bytes()),
                // if None, means CodeDB did not include the code, could cause by: EXTCODESIZE
                code: self
                    .code_db
                    .0
                    .get(&acc.code_hash)
                    .map(|vec| Bytecode::new_raw(revm::primitives::Bytes::from(vec.clone()))),
            };
            Ok(Some(acc))
        } else {
            Ok(None)
        }
    }

    /// Get account code by its hash.
    fn code_by_hash_ref(&self, _: B256) -> Result<Bytecode, Self::Error> {
        panic!("Should not be called. Code is either loaded or not needed (like EXTCODESIZE)");
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
        unimplemented!("BLOCKHASH is disabled")
    }
}

impl revm::Database for ReadOnlyDB {
    type Error = Infallible;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        DatabaseRef::basic_ref(self, address)
    }

    fn code_by_hash(&mut self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        panic!("Should not be called. Code is already loaded");
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        DatabaseRef::storage_ref(self, address, index)
    }

    fn block_hash(&mut self, _: u64) -> Result<B256, Self::Error> {
        unimplemented!("BLOCKHASH is disabled")
    }
}
