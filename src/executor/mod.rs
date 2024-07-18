use crate::database::ReadOnlyDB;
use eth_types::{geth_types::TxType, l2_types::BlockTraceV2, H160, H256, U256};
use mpt_zktrie::AccountData;
use revm::{
    db::CacheDB,
    primitives::{AccountInfo, BlockEnv, Env, SpecId, TxEnv},
};
use std::fmt::Debug;
use zktrie::ZkTrie;

mod builder;
/// Execute hooks
pub mod hooks;
pub use builder::EvmExecutorBuilder;

/// EVM executor that handles the block.
pub struct EvmExecutor {
    db: CacheDB<ReadOnlyDB>,
    zktrie: ZkTrie,
    spec_id: SpecId,
    hooks: hooks::ExecuteHooks,
}
impl EvmExecutor {
    /// Get reference to the DB
    pub fn db(&self) -> &CacheDB<ReadOnlyDB> {
        &self.db
    }

    /// Handle a block.
    pub fn handle_block(&mut self, l2_trace: &BlockTraceV2) -> H256 {
        debug!("handle block {:?}", l2_trace.header.number.unwrap());
        let mut env = Box::<Env>::default();
        env.cfg.chain_id = l2_trace.chain_id;
        env.block = BlockEnv::from(l2_trace);

        for (idx, tx) in l2_trace.transactions.iter().enumerate() {
            trace!("handle {idx}th tx");
            trace!("{tx:#?}");
            let mut env = env.clone();
            env.tx = TxEnv::from(tx);
            if tx.type_ == 0 {
                env.tx.chain_id = Some(l2_trace.chain_id);
            }
            let eth_tx = tx.to_eth_tx(
                l2_trace.header.hash,
                l2_trace.header.number,
                Some(idx.into()),
                l2_trace.header.base_fee_per_gas,
            );
            let tx_type = TxType::get_tx_type(&eth_tx);
            if tx_type.is_l1_msg() {
                env.tx.nonce = None; // clear nonce for l1 msg
                env.cfg.disable_base_fee = true; // disable base fee for l1 msg
            }
            env.tx.scroll.is_l1_msg = tx_type.is_l1_msg();
            env.tx.scroll.rlp_bytes = Some(revm::primitives::Bytes::from(eth_tx.rlp().to_vec()));
            trace!("{env:#?}");
            {
                let mut revm = revm::Evm::builder()
                    .with_spec_id(self.spec_id)
                    .with_db(&mut self.db)
                    .with_spec_id(self.spec_id)
                    .with_env(env)
                    .build();
                trace!("handler cfg: {:?}", revm.handler.cfg);
                let result = revm.transact_commit().unwrap(); // TODO: handle error
                trace!("{result:#?}");
            }
            self.hooks.post_tx_execution(self, idx);
            debug!("handle {idx}th tx done");
        }
        self.commit_changes();
        H256::from(self.zktrie.root())
    }

    fn commit_changes(&mut self) {
        // let changes = self.db.accounts;
        let sdb = &self.db.db.sdb;

        #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
        std::fs::create_dir_all("/tmp/sbv-debug").expect("failed to create debug dir");

        #[cfg(feature = "debug-account")]
        let mut debug_account = std::collections::BTreeMap::new();

        for (addr, db_acc) in self.db.accounts.iter() {
            let Some(info): Option<AccountInfo> = db_acc.info() else {
                continue;
            };
            let (_, acc) = sdb.get_account(&H160::from(*addr.0));
            if acc.is_empty() && info.is_empty() {
                continue;
            }
            trace!("committing {addr}, {:?} {db_acc:?}", db_acc.account_state);
            let mut acc_data = self
                .zktrie
                .get_account(addr.as_slice())
                .map(AccountData::from)
                .unwrap_or_default();
            acc_data.nonce = info.nonce;
            acc_data.balance = U256(*info.balance.as_limbs());
            if !db_acc.storage.is_empty() {
                #[cfg(feature = "debug-storage")]
                let mut debug_storage = std::collections::BTreeMap::new();

                #[cfg(feature = "debug-storage")]
                #[derive(serde::Serialize)]
                struct StorageOps {
                    kind: &'static str,
                    key: revm::primitives::U256,
                    value: Option<revm::primitives::U256>,
                }

                // get current storage root
                let storage_root_before = acc_data.storage_root;
                // get storage tire
                let mut storage_tire = self
                    .zktrie
                    .get_db()
                    .new_trie(storage_root_before.as_fixed_bytes())
                    .expect("unable to get storage trie");
                for (key, value) in db_acc.storage.iter() {
                    if !value.is_zero() {
                        storage_tire
                            .update_store(&key.to_be_bytes::<32>(), &value.to_be_bytes())
                            .expect("failed to update storage");

                        #[cfg(feature = "debug-storage")]
                        debug_storage.insert(
                            *key,
                            StorageOps {
                                kind: "update",
                                key: *key,
                                value: Some(*value),
                            },
                        );
                    } else {
                        storage_tire.delete(&key.to_be_bytes::<32>());

                        #[cfg(feature = "debug-storage")]
                        debug_storage.insert(
                            *key,
                            StorageOps {
                                kind: "delete",
                                key: *key,
                                value: None,
                            },
                        );
                    }
                }
                acc_data.storage_root = H256::from(storage_tire.root());

                #[cfg(feature = "debug-storage")]
                {
                    let output = std::fs::File::create(format!(
                        "/tmp/sbv-debug/storage_{:?}_{:?}.csv",
                        addr, acc_data.storage_root
                    ))
                    .expect("failed to create debug file");
                    let mut wtr = csv::Writer::from_writer(output);
                    for ops in debug_storage.into_values() {
                        wtr.serialize(ops).expect("failed to write record");
                    }
                }
            }
            if (acc.is_empty() && !info.is_empty()) || acc.code_hash.0 != info.code_hash.0 {
                acc_data.poseidon_code_hash = H256::from(info.code_hash.0);
                acc_data.keccak_code_hash = H256::from(info.keccak_code_hash.0);
                acc_data.code_size = self
                    .db
                    .contracts
                    .get(&db_acc.info.code_hash)
                    .map(|c| c.len())
                    .unwrap_or_default() as u64;
            }

            #[cfg(feature = "debug-account")]
            debug_account.insert(*addr, acc_data.clone());

            self.zktrie
                .update_account(addr.as_slice(), &acc_data.into())
                .expect("failed to update account");
        }

        #[cfg(feature = "debug-account")]
        {
            let output = std::fs::File::create(format!(
                "/tmp/sbv-debug/account_0x{}.csv",
                hex::encode(self.zktrie.root())
            ))
            .expect("failed to create debug file");
            let mut wtr = csv::Writer::from_writer(output);

            #[derive(serde::Serialize)]
            pub struct AccountData {
                addr: revm::primitives::Address,
                nonce: u64,
                balance: U256,
                keccak_code_hash: H256,
                poseidon_code_hash: H256,
                code_size: u64,
                storage_root: H256,
            }

            for (addr, acc) in debug_account.into_iter() {
                wtr.serialize(AccountData {
                    addr,
                    nonce: acc.nonce,
                    balance: acc.balance,
                    keccak_code_hash: acc.keccak_code_hash,
                    poseidon_code_hash: acc.poseidon_code_hash,
                    code_size: acc.code_size,
                    storage_root: acc.storage_root,
                })
                .expect("failed to write record");
            }
        }
    }
}

impl Debug for EvmExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvmExecutor")
            .field("db", &self.db)
            .field("zktrie", &self.zktrie.root())
            .finish()
    }
}
