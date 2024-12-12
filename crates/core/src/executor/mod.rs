use crate::{database::EvmDatabase, error::VerificationError};
use revm::primitives::{Authorization, SignedAuthorization};
use revm::{
    db::{AccountState, CacheDB},
    primitives::{AccountInfo, BlobExcessGasAndPrice, BlockEnv, Bytes, Env, TxEnv, KECCAK_EMPTY},
};
use sbv_chainspec::{revm_spec, ChainSpec, Head};
use sbv_kv::KeyValueStore;
use sbv_primitives::{BlockHeader, BlockWitness, B256, U256};
use sbv_trie::{TrieAccount, TrieNode};
use std::fmt::Debug;

mod builder;
pub use builder::EvmExecutorBuilder;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<CodeDb, NodesProvider, Witness> {
    chain_spec: ChainSpec,
    db: CacheDB<EvmDatabase<CodeDb, NodesProvider>>,
    witness: Witness,
}

/// Block execution result
#[derive(Debug, Clone)]
pub struct BlockExecutionResult {
    /// Gas used in this block
    pub gas_used: u64,
    /// RLP bytes of transactions
    pub tx_rlps: Vec<Bytes>,
}

impl<
        CodeDb: KeyValueStore<B256, Bytes>,
        NodesProvider: KeyValueStore<B256, TrieNode>,
        Witness: BlockWitness,
    > EvmExecutor<CodeDb, NodesProvider, Witness>
{
    /// Get reference to the DB
    pub fn db(&self) -> &CacheDB<EvmDatabase<CodeDb, NodesProvider>> {
        &self.db
    }

    /// Handle the block with the given witness
    pub fn handle_block(&mut self) -> Result<BlockExecutionResult, VerificationError> {
        #[allow(clippy::let_and_return)]
        let gas_used = measure_duration_millis!(
            handle_block_duration_milliseconds,
            cycle_track!(self.handle_block_inner(), "handle_block")
        )?;

        #[cfg(feature = "metrics")]
        sbv_utils::metrics::REGISTRY.block_counter.inc();

        Ok(gas_used)
    }

    #[inline(always)]
    fn handle_block_inner(&mut self) -> Result<BlockExecutionResult, VerificationError> {
        let header = self.witness.header();
        let block_number = header.number();
        dev_debug!("handle block #{block_number}");

        let spec_id = revm_spec(
            &self.chain_spec,
            &Head {
                number: block_number,
                hash: header.hash(),
                timestamp: header.timestamp(),
                difficulty: header.difficulty(),
                ..Default::default()
            },
        );
        dev_trace!("use spec id {spec_id:?}");

        // FIXME: scroll needs migrate on curie block

        let mut env = Box::<Env>::default();
        env.cfg.chain_id = self.chain_spec.chain.id();
        env.block = BlockEnv {
            number: U256::from_limbs([block_number, 0, 0, 0]),
            coinbase: self.chain_spec.genesis.coinbase,
            timestamp: U256::from_limbs([header.timestamp(), 0, 0, 0]),
            gas_limit: U256::from_limbs([header.gas_limit(), 0, 0, 0]),
            basefee: U256::from_limbs([header.base_fee_per_gas().unwrap_or_default(), 0, 0, 0]),
            difficulty: header.difficulty(),
            prevrandao: header.prevrandao(),
            blob_excess_gas_and_price: header.excess_blob_gas().map(BlobExcessGasAndPrice::new),
        };

        let mut gas_used = 0;
        let mut tx_rlps = Vec::with_capacity(self.witness.num_transactions());

        for (idx, tx) in self.witness.build_typed_transactions().enumerate() {
            cycle_tracker_start!("handle tx");

            dev_trace!("handle {idx}th tx");

            let tx = tx.map_err(|e| VerificationError::InvalidSignature { idx, source: e })?;
            let caller = tx
                .get_or_recover_signer()
                .map_err(|e| VerificationError::InvalidSignature { idx, source: e })?;
            let rlp_bytes = tx.rlp();
            tx_rlps.push(rlp_bytes.clone());

            dev_trace!("{tx:#?}");
            let mut env = env.clone();
            env.tx = TxEnv {
                caller,
                gas_limit: tx.gas_limit(),
                gas_price: tx
                    .effective_gas_price(header.base_fee_per_gas().unwrap_or_default())
                    .map(U256::from)
                    .ok_or_else(|| VerificationError::InvalidGasPrice {
                        tx_hash: *tx.tx_hash(),
                    })?,
                transact_to: tx.kind(),
                value: tx.value(),
                data: tx.input(),
                nonce: tx.nonce(),
                chain_id: tx.chain_id(),
                access_list: tx.access_list().cloned().unwrap_or_default().0,
                gas_priority_fee: tx.max_priority_fee_per_gas().map(U256::from),
                blob_hashes: tx
                    .blob_versioned_hashes()
                    .map(|hashes| hashes.to_vec())
                    .unwrap_or_default(),
                max_fee_per_blob_gas: tx.max_fee_per_blob_gas().map(U256::from),
                authorization_list: tx.authorization_list().map(|auths| auths.to_vec().into()),
                #[cfg(feature = "scroll")]
                scroll: revm::primitives::ScrollFields {
                    is_l1_msg: tx.is_l1_msg(),
                    rlp_bytes: Some(rlp_bytes),
                },
            };

            #[cfg(feature = "scroll")]
            if tx.is_l1_msg() {
                env.cfg.disable_base_fee = true; // disable base fee for l1 msg
            }

            dev_trace!("{env:#?}");

            {
                let mut revm = cycle_track!(
                    revm::Evm::builder()
                        .with_spec_id(spec_id)
                        .with_db(&mut self.db)
                        .with_env(env)
                        // .with_external_context(CustomPrintTracer::default())
                        // .append_handler_register(inspector_handle_register)
                        .build(),
                    "build Evm"
                );

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
    pub fn commit_changes(self) -> B256 {
        measure_duration_millis!(
            commit_changes_duration_milliseconds,
            cycle_track!(self.commit_changes_inner(), "commit_changes")
        )
    }

    fn commit_changes_inner(mut self) -> B256 {
        let provider = &self.db.db.nodes_provider;
        let state = &mut self.db.db.state;

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
                self.db.db.code_db.or_insert_with(info.code_hash, || {
                    Bytes::copy_from_slice(code.original_byte_slice())
                });

                debug_assert_eq!(
                    info.code_hash,
                    code.hash_slow(),
                    "code hash mismatch for account {addr:?}",
                );
                #[cfg(any(feature = "debug-account", feature = "debug-storage"))]
                debug_recorder.record_code(info.code_hash, code.bytecode().as_ref());
            }

            dev_trace!("committing {addr}, {:?} {db_acc:?}", db_acc.account_state);
            cycle_tracker_start!("commit account");

            let storage_root = if !db_acc.storage.is_empty() {
                cycle_tracker_start!("update storage_tire");
                for (key, value) in db_acc.storage.iter() {
                    measure_duration_micros!(
                        zktrie_update_duration_microseconds,
                        cycle_track!(
                            state.update_storage(provider, *addr, *key, *value),
                            "Zktrie::update_store"
                        )
                    );

                    #[cfg(feature = "debug-storage")]
                    debug_recorder.record_storage(*addr, *key, *value);
                }

                let storage_root = measure_duration_micros!(
                    zktrie_commit_duration_microseconds,
                    cycle_track!(state.commit_storage(provider, *addr), "Zktrie::commit")
                );

                cycle_tracker_end!("update storage_tire");

                #[cfg(feature = "debug-storage")]
                debug_recorder.record_storage_root(*addr, storage_root);

                storage_root
            } else {
                state.storage_root(*addr)
            };

            if !info.is_empty() {
                // if account not exist, all fields will be zero.
                // but if account exist, code_hash will be empty hash if code is empty
                if info.is_empty_code_hash() {
                    info.code_hash = KECCAK_EMPTY;
                }
            } else {
                info.code_hash = B256::ZERO;
            }

            #[cfg(feature = "debug-account")]
            debug_recorder.record_account(*addr, info.clone(), storage_root);

            let trie_account = TrieAccount {
                nonce: info.nonce,
                balance: info.balance,
                storage_root,
                code_hash: info.code_hash,
            };
            dev_trace!("committing account {addr}: {trie_account:?}");
            measure_duration_micros!(
                zktrie_update_duration_microseconds,
                cycle_track!(
                    state.update_account(*addr, trie_account),
                    "Zktrie::update_account"
                )
            );

            cycle_tracker_end!("commit account");
        }

        measure_duration_micros!(
            zktrie_commit_duration_microseconds,
            cycle_track!(state.commit_state(), "Zktrie::commit")
        )
    }
}
