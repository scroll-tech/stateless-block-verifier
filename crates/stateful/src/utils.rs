use alloy::primitives::ChainId;
use alloy::rpc::types::BlockTransactions;
use alloy::transports::{RpcError, TransportErrorKind};
use sbv::dev_warn;
use sbv::primitives::types::AlloyTransaction;
use sbv::primitives::Address;
use std::future::Future;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::RetryIf;

/// Retry the given future if it returns a transport error.
#[inline(always)]
pub fn retry_if_transport_error<F, Fut, T>(
    f: F,
) -> impl Future<Output = Result<T, RpcError<TransportErrorKind>>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, RpcError<TransportErrorKind>>>,
{
    RetryIf::spawn(
        ExponentialBackoff::from_millis(10).map(jitter).take(10),
        f,
        |e: &RpcError<TransportErrorKind>| {
            if e.is_transport_error() {
                dev_warn!("retrying request due to transport error: {:?}", e);
                true
            } else {
                false
            }
        },
    )
}

/// Retry the given future if it returns a transport error.
#[macro_export]
macro_rules! retry_if_transport_error {
    ($fut:expr) => {
        $crate::utils::retry_if_transport_error(|| async { $fut.await }).await
    };
}

/// Patch the given block to fix the miner and chain_id.
#[inline(always)]
pub fn patch_fix_block(
    block: &mut alloy::rpc::types::Block<AlloyTransaction>,
    coinbase: Address,
    chain_id: ChainId,
) {
    // Clique, which uses header.miner for a different purpose. in particular, header.miner != coinbase.
    block.header.miner = coinbase;

    if let BlockTransactions::Full(ref mut txs) = block.transactions {
        for (idx, tx) in txs.iter_mut().enumerate() {
            let tx_type = tx.transaction_type.unwrap_or(0);
            if tx_type == 0 && tx.signature.unwrap().v.to::<u64>() >= 35 {
                tx.chain_id = Some(chain_id);
                dev_trace!(
                    "block#{block_number} tx#{idx} is Eip155 tx but chain_id is not set",
                    block_number = block.header.number
                );
            }
        }
    }
}
