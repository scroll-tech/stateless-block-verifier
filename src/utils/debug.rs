use eth_types::H256;
use mpt_zktrie::AccountData;
use revm::primitives::{Address, U256};
use std::collections::BTreeMap;

#[derive(serde::Serialize)]
struct StorageOps {
    kind: &'static str,
    key: U256,
    value: Option<U256>,
}

#[derive(Default)]
pub struct DebugRecorder {
    accounts: BTreeMap<Address, AccountData>,
    storages_roots: BTreeMap<Address, H256>,
    storages: BTreeMap<Address, BTreeMap<U256, StorageOps>>,
}

impl DebugRecorder {
    pub fn new() -> Self {
        #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
        std::fs::create_dir_all("/tmp/sbv-debug").expect("failed to create debug dir");

        Self::default()
    }

    #[cfg(feature = "debug-account")]
    pub fn record_account(&mut self, addr: Address, data: AccountData) {
        self.accounts.insert(addr, data);
    }

    #[cfg(feature = "debug-storage")]
    pub fn record_storage_root(&mut self, addr: Address, storage_root: H256) {
        self.storages_roots.insert(addr, storage_root);
    }

    #[cfg(feature = "debug-storage")]
    pub fn record_storage(&mut self, addr: Address, key: U256, value: U256) {
        let entry = self.storages.entry(addr).or_default();
        if !value.is_zero() {
            entry.insert(
                key,
                StorageOps {
                    kind: "update",
                    key,
                    value: Some(value),
                },
            );
        } else {
            entry.insert(
                key,
                StorageOps {
                    kind: "delete",
                    key,
                    value: None,
                },
            );
        }
    }
}

impl Drop for DebugRecorder {
    fn drop(&mut self) {
        #[cfg(feature = "debug-account")]
        {
            let output = std::fs::File::create("/tmp/sbv-debug/accounts.csv")
                .expect("failed to create debug file");
            let mut wtr = csv::Writer::from_writer(output);

            #[derive(serde::Serialize)]
            pub struct AccountData {
                addr: Address,
                nonce: u64,
                balance: U256,
                keccak_code_hash: H256,
                poseidon_code_hash: H256,
                code_size: u64,
                storage_root: H256,
            }

            for (addr, acc) in self.accounts.iter() {
                wtr.serialize(AccountData {
                    addr: *addr,
                    nonce: acc.nonce,
                    balance: U256::from_limbs(acc.balance.0),
                    keccak_code_hash: acc.keccak_code_hash,
                    poseidon_code_hash: acc.poseidon_code_hash,
                    code_size: acc.code_size,
                    storage_root: acc.storage_root,
                })
                .expect("failed to write record");
            }
        }

        #[cfg(feature = "debug-storage")]
        {
            for (addr, storages) in self.storages.iter() {
                let storage_root = self.storages_roots.get(addr).copied().unwrap_or_default();
                let output = std::fs::File::create(format!(
                    "/tmp/sbv-debug/storage_{:?}_{:?}.csv",
                    addr, storage_root
                ))
                .expect("failed to create debug file");
                let mut wtr = csv::Writer::from_writer(output);
                for ops in storages.values() {
                    wtr.serialize(ops).expect("failed to write record");
                }
            }
        }
    }
}