use sbv_core::DatabaseRef;
use sbv_kv::nohash::NoHashMap;
use sbv_primitives::{
    Address, B256, BlockHash, BlockNumber, U256,
    types::{AccountInfo, Bytecode},
};
use std::convert::Infallible;
use t8n_types::TransitionToolInput;

#[derive(Debug)]
pub(crate) struct AllocDb {
    accounts: NoHashMap<Address, AccountInfo>,
    storages: NoHashMap<Address, NoHashMap<U256, U256>>,
    codes: NoHashMap<B256, Bytecode>,
    block_hashes: NoHashMap<BlockNumber, BlockHash>,
}

impl AllocDb {
    pub(crate) fn new(input: &TransitionToolInput) -> Self {
        let mut accounts = NoHashMap::default();
        let mut storages = NoHashMap::default();
        let mut codes = NoHashMap::default();
        let block_hashes = input
            .env
            .block_hashes
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();

        for (addr, acc) in input.alloc.iter() {
            let code = Bytecode::new_raw(acc.code.clone());
            let code_hash = code.hash_slow();

            let acc_info = AccountInfo {
                balance: U256::from(acc.balance),
                nonce: acc.nonce,
                code_hash,
                code: Some(code.clone()),
            };
            accounts.insert(*addr, acc_info);
            codes.insert(code_hash, code);

            let storage = acc.storage.iter().map(|(k, v)| (*k, *v)).collect();
            storages.insert(*addr, storage);
        }

        Self {
            accounts,
            storages,
            codes,
            block_hashes,
        }
    }
}

impl DatabaseRef for AllocDb {
    type Error = Infallible;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(self.accounts.get(&address).cloned())
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(self.codes.get(&code_hash).cloned().unwrap_or_default())
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(self
            .storages
            .get(&address)
            .and_then(|s| s.get(&index))
            .copied()
            .unwrap_or_default())
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        Ok(self
            .block_hashes
            .get(&number)
            .copied()
            .expect("Block hash not found"))
    }
}
