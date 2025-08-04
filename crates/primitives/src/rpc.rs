pub use alloy_rpc_types_eth::{Header, Transaction as AlloyRpcTransaction};

#[cfg(not(feature = "scroll"))]
pub use alloy_rpc_types_eth::{Transaction, TransactionReceipt, TransactionRequest};
#[cfg(feature = "scroll")]
pub use scroll_alloy_rpc_types::{
    ScrollTransactionReceipt as TransactionReceipt, ScrollTransactionRequest as TransactionRequest,
    Transaction,
};

/// Block representation for RPC.
pub type Block = alloy_rpc_types_eth::Block<Transaction>;

#[cfg(feature = "consensus-types")]
use crate::{
    consensus::{Transaction as _, TxEnvelope},
    eips::{Encodable2718, Typed2718},
};

#[cfg(feature = "consensus-types")]
impl crate::Transaction {
    /// Create a transaction from a rpc transaction
    #[cfg(feature = "scroll")]
    pub fn from_rpc(tx: &Transaction) -> Self {
        crate::Transaction::from_rpc_inner(&tx.inner)
    }

    /// Create a transaction from a rpc transaction
    #[cfg(not(feature = "scroll"))]
    pub fn from_rpc(tx: &Transaction) -> Self {
        crate::Transaction::from_rpc_inner(tx)
    }

    fn from_rpc_inner(tx: &AlloyRpcTransaction<TxEnvelope>) -> Self {
        Self {
            hash: tx.inner.trie_hash(),
            nonce: tx.nonce(),
            from: tx.inner.signer(),
            to: tx.to(),
            value: tx.value(),
            gas_price: tx.gas_price(),
            gas: tx.gas_limit(),
            max_fee_per_gas: tx.max_fee_per_gas(),
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas(),
            max_fee_per_blob_gas: tx.max_fee_per_blob_gas(),
            input: tx.input().clone(),
            #[cfg(feature = "scroll")]
            signature: tx.inner.signature(),
            #[cfg(not(feature = "scroll"))]
            signature: Some(*tx.inner.signature()),
            chain_id: tx.chain_id(),
            blob_versioned_hashes: tx.blob_versioned_hashes().map(ToOwned::to_owned),
            access_list: tx.access_list().map(ToOwned::to_owned),
            transaction_type: tx.ty(),
            authorization_list: tx
                .authorization_list()
                .map(|list| list.into_iter().map(Into::into).collect()),
            #[cfg(feature = "scroll")]
            queue_index: tx.inner.as_l1_message().map(|tx| tx.queue_index),
        }
    }
}
