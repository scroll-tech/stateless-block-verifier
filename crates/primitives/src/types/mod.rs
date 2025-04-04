mod access_list;
mod auth_list;
mod block_header;
mod signature;
mod transaction;
mod withdrawal;
mod witness;

pub use access_list::AccessList;
pub use block_header::BlockHeader;
pub use signature::Signature;
pub use transaction::Transaction;
pub use withdrawal::Withdrawal;
pub use witness::{BlockWitness, ExecutionWitness};

#[cfg(feature = "rkyv")]
mod rkyv_types {
    pub use super::{
        access_list::{ArchivedAccessList, ArchivedAccessListItem},
        block_header::ArchivedBlockHeader,
        signature::ArchivedSignature,
        transaction::ArchivedTransaction,
        withdrawal::ArchivedWithdrawal,
        witness::ArchivedBlockWitness,
    };
}
#[cfg(feature = "rkyv")]
pub use rkyv_types::*;

/// re-export types from alloy_consensus
#[cfg(feature = "consensus-types")]
pub mod consensus;
#[cfg(feature = "consensus-types")]
pub use consensus::{Header as AlloyHeader, TypedTransaction as AlloyTypedTransaction};

/// re-export types from alloy_eips
#[cfg(feature = "eips")]
pub mod eips;

#[cfg(feature = "eips")]
pub use eips::eip4895::{Withdrawal as AlloyWithdrawal, Withdrawals as AlloyWithdrawals};

/// re-export types from alloy_network
#[cfg(feature = "network-types")]
pub mod network {
    /// Network definition
    #[cfg(not(feature = "scroll"))]
    pub type Network = alloy_network::Ethereum;
    /// Network definition
    #[cfg(feature = "scroll")]
    pub type Network = scroll_alloy_network::Scroll;
}
#[cfg(feature = "network-types")]
pub use network::*;

/// re-export types from revm
#[cfg(feature = "revm-types")]
pub use revm;
#[cfg(feature = "revm-types")]
pub use revm::primitives::{AccountInfo, Bytecode};

/// re-export types from reth_primitives
#[cfg(feature = "reth-types")]
pub mod reth;

/// re-export types from alloy_rpc_types_eth
#[cfg(feature = "rpc-types")]
pub mod rpc;

/// Scroll types
#[cfg(feature = "scroll")]
pub mod scroll;
