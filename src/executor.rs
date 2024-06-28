use crate::{
    database::ReadOnlyDB,
    utils::{collect_account_proofs, collect_storage_proofs},
    HardforkConfig,
};
use eth_types::{
    geth_types::TxType,
    l2_types::{BlockTrace, ExecutionResult},
    H160, H256, U256,
};
use log::Level;
use mpt_zktrie::{AccountData, ZktrieState};
use revm::{
    db::CacheDB,
    primitives::{AccountInfo, BlockEnv, Env, SpecId, TxEnv},
    DatabaseRef,
};
use std::fmt::Debug;
use zktrie::ZkTrie;

/// EVM executor that handles the block.
pub struct EvmExecutor {
    db: CacheDB<ReadOnlyDB>,
    zktrie: ZkTrie,
    spec_id: SpecId,
    disable_checks: bool,
}
impl EvmExecutor {
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn new(l2_trace: &BlockTrace, fork_config: &HardforkConfig, disable_checks: bool) -> Self {
        let block_number = l2_trace.header.number.unwrap().as_u64();
        let spec_id = fork_config.get_spec_id(block_number);

        let mut db = CacheDB::new(ReadOnlyDB::new(l2_trace));
        fork_config
            .migrate(block_number, &mut db)
            .expect("failed to migrate");

        let old_root = l2_trace.storage_trace.root_before;
        let zktrie_state = ZktrieState::from_trace_with_additional(
            old_root,
            collect_account_proofs(&l2_trace.storage_trace),
            collect_storage_proofs(&l2_trace.storage_trace),
            l2_trace
                .storage_trace
                .deletion_proofs
                .iter()
                .map(|s| s.as_ref()),
        )
        .unwrap();
        let root = *zktrie_state.root();
        debug!("building partial statedb done, root {}", hex::encode(root));

        let mem_db = zktrie_state.into_inner();
        let zktrie = mem_db.new_trie(&root).unwrap();

        Self {
            db,
            zktrie,
            spec_id,
            disable_checks,
        }
    }

    /// Handle a block.
    pub fn handle_block(&mut self, l2_trace: &BlockTrace) -> H256 {
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
                    .with_db(&mut self.db)
                    .with_spec_id(self.spec_id)
                    .with_env(env)
                    .build();
                let result = revm.transact_commit().unwrap(); // TODO: handle error
                trace!("{result:#?}");
            }
            debug!("handle {idx}th tx done");

            if !self.disable_checks {
                if let Some(exec) = l2_trace.execution_results.get(idx) {
                    debug!("post check {idx}th tx");
                    self.post_check(exec);
                }
            }
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

    fn post_check(&mut self, exec: &ExecutionResult) {
        for account_post_state in exec.account_after.iter() {
            let local_acc = self
                .db
                .basic_ref(account_post_state.address.0.into())
                .unwrap()
                .unwrap();
            if log_enabled!(Level::Trace) {
                let mut local_acc = local_acc.clone();
                local_acc.code = None;
                trace!("local acc {local_acc:?}, trace acc {account_post_state:?}");
            }
            let local_balance = U256(*local_acc.balance.as_limbs());
            if local_balance != account_post_state.balance {
                let post = account_post_state.balance;
                error!(
                    "incorrect balance, local {:#x} {} post {:#x} (diff {}{:#x})",
                    local_balance,
                    if local_balance < post { "<" } else { ">" },
                    post,
                    if local_balance < post { "-" } else { "+" },
                    if local_balance < post {
                        post - local_balance
                    } else {
                        local_balance - post
                    }
                )
            }
            if local_acc.nonce != account_post_state.nonce {
                error!("incorrect nonce")
            }
            let p_hash = account_post_state.poseidon_code_hash;
            if p_hash.is_zero() {
                if !local_acc.is_empty() {
                    error!("incorrect poseidon_code_hash")
                }
            } else if local_acc.code_hash.0 != p_hash.0 {
                error!("incorrect poseidon_code_hash")
            }
            let k_hash = account_post_state.keccak_code_hash;
            if k_hash.is_zero() {
                if !local_acc.is_empty() {
                    error!("incorrect keccak_code_hash")
                }
            } else if local_acc.keccak_code_hash.0 != k_hash.0 {
                error!("incorrect keccak_code_hash")
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
