use core::error;
use eth_types::{geth_types::TxType, H160, H256, U256};
use mpt_zktrie::{AccountData, ZktrieState};
use revm::inspectors::CustomPrintTracer;
use revm::precompile::B256;
use revm::{
    db::CacheDB,
    inspector_handle_register,
    primitives::{AccountInfo, Env, SpecId},
    DatabaseRef,
};
use std::convert::Infallible;
use std::fmt::Debug;
use std::rc::Rc;
use zktrie::{UpdateDb, ZkMemoryDb, ZkTrie};

use crate::{
    cycle_tracker_end, cycle_tracker_start,
    database::ReadOnlyDB,
    dev_debug, dev_trace,
    error::VerificationError,
    utils::ext::{BlockTraceRevmExt, TxRevmExt},
    HardforkConfig,
};

mod builder;
use crate::utils::ext::BlockRevmDbExt;
pub use builder::EvmExecutorBuilder;

/// Execute hooks
pub mod hooks;

/// EVM executor that handles the block.
pub struct EvmExecutor {
    hardfork_config: HardforkConfig,
    db: CacheDB<ReadOnlyDB>,
    zktrie_db: Rc<ZkMemoryDb>,
    zktrie: ZkTrie<UpdateDb>,
    spec_id: SpecId,
    hooks: hooks::ExecuteHooks,
}

impl EvmExecutor {
    /// Get reference to the DB
    pub fn db(&self) -> &CacheDB<ReadOnlyDB> {
        &self.db
    }

    /// Update the DB
    pub fn update_db<T: BlockRevmDbExt>(&mut self, l2_trace: &T, zktrie_state: &ZktrieState) {
        self.db.db.update(l2_trace, zktrie_state)
    }

    /// Handle a block.
    pub fn handle_block<T: BlockTraceRevmExt>(
        &mut self,
        l2_trace: &T,
    ) -> Result<(), VerificationError> {
        self.hardfork_config
            .migrate(l2_trace.number(), &mut self.db)
            .unwrap();

        dev_debug!("handle block {:?}", l2_trace.number());
        let mut env = Box::<Env>::default();
        env.cfg.chain_id = l2_trace.chain_id();
        cycle_tracker_start!("create BlockEnv");
        env.block = l2_trace.env();
        cycle_tracker_end!("create BlockEnv");

        for (idx, tx) in l2_trace.transactions().enumerate() {
            cycle_tracker_start!("handle tx {}", idx);

            dev_trace!("handle {idx}th tx");

            dev_trace!("{tx:#?}");
            let mut env = env.clone();
            env.tx = tx.tx_env();
            if tx.raw_type() == 0 {
                env.tx.chain_id = Some(l2_trace.chain_id());
            }
            let eth_tx = tx.to_eth_tx(
                l2_trace.block_hash(),
                l2_trace.number(),
                idx,
                l2_trace.base_fee_per_gas(),
            );

            let recovered_address =
                eth_tx
                    .recover_from()
                    .map_err(|error| VerificationError::SignerRecovery {
                        tx_hash: eth_tx.hash,
                        source: error,
                    })?;

            // verify that the transaction is valid
            if recovered_address != eth_tx.from {
                return Err(VerificationError::SenderSignerMismatch {
                    tx_hash: eth_tx.hash,
                    sender: eth_tx.from,
                    signer: recovered_address,
                });
            }

            let tx_type = TxType::get_tx_type(&eth_tx);
            if tx_type.is_l1_msg() {
                env.tx.nonce = None; // clear nonce for l1 msg
                env.cfg.disable_base_fee = true; // disable base fee for l1 msg
            }
            env.tx.scroll.is_l1_msg = tx_type.is_l1_msg();
            let rlp_bytes = eth_tx.rlp().to_vec();
            self.hooks.tx_rlp(self, &rlp_bytes);
            env.tx.scroll.rlp_bytes = Some(revm::primitives::Bytes::from(rlp_bytes));

            dev_trace!("{env:#?}");
            {
                cycle_tracker_start!("build Evm");
                let mut revm = revm::Evm::builder()
                    .with_spec_id(self.spec_id)
                    .with_db(&mut self.db)
                    .with_spec_id(self.spec_id)
                    .with_env(env)
                    // .with_external_context(CustomPrintTracer::default())
                    // .append_handler_register(inspector_handle_register)
                    .build();
                cycle_tracker_end!("build Evm");

                dev_trace!("handler cfg: {:?}", revm.handler.cfg);

                cycle_tracker_start!("transact_commit");
                let result =
                    revm.transact_commit()
                        .map_err(|e| VerificationError::EvmExecution {
                            tx_hash: eth_tx.hash,
                            source: e,
                        })?;
                cycle_tracker_end!("transact_commit");

                dev_trace!("{result:#?}");
            }
            self.hooks.post_tx_execution(self, idx);

            dev_debug!("handle {idx}th tx done");
            cycle_tracker_end!("handle tx {}", idx);
        }
        Ok(())
    }

    /// Commit pending changes in cache db to zktrie
    pub fn commit_changes(&mut self) -> H256 {
        cycle_tracker_start!("commit_changes");
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

            dev_trace!("committing {addr}, {:?} {db_acc:?}", db_acc.account_state);
            cycle_tracker_start!("commit account {}", addr);

            cycle_tracker_start!("get acc_data");
            let mut acc_data = self
                .zktrie
                .get_account(addr.as_slice())
                .map(AccountData::from)
                .unwrap_or_default();
            cycle_tracker_end!("get acc_data");

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
                cycle_tracker_start!("update storage_tire");
                let mut storage_tire = self
                    .zktrie_db
                    .new_trie(storage_root_before.as_fixed_bytes())
                    .expect("unable to get storage trie");
                for (key, value) in db_acc.storage.iter() {
                    if !value.is_zero() {
                        cycle_tracker_start!("Zktrie::update_store");
                        storage_tire
                            .update_store(&key.to_be_bytes::<32>(), &value.to_be_bytes())
                            .expect("failed to update storage");
                        cycle_tracker_end!("Zktrie::update_store");

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
                        cycle_tracker_start!("Zktrie::delete");
                        storage_tire.delete(&key.to_be_bytes::<32>());
                        cycle_tracker_end!("Zktrie::delete");

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
                cycle_tracker_end!("update storage_tire");
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
            // When the acc from StateDB is empty and info is not, also the code hash changes,
            // we need to update the code hash and code size
            if (acc.is_empty() && !info.is_empty()) || acc.keccak_code_hash.0 != info.code_hash.0 {
                assert_ne!(
                    info.poseidon_code_hash,
                    B256::ZERO,
                    "revm didn't update poseidon_code_hash, acc from StateDB: {acc:?}, revm: {info:?}",
                );
                acc_data.poseidon_code_hash = H256::from(info.poseidon_code_hash.0);
                acc_data.keccak_code_hash = H256::from(info.code_hash.0);
                acc_data.code_size = self
                    .db
                    .contracts
                    .get(&db_acc.info.code_hash)
                    .map(|c| c.len())
                    .unwrap_or_default() as u64;
            }

            #[cfg(feature = "debug-account")]
            debug_account.insert(*addr, acc_data);

            cycle_tracker_start!("Zktrie::update_account");
            self.zktrie
                .update_account(addr.as_slice(), &acc_data.into())
                .expect("failed to update account");
            cycle_tracker_end!("Zktrie::update_account");

            cycle_tracker_end!("commit account {}", addr);
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
        cycle_tracker_end!("commit_changes");
        H256::from(self.zktrie.root())
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
