use eth_types::l2_types::ExecutionResult;
use log::Level;
use revm::DatabaseRef;
use std::fmt::Debug;

pub(crate) mod ext;

/// Blanket trait for block trace extensions.
pub trait BlockTraceExt:
    ext::BlockTraceRevmExt + ext::BlockRevmDbExt + ext::BlockZktrieExt
{
    /// Get the root hash after the block.
    fn root_after(&self) -> eth_types::U256;
}

impl BlockTraceExt for eth_types::l2_types::BlockTrace {
    fn root_after(&self) -> eth_types::U256 {
        eth_types::U256::from_big_endian(&self.storage_trace.root_after.0)
    }
}

impl BlockTraceExt for eth_types::l2_types::BlockTraceV2 {
    fn root_after(&self) -> eth_types::U256 {
        eth_types::U256::from_big_endian(&self.storage_trace.root_after.0)
    }
}

impl BlockTraceExt for eth_types::l2_types::ArchivedBlockTraceV2 {
    fn root_after(&self) -> eth_types::U256 {
        eth_types::U256::from_big_endian(self.storage_trace.root_after.0.as_ref())
    }
}

impl<T: BlockTraceExt> BlockTraceExt for &T {
    fn root_after(&self) -> eth_types::U256 {
        (*self).root_after()
    }
}

/// Check the post state of the block with the execution result.
pub fn post_check<DB: DatabaseRef>(db: DB, exec: &ExecutionResult) -> bool
where
    <DB as DatabaseRef>::Error: Debug,
{
    let mut ok = true;
    for account_post_state in exec.account_after.iter() {
        let local_acc = db
            .basic_ref(account_post_state.address.0.into())
            .unwrap()
            .unwrap();
        if log_enabled!(Level::Trace) {
            let mut local_acc = local_acc.clone();
            local_acc.code = None;
            trace!("local acc {local_acc:?}, trace acc {account_post_state:?}");
        }
        let local_balance = eth_types::U256(*local_acc.balance.as_limbs());
        if local_balance != account_post_state.balance {
            ok = false;
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
            ok = false;
            error!("incorrect nonce")
        }
        let p_hash = account_post_state.poseidon_code_hash;
        if p_hash.is_zero() {
            if !local_acc.is_empty() {
                ok = false;
                error!("incorrect poseidon_code_hash")
            }
        } else if local_acc.code_hash.0 != p_hash.0 {
            ok = false;
            error!("incorrect poseidon_code_hash")
        }
        let k_hash = account_post_state.keccak_code_hash;
        if k_hash.is_zero() {
            if !local_acc.is_empty() {
                ok = false;
                error!("incorrect keccak_code_hash")
            }
        } else if local_acc.keccak_code_hash.0 != k_hash.0 {
            ok = false;
            error!("incorrect keccak_code_hash")
        }
    }
    ok
}
