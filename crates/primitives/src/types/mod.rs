mod access_list;
mod auth_list;
mod block_header;
#[cfg(feature = "scroll")]
mod scroll;
mod signature;
mod transaction;
mod withdrawal;
mod witness;

pub use access_list::{AccessList, AccessListItem, ArchivedAccessList, ArchivedAccessListItem};
pub use alloy_consensus::{Header as AlloyHeader, TypedTransaction as AlloyTypedTransaction};
pub use alloy_eips::eip4895::{Withdrawal as AlloyWithdrawal, Withdrawals as AlloyWithdrawals};
pub use block_header::{ArchivedBlockHeader, BlockHeader};
#[cfg(feature = "scroll")]
pub use scroll::*;
pub use signature::{ArchivedSignature, Signature};
pub use transaction::{ArchivedTransaction, Transaction};
pub use withdrawal::{ArchivedWithdrawal, Withdrawal};
pub use witness::{ArchivedBlockWitness, BlockWitness, ExecutionWitness};

/// re-export types from alloy_consensus
pub mod consensus {
    pub use alloy_consensus::{
        BlockHeader, Header, SignableTransaction, Transaction, TxEip1559, TxEip2930, TxEip4844,
        TxEip4844Variant, TxEip4844WithSidecar, TxEip7702, TxLegacy, Typed2718,
    };

    #[cfg(not(feature = "scroll"))]
    pub use alloy_consensus::{TxEnvelope, TxType, TypedTransaction};
    use reth_primitives_traits::transaction::signature::Signature;
    pub use reth_primitives_traits::transaction::signed::SignedTransaction;
    #[cfg(feature = "scroll")]
    pub use scroll_alloy_consensus::{
        ScrollReceiptEnvelope as ReceiptEnvelope, ScrollTxEnvelope as TxEnvelope,
        ScrollTxType as TxType, ScrollTypedTransaction as TypedTransaction, TxL1Message,
    };

    /// Extension trait for `TxEnvelope`
    pub trait TxEnvelopeExt {
        /// get the signature of the transaction
        fn signature(&self) -> Option<&Signature>;

        /// get the index of the transaction in the queue
        fn queue_index(&self) -> Option<u64> {
            None
        }
    }

    #[cfg(not(feature = "scroll"))]
    impl TxEnvelopeExt for TxEnvelope {
        fn signature(&self) -> Option<&Signature> {
            Some(TxEnvelope::signature(self))
        }
    }

    #[cfg(feature = "scroll")]
    impl TxEnvelopeExt for TxEnvelope {
        fn signature(&self) -> Option<&Signature> {
            match self {
                TxEnvelope::Legacy(tx) => Some(tx.signature()),
                TxEnvelope::Eip2930(tx) => Some(tx.signature()),
                TxEnvelope::Eip1559(tx) => Some(tx.signature()),
                TxEnvelope::Eip7702(tx) => Some(tx.signature()),
                _ => None,
            }
        }

        fn queue_index(&self) -> Option<u64> {
            match self {
                TxEnvelope::L1Message(tx) => Some(tx.queue_index),
                _ => None,
            }
        }
    }
}

/// re-export types from reth_primitives
pub mod reth {
    #[cfg(not(feature = "scroll"))]
    pub use reth_primitives::{Block, BlockBody, Receipt, TransactionSigned};
    #[cfg(feature = "scroll")]
    pub use reth_scroll_primitives::{
        ScrollBlock as Block, ScrollBlockBody as BlockBody, ScrollPrimitives,
        ScrollReceipt as Receipt, ScrollTransactionSigned as TransactionSigned,
    };
}

/// re-export types from alloy_rpc_types_eth
pub mod rpc {
    pub use alloy_rpc_types_eth::{Header, Transaction as AlloyRpcTransaction};

    #[cfg(not(feature = "scroll"))]
    pub use alloy_rpc_types_eth::{Transaction, TransactionReceipt, TransactionRequest};

    #[cfg(feature = "scroll")]
    pub use scroll_alloy_rpc_types::{
        ScrollTransactionReceipt as TransactionReceipt,
        ScrollTransactionRequest as TransactionRequest, Transaction,
    };

    /// Block representation for RPC.
    pub type Block = alloy_rpc_types_eth::Block<Transaction>;
}
