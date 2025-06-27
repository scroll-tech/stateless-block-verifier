use revm::primitives::{AccountInfo, Address, B256, U256, hex};
use std::{collections::BTreeMap, io::Write, path::PathBuf};

#[derive(Debug, serde::Serialize)]
struct StorageOps {
    kind: &'static str,
    key: U256,
    value: Option<U256>,
}

#[derive(Debug, serde::Serialize)]
struct AccountData {
    addr: Address,
    nonce: u64,
    balance: U256,
    code_hash: B256,
    code_size: u64,
    storage_root: B256,
}

/// Debug recorder for recording account and storage data.
#[derive(Debug)]
pub struct DebugRecorder {
    base_dir: PathBuf,
    accounts: BTreeMap<Address, AccountData>,
    storages_roots: BTreeMap<Address, B256>,
    storages: BTreeMap<Address, BTreeMap<U256, StorageOps>>,
    codes: BTreeMap<B256, Vec<u8>>,
}

impl DebugRecorder {
    /// Create a new debug recorder.
    pub fn new(prefix: &str, prev_root: B256) -> Self {
        let base_dir = PathBuf::from(format!("/tmp/sbv-debug/{prefix}/{prev_root:?}"));

        #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
        std::fs::create_dir_all(&base_dir).expect("failed to create debug dir");

        Self {
            base_dir,
            accounts: BTreeMap::new(),
            storages_roots: BTreeMap::new(),
            storages: BTreeMap::new(),
            codes: BTreeMap::new(),
        }
    }

    /// Record the account data.
    #[cfg(feature = "debug-account")]
    #[allow(clippy::too_many_arguments)]
    pub fn record_account(&mut self, addr: Address, info: AccountInfo, storage_root: B256) {
        self.accounts.insert(
            addr,
            AccountData {
                addr,
                nonce: info.nonce,
                balance: info.balance,
                code_hash: info.code_hash,
                code_size: info.code_size as u64,
                storage_root,
            },
        );
    }

    /// Record the storage root of an account.
    #[cfg(feature = "debug-storage")]
    pub fn record_storage_root(&mut self, addr: Address, storage_root: B256) {
        self.storages_roots.insert(addr, storage_root);
    }

    /// Record the storage operation.
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

    /// Record the code
    #[cfg(feature = "debug-account")]
    pub fn record_code(&mut self, code_hash: B256, code: &[u8]) {
        self.codes.insert(code_hash, code.to_owned());
    }
}

impl Drop for DebugRecorder {
    fn drop(&mut self) {
        #[cfg(feature = "debug-account")]
        {
            let output = std::fs::File::create(self.base_dir.join("accounts.csv"))
                .expect("failed to create debug file");
            let mut wtr = csv::Writer::from_writer(output);

            for (_, acc) in self.accounts.iter() {
                wtr.serialize(acc).expect("failed to write record");
            }

            for (code_hash, code) in self.codes.iter() {
                let mut output =
                    std::fs::File::create(self.base_dir.join(format!("code_{code_hash:?}.txt")))
                        .expect("failed to create debug file");
                let code = hex::encode(code);
                output
                    .write_all(code.as_bytes())
                    .expect("failed to write code");
            }
        }

        #[cfg(feature = "debug-storage")]
        {
            for (addr, storages) in self.storages.iter() {
                let storage_root = self.storages_roots.get(addr).copied().unwrap_or_default();
                let output = std::fs::File::create(
                    self.base_dir
                        .join(format!("storage_{addr:?}_{storage_root:?}.csv")),
                )
                .expect("failed to create debug file");
                let mut wtr = csv::Writer::from_writer(output);
                for ops in storages.values() {
                    wtr.serialize(ops).expect("failed to write record");
                }
            }
        }
    }
}
