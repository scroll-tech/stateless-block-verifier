use crate::{
    database::ReadOnlyDB,
    error::VerificationError,
    error::ZkTrieError,
    utils::ext::{BlockTraceRevmExt, TxRevmExt},
    HardforkConfig,
};
use eth_types::{geth_types::TxType, H256, U256};
use mpt_zktrie::{AccountData, ZktrieState};
use revm::db::AccountState;
use revm::precompile::B256;
use revm::primitives::{KECCAK_EMPTY, POSEIDON_EMPTY};
use revm::{
    db::CacheDB,
    primitives::{AccountInfo, Env, SpecId},
};
use std::fmt::Debug;

mod builder;
use crate::utils::ext::BlockTraceExt;
pub use builder::EvmExecutorBuilder;

/// Execute hooks
pub mod hooks;

/// EVM executor that handles the block.
pub struct EvmExecutor {
    hardfork_config: HardforkConfig,
    db: CacheDB<ReadOnlyDB>,
    spec_id: SpecId,
    hooks: hooks::ExecuteHooks,
}

impl EvmExecutor {
    /// Get reference to the DB
    pub fn db(&self) -> &CacheDB<ReadOnlyDB> {
        &self.db
    }

    /// Update the DB
    pub fn update_db<T: BlockTraceExt>(&mut self, l2_trace: &T) -> Result<(), ZkTrieError> {
        self.db.db.invalidate_storage_root_caches(
            self.db
                .accounts
                .iter()
                .map(|(addr, acc)| (*addr, acc.account_state.clone())),
        );

        self.db.db.update(l2_trace)
    }

    /// Handle a block.
    pub fn handle_block<T: BlockTraceRevmExt>(
        &mut self,
        l2_trace: &T,
    ) -> Result<(), VerificationError> {
        measure_duration_histogram!(
            handle_block_duration_microseconds,
            self.handle_block_inner(l2_trace)
        )?;

        #[cfg(feature = "metrics")]
        crate::metrics::REGISTRY.block_counter.inc();

        Ok(())
    }

    #[inline(always)]
    fn handle_block_inner<T: BlockTraceRevmExt>(
        &mut self,
        l2_trace: &T,
    ) -> Result<(), VerificationError> {
        self.hardfork_config
            .migrate(l2_trace.number(), &mut self.db)
            .unwrap();

        dev_debug!("handle block {:?}", l2_trace.number());
        let mut env = Box::<Env>::default();
        env.cfg.chain_id = l2_trace.chain_id();
        env.block = cycle_track!(l2_trace.env(), "create BlockEnv");

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

            let tx_type = TxType::get_tx_type(&eth_tx);

            if !tx_type.is_l1_msg() {
                let recovered_address = cycle_track!(
                    eth_tx
                        .recover_from()
                        .map_err(|source| VerificationError::SignerRecovery {
                            tx_hash: eth_tx.hash,
                            source,
                        })?,
                    "recover address"
                );

                // verify that the transaction is valid
                if recovered_address != eth_tx.from {
                    return Err(VerificationError::SenderSignerMismatch {
                        tx_hash: eth_tx.hash,
                        sender: eth_tx.from,
                        signer: recovered_address,
                    });
                }
            }
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
                let mut revm = cycle_track!(
                    revm::Evm::builder()
                        .with_spec_id(self.spec_id)
                        .with_db(&mut self.db)
                        .with_env(env)
                        // .with_external_context(CustomPrintTracer::default())
                        // .append_handler_register(inspector_handle_register)
                        .build(),
                    "build Evm"
                );

                dev_trace!("handler cfg: {:?}", revm.handler.cfg);

                let _result =
                    cycle_track!(revm.transact_commit(), "transact_commit").map_err(|e| {
                        VerificationError::EvmExecution {
                            tx_hash: eth_tx.hash,
                            source: e,
                        }
                    })?;

                dev_trace!("{_result:#?}");
            }
            self.hooks.post_tx_execution(self, idx);

            dev_debug!("handle {idx}th tx done");
            cycle_tracker_end!("handle tx {}", idx);
        }
        Ok(())
    }

    /// Commit pending changes in cache db to zktrie
    pub fn commit_changes(&mut self, zktrie_state: &mut ZktrieState) -> H256 {
        measure_duration_histogram!(
            commit_changes_duration_microseconds,
            cycle_track!(self.commit_changes_inner(zktrie_state), "commit_changes")
        )
    }

    fn commit_changes_inner(&mut self, zktrie_state: &mut ZktrieState) -> H256 {
        let mut zktrie = zktrie_state
            .zk_db
            .new_trie(&zktrie_state.trie_root)
            .expect("infallible");

        #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
        let mut debug_recorder = crate::utils::debug::DebugRecorder::new();

        for (addr, db_acc) in self.db.accounts.iter() {
            // If EVM didn't touch the account, we don't need to update it
            if db_acc.account_state == AccountState::None {
                continue;
            }
            let Some(info): Option<AccountInfo> = db_acc.info() else {
                continue;
            };
            if info.is_empty() {
                continue;
            }

            dev_trace!("committing {addr}, {:?} {db_acc:?}", db_acc.account_state);
            cycle_tracker_start!("commit account {}", addr);

            let mut acc_data = AccountData {
                nonce: info.nonce,
                balance: U256(*info.balance.as_limbs()),
                storage_root: self.db.db.prev_storage_root(addr).0.into(),
                ..Default::default()
            };

            if !db_acc.storage.is_empty() {
                // get current storage root
                let storage_root_before = acc_data.storage_root;
                // get storage tire
                cycle_tracker_start!("update storage_tire");
                let mut storage_trie = zktrie_state
                    .zk_db
                    .new_trie(storage_root_before.as_fixed_bytes())
                    .expect("unable to get storage trie");
                for (key, value) in db_acc.storage.iter() {
                    if !value.is_zero() {
                        cycle_track!(
                            storage_trie
                                .update_store(&key.to_be_bytes::<32>(), &value.to_be_bytes())
                                .expect("failed to update storage"),
                            "Zktrie::update_store"
                        );
                    } else {
                        cycle_track!(
                            storage_trie.delete(&key.to_be_bytes::<32>()),
                            "Zktrie::delete"
                        );
                    }

                    #[cfg(feature = "debug-storage")]
                    debug_recorder.record_storage(*addr, *key, *value);
                }

                if storage_trie.is_trie_dirty() {
                    storage_trie.prepare_root();
                }

                cycle_tracker_end!("update storage_tire");
                acc_data.storage_root = H256::from(storage_trie.root());

                #[cfg(feature = "debug-storage")]
                debug_recorder.record_storage_root(*addr, acc_data.storage_root);

                self.db
                    .db
                    .set_prev_storage_root(*addr, acc_data.storage_root.0.into());
            }
            if !info.is_empty() {
                // if account not exist, all fields will be zero.
                // but if account exist, code_hash will be empty hash if code is empty
                if info.is_empty_code_hash() {
                    acc_data.poseidon_code_hash = H256::from(POSEIDON_EMPTY.0);
                    acc_data.keccak_code_hash = H256::from(KECCAK_EMPTY.0);
                } else {
                    assert_ne!(
                        info.poseidon_code_hash,
                        B256::ZERO,
                        "revm didn't update poseidon_code_hash, revm: {info:?}",
                    );
                    acc_data.poseidon_code_hash = H256::from(info.poseidon_code_hash.0);
                    acc_data.keccak_code_hash = H256::from(info.code_hash.0);
                    acc_data.code_size = info.code_size as u64;
                }
            }

            #[cfg(feature = "debug-account")]
            debug_recorder.record_account(*addr, acc_data);

            cycle_track!(
                zktrie
                    .update_account(addr.as_slice(), &acc_data.into())
                    .expect("failed to update account"),
                "Zktrie::update_account"
            );

            cycle_tracker_end!("commit account {}", addr);
        }

        if zktrie.is_trie_dirty() {
            zktrie.prepare_root();
        }

        let root_after = zktrie.root();

        zktrie_state.switch_to(root_after);

        H256::from(root_after)
    }
}

impl Debug for EvmExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvmExecutor")
            .field("db", &self.db)
            .field("spec_id", &self.spec_id)
            .finish()
    }
}
