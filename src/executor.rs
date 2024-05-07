use crate::database::ReadOnlyDB;
use crate::utils::{collect_account_proofs, collect_storage_proofs};
use eth_types::{
    geth_types::TxType,
    l2_types::{BlockTrace, ExecutionResult},
    H256, U256,
};
use mpt_zktrie::{AccountData, ZktrieState};
use revm::db::{AccountState, CacheDB};
use revm::primitives::{AccountInfo, BlockEnv, Env, TxEnv};
use log::Level;
use revm::DatabaseRef;
use std::fmt::Debug;
use zktrie::ZkTrie;

/// EVM executor that handles the block.
pub struct EvmExecutor {
    db: CacheDB<ReadOnlyDB>,
    zktrie: ZkTrie,
    disable_checks: bool,
}

impl EvmExecutor {
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn new(l2_trace: &BlockTrace, disable_checks: bool) -> Self {
        let db = CacheDB::new(ReadOnlyDB::new(l2_trace));

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

        Self { db, zktrie, disable_checks }
    }

    /// Handle a block.
    pub fn handle_block(&mut self, l2_trace: &BlockTrace) -> H256 {
        debug!("handle block {:?}", l2_trace.header.number.unwrap());
        let mut env = Box::<Env>::default();
        env.cfg.chain_id = l2_trace.chain_id;
        env.block = BlockEnv::from(l2_trace);

        for (idx, (tx, exec)) in l2_trace
            .transactions
            .iter()
            .zip(l2_trace.execution_results.iter())
            .enumerate()
        {
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
            env.tx.scroll.is_l1_msg = tx_type.is_l1_msg();
            env.tx.scroll.rlp_bytes = Some(revm::primitives::Bytes::from(eth_tx.rlp().to_vec()));
            trace!("{env:#?}");
            {
                let mut revm = revm::Evm::builder()
                    .with_db(&mut self.db)
                    .with_env(env)
                    .build();
                let result = revm.transact_commit().unwrap(); // TODO: handle error
                trace!("{result:#?}");
            }
            debug!("handle {idx}th tx done");

            if !self.disable_checks {
                debug!("post check {idx}th tx");
                self.post_check(exec);
            }
        }
        self.commit_changes();
        H256::from(self.zktrie.root())
    }

    fn commit_changes(&mut self) {
        // let changes = self.db.accounts;
        let sdb = &self.db.db.sdb;
        for (addr, db_acc) in self.db.accounts.iter() {
            if matches!(db_acc.account_state, AccountState::None) {
                continue;
            }
            let info: AccountInfo = db_acc.info().unwrap(); // there's no self-destruct
            let mut acc_data = self
                .zktrie
                .get_account(addr.as_slice())
                .map(AccountData::from)
                .unwrap_or_default();
            acc_data.nonce = info.nonce;
            acc_data.balance = U256(*info.balance.as_limbs());
            if !db_acc.storage.is_empty() {
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
                    } else {
                        storage_tire.delete(&key.to_be_bytes::<32>());
                    }
                }
                acc_data.storage_root = H256::from(storage_tire.root());
            }
            if sdb.get_account(&addr.into_array().into()).1.is_empty() && !info.is_empty() {
                acc_data.poseidon_code_hash = H256::from(info.code_hash.0);
                acc_data.keccak_code_hash = H256::from(info.keccak_code_hash.0);
                acc_data.code_size = self
                    .db
                    .contracts
                    .get(&db_acc.info.code_hash)
                    .map(|c| c.len())
                    .unwrap_or_default() as u64;
            }
            self.zktrie
                .update_account(addr.as_slice(), &acc_data.into())
                .expect("failed to update account");
        }
    }

    fn post_check(&mut self, exec: &ExecutionResult) {
        for account_post_state in exec.account_after.iter() {
            if let Some(address) = account_post_state.address {
                let local_acc = self.db.basic_ref(address.0.into()).unwrap().unwrap();
                if log_enabled!(Level::Trace) {
                    let mut local_acc = local_acc.clone();
                    local_acc.code = None;
                    trace!("local acc {local_acc:?}, trace acc {account_post_state:?}");
                }
                let local_balance = U256(*local_acc.balance.as_limbs());
                if local_balance != account_post_state.balance.unwrap() {
                    let post = account_post_state.balance.unwrap();
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
                if local_acc.nonce != account_post_state.nonce.unwrap() {
                    error!("incorrect nonce")
                }
                let p_hash = account_post_state.poseidon_code_hash.unwrap();
                if p_hash.is_zero() {
                    if !local_acc.is_empty() {
                        error!("incorrect poseidon_code_hash")
                    }
                } else if local_acc.code_hash.0 != p_hash.0 {
                    error!("incorrect poseidon_code_hash")
                }
                let k_hash = account_post_state.keccak_code_hash.unwrap();
                if k_hash.is_zero() {
                    if !local_acc.is_empty() {
                        error!("incorrect keccak_code_hash")
                    }
                } else if local_acc.keccak_code_hash.0 != k_hash.0 {
                    error!("incorrect keccak_code_hash")
                }
                if let Some(storage) = account_post_state.storage.clone() {
                    let k = storage.key.unwrap();
                    let local_v = self.db.db.sdb.get_storage(&address, &k).1;
                    if *local_v != storage.value.unwrap() {
                        error!("incorrect storage for k = {k}")
                    }
                }
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
