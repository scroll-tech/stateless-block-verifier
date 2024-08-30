use eth_types::l2_types::ExecutionResult;

use revm::DatabaseRef;
use std::fmt::Debug;

#[cfg(feature = "dev")]
use tracing::Level;

/// Debugging utilities.
#[cfg(any(feature = "debug-account", feature = "debug-storage"))]
pub(crate) mod debug;
/// Extensions for block trace.
pub mod ext;

/// Blanket trait for block trace extensions.
pub trait BlockTraceExt:
    ext::BlockTraceExt + ext::BlockTraceRevmExt + ext::BlockZktrieExt + ext::BlockChunkExt
{
}

impl BlockTraceExt for eth_types::l2_types::BlockTrace {}

impl BlockTraceExt for eth_types::l2_types::BlockTraceV2 {}

impl BlockTraceExt for eth_types::l2_types::ArchivedBlockTraceV2 {}
impl<T: BlockTraceExt> BlockTraceExt for &T {}

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

        #[cfg(feature = "dev")]
        if tracing::enabled!(Level::TRACE) {
            let mut local_acc = local_acc.clone();
            local_acc.code = None;
            dev_trace!("local acc {local_acc:?}, trace acc {account_post_state:?}");
        }
        let local_balance = eth_types::U256(*local_acc.balance.as_limbs());
        if local_balance != account_post_state.balance {
            ok = false;

            let _post = account_post_state.balance;
            #[cfg(feature = "dev")]
            dev_error!(
                "incorrect balance, local {:#x} {} post {:#x} (diff {}{:#x})",
                local_balance,
                if local_balance < _post { "<" } else { ">" },
                _post,
                if local_balance < _post { "-" } else { "+" },
                if local_balance < _post {
                    _post - local_balance
                } else {
                    local_balance - _post
                }
            )
        }
        if local_acc.nonce != account_post_state.nonce {
            ok = false;

            dev_error!("incorrect nonce")
        }
        let p_hash = account_post_state.poseidon_code_hash;
        if p_hash.is_zero() {
            if !local_acc.is_empty() {
                ok = false;

                dev_error!("incorrect poseidon_code_hash")
            }
        } else if local_acc.poseidon_code_hash.0 != p_hash.0 {
            ok = false;

            dev_error!("incorrect poseidon_code_hash")
        }
        let k_hash = account_post_state.keccak_code_hash;
        if k_hash.is_zero() {
            if !local_acc.is_empty() {
                ok = false;

                dev_error!("incorrect keccak_code_hash")
            }
        } else if local_acc.code_hash.0 != k_hash.0 {
            ok = false;

            dev_error!("incorrect keccak_code_hash")
        }
    }
    ok
}
