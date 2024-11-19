use crate::{
    database::EvmDatabase, error::DatabaseError, error::VerificationError, HardforkConfig,
};
use revm::db::AccountState;
use revm::primitives::alloy_primitives::ChainId;
use revm::primitives::{BlockEnv, TxEnv, U256};
use revm::{
    db::CacheDB,
    primitives::{AccountInfo, Env, B256, KECCAK_EMPTY, POSEIDON_EMPTY},
};
use sbv_primitives::{
    alloy_primitives::Bytes,
    zk_trie::{
        db::kv::KVDatabase,
        hash::{key_hasher::NoCacheHasher, HashScheme},
        scroll_types::Account,
        trie::ZkTrie,
    },
    Block, Transaction, TxTrace,
};
use std::fmt::Debug;
use std::mem::ManuallyDrop;

mod builder;
pub use builder::EvmExecutorBuilder;

/// EVM executor that handles the block.
pub struct EvmExecutor<'db, CodeDb, ZkDb, H> {
    chain_id: ChainId,
    hardfork_config: HardforkConfig,
    db: CacheDB<EvmDatabase<'db, CodeDb, ZkDb, H>>,
}

/// Block execution result
#[derive(Debug, Clone)]
pub struct BlockExecutionResult {
    /// Gas used in this block
    pub gas_used: u64,
    /// RLP bytes of transactions
    pub tx_rlps: Vec<Bytes>,
}

impl<CodeDb: KVDatabase, ZkDb: KVDatabase + 'static, H: HashScheme>
    EvmExecutor<'_, CodeDb, ZkDb, H>
{
    /// Get reference to the DB
    pub fn db(&self) -> &CacheDB<EvmDatabase<CodeDb, ZkDb, H>> {
        &self.db
    }

    /// Insert codes from trace into CodeDB
    pub fn insert_codes<T: Block>(&mut self, l2_trace: &T) -> Result<(), DatabaseError> {
        self.db.db.insert_codes(l2_trace)
    }

    /// Handle a block.
    pub fn handle_block<T: Block>(
        &mut self,
        l2_trace: &T,
    ) -> Result<BlockExecutionResult, VerificationError> {
        #[allow(clippy::let_and_return)]
        let gas_used = measure_duration_millis!(
            handle_block_duration_milliseconds,
            cycle_track!(self.handle_block_inner(l2_trace), "handle_block")
        )?;

        #[cfg(feature = "metrics")]
        sbv_utils::metrics::REGISTRY.block_counter.inc();

        Ok(gas_used)
    }

    #[inline(always)]
    fn handle_block_inner<T: Block>(
        &mut self,
        l2_trace: &T,
    ) -> Result<BlockExecutionResult, VerificationError> {
        let spec_id = self.hardfork_config.get_spec_id(l2_trace.number());
        dev_trace!("use spec id {spec_id:?}",);
        self.hardfork_config
            .migrate(l2_trace.number(), &mut self.db)
            .unwrap();

        dev_debug!("handle block {:?}", l2_trace.number());
        let mut env = Box::<Env>::default();
        env.cfg.chain_id = self.chain_id;
        env.block = BlockEnv {
            number: U256::from_limbs([l2_trace.number(), 0, 0, 0]),
            coinbase: l2_trace.coinbase(),
            timestamp: l2_trace.timestamp(),
            gas_limit: l2_trace.gas_limit(),
            basefee: l2_trace.base_fee_per_gas().unwrap_or_default(),
            difficulty: l2_trace.difficulty(),
            prevrandao: l2_trace.prevrandao(),
            blob_excess_gas_and_price: None,
        };

        let mut gas_used = 0;
        let mut tx_rlps = Vec::with_capacity(l2_trace.num_txs());

        for (idx, tx) in l2_trace.transactions().enumerate() {
            cycle_tracker_start!("handle tx");

            dev_trace!("handle {idx}th tx");

            let tx = tx
                .try_build_typed_tx()
                .map_err(|e| VerificationError::InvalidSignature {
                    tx_hash: tx.tx_hash(),
                    source: e,
                })?;
            let tx = ManuallyDrop::new(tx);

            dev_trace!("{tx:#?}");
            let mut env = env.clone();
            env.tx = TxEnv {
                caller: tx.get_or_recover_signer().map_err(|e| {
                    VerificationError::InvalidSignature {
                        tx_hash: *tx.tx_hash(),
                        source: e,
                    }
                })?,
                gas_limit: tx.gas_limit(),
                gas_price: tx
                    .effective_gas_price(l2_trace.base_fee_per_gas().unwrap_or_default().to())
                    .map(U256::from)
                    .ok_or_else(|| VerificationError::InvalidGasPrice {
                        tx_hash: *tx.tx_hash(),
                    })?,
                transact_to: tx.kind(),
                value: tx.value(),
                data: tx.data(),
                nonce: if !tx.is_l1_msg() {
                    Some(tx.nonce())
                } else {
                    None
                },
                chain_id: tx.chain_id(),
                access_list: tx.access_list().cloned().unwrap_or_default().0,
                gas_priority_fee: tx.max_priority_fee_per_gas().map(U256::from),
                ..Default::default()
            };

            if tx.is_l1_msg() {
                env.cfg.disable_base_fee = true; // disable base fee for l1 msg
            }
            env.tx.scroll.is_l1_msg = tx.is_l1_msg();
            let rlp_bytes = tx.rlp();
            tx_rlps.push(rlp_bytes.clone());
            env.tx.scroll.rlp_bytes = Some(rlp_bytes);

            dev_trace!("{env:#?}");
            {
                let revm = cycle_track!(
                    revm::Evm::builder()
                        .with_spec_id(spec_id)
                        .with_db(&mut self.db)
                        .with_env(env)
                        // .with_external_context(CustomPrintTracer::default())
                        // .append_handler_register(inspector_handle_register)
                        .build(),
                    "build Evm"
                );
                let mut revm = ManuallyDrop::new(revm);

                dev_trace!("handler cfg: {:?}", revm.handler.cfg);

                let result = measure_duration_millis!(
                    transact_commit_duration_milliseconds,
                    cycle_track!(revm.transact_commit(), "transact_commit").map_err(|e| {
                        VerificationError::EvmExecution {
                            tx_hash: *tx.tx_hash(),
                            source: e,
                        }
                    })?
                );

                gas_used += result.gas_used();

                dev_trace!("{result:#?}");
            }

            dev_debug!("handle {idx}th tx done");
            cycle_tracker_end!("handle tx");
        }
        Ok(BlockExecutionResult { gas_used, tx_rlps })
    }

    /// Commit pending changes in cache db to zktrie
    pub fn commit_changes(&mut self) -> Result<B256, DatabaseError> {
        measure_duration_millis!(
            commit_changes_duration_milliseconds,
            cycle_track!(self.commit_changes_inner(), "commit_changes")
        )
    }

    fn commit_changes_inner(&mut self) -> Result<B256, DatabaseError> {
        let mut zktrie = ZkTrie::<H>::new_with_root(
            self.db.db.zktrie_db,
            NoCacheHasher,
            self.db.db.committed_zktrie_root(),
        )
        .expect("infallible");

        #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
        let mut debug_recorder = sbv_utils::DebugRecorder::new(
            std::any::type_name_of_val(&self),
            self.db.db.committed_zktrie_root(),
        );

        for (addr, db_acc) in self.db.accounts.iter() {
            // If EVM didn't touch the account, we don't need to update it
            if db_acc.account_state == AccountState::None {
                continue;
            }
            let Some(mut info): Option<AccountInfo> = db_acc.info() else {
                continue;
            };
            if info.is_empty() {
                continue;
            }
            if let Some(ref code) = info.code {
                self.db
                    .db
                    .code_db
                    .or_put(info.code_hash.as_slice(), code.original_byte_slice())
                    .unwrap();
                debug_assert_eq!(
                    info.code_hash,
                    code.hash_slow(),
                    "code hash mismatch for account {addr:?}",
                );
                assert_eq!(
                    info.code_size,
                    code.original_bytes().len(),
                    "code size mismatch for account {addr:?}",
                );
                #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
                debug_recorder.record_code(info.code_hash, code.bytecode().as_ref());
            }

            dev_trace!("committing {addr}, {:?} {db_acc:?}", db_acc.account_state);
            cycle_tracker_start!("commit account");

            let mut storage_root = self.db.db.prev_storage_root(addr);

            if !db_acc.storage.is_empty() {
                // get current storage root
                let storage_root_before = storage_root;
                // get storage tire
                cycle_tracker_start!("update storage_tire");
                let mut storage_trie = cycle_track!(
                    ZkTrie::<H>::new_with_root(
                        self.db.db.zktrie_db,
                        NoCacheHasher,
                        storage_root_before,
                    ),
                    "Zktrie::new_with_root"
                )
                .expect("unable to get storage trie");
                for (key, value) in db_acc.storage.iter() {
                    if !value.is_zero() {
                        measure_duration_micros!(
                            zktrie_update_duration_microseconds,
                            cycle_track!(
                                storage_trie
                                    .update(self.db.db.zktrie_db, key.to_be_bytes::<32>(), value)
                                    .map_err(DatabaseError::zk_trie)?,
                                "Zktrie::update_store"
                            )
                        );
                    } else {
                        measure_duration_micros!(
                            zktrie_delete_duration_microseconds,
                            cycle_track!(
                                storage_trie
                                    .delete(self.db.db.zktrie_db, key.to_be_bytes::<32>())
                                    .map_err(DatabaseError::zk_trie)?,
                                "Zktrie::delete"
                            )
                        );
                    }

                    #[cfg(feature = "debug-storage")]
                    debug_recorder.record_storage(*addr, *key, *value);
                }

                measure_duration_micros!(
                    zktrie_commit_duration_microseconds,
                    cycle_track!(
                        storage_trie
                            .commit(self.db.db.zktrie_db)
                            .map_err(DatabaseError::zk_trie)?,
                        "Zktrie::commit"
                    )
                );

                cycle_tracker_end!("update storage_tire");
                storage_root = *storage_trie.root().unwrap_ref();

                self.db.db.update_storage_root_cache(*addr, storage_trie);

                #[cfg(feature = "debug-storage")]
                debug_recorder.record_storage_root(*addr, storage_root);
            }
            if !info.is_empty() {
                // if account not exist, all fields will be zero.
                // but if account exist, code_hash will be empty hash if code is empty
                if info.is_empty_code_hash() {
                    info.code_hash = KECCAK_EMPTY.0.into();
                    info.poseidon_code_hash = POSEIDON_EMPTY.0.into();
                } else {
                    assert_ne!(
                        info.poseidon_code_hash,
                        B256::ZERO,
                        "revm didn't update poseidon_code_hash, revm: {info:?}",
                    );
                }
            } else {
                info.code_hash = B256::ZERO;
                info.poseidon_code_hash = B256::ZERO;
            }

            #[cfg(feature = "debug-account")]
            debug_recorder.record_account(*addr, info.clone(), storage_root);

            let acc_data = Account::from_revm_account_with_storage_root(info, storage_root);
            dev_trace!("committing account {addr}: {acc_data:?}");
            measure_duration_micros!(
                zktrie_update_duration_microseconds,
                cycle_track!(
                    zktrie
                        .update(self.db.db.zktrie_db, addr, acc_data)
                        .expect("failed to update account"),
                    "Zktrie::update_account"
                )
            );

            cycle_tracker_end!("commit account");
        }

        measure_duration_micros!(
            zktrie_commit_duration_microseconds,
            cycle_track!(
                zktrie
                    .commit(self.db.db.zktrie_db)
                    .map_err(DatabaseError::zk_trie)?,
                "Zktrie::commit"
            )
        );

        let root_after = *zktrie.root().unwrap_ref();

        self.db.db.updated_committed_zktrie_root(root_after);

        self.db.accounts.clear();
        self.db.contracts.clear();
        self.db.block_hashes.clear();
        self.db.logs.clear();

        Ok(B256::from(root_after))
    }
}

impl<CodeDb, ZkDb, H> Debug for EvmExecutor<'_, CodeDb, ZkDb, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvmExecutor").field("db", &self.db).finish()
    }
}
