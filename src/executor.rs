use crate::database::EvmDatabase;
use eth_types::{
    geth_types::TxType,
    l2_types::{BlockTrace, ExecutionResult},
    H256, U256,
};
use log::Level;
use revm::primitives::{BlockEnv, Env, TxEnv};
use revm::DatabaseRef;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor {
    db: EvmDatabase,
    disable_checks: bool,
}

impl EvmExecutor {
    /// Initialize an EVM executor from a block trace as the initial state.
    pub fn new(l2_trace: &BlockTrace, disable_checks: bool) -> Self {
        let db = EvmDatabase::new(l2_trace);

        Self { db, disable_checks }
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
                self.post_check(exec);
            }
        }
        self.db.commit_cache();
        self.db.root()
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
                let local_balance = U256::from_little_endian(local_acc.balance.as_le_slice());
                if local_balance != account_post_state.balance.unwrap() {
                    let local = local_balance;
                    let post = account_post_state.balance.unwrap();
                    error!(
                        "incorrect balance, local {:#x} {} post {:#x} (diff {}{:#x})",
                        local,
                        if local < post { "<" } else { ">" },
                        post,
                        if local < post { "-" } else { "+" },
                        if local < post {
                            post - local
                        } else {
                            local - post
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
                    let local_v = self.db.sdb.get_storage(&address, &k).1;
                    if *local_v != storage.value.unwrap() {
                        error!("incorrect storage for k = {k}")
                    }
                }
            }
        }
    }
}
