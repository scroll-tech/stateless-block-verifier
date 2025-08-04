//! Stateless Block Verifier primitives library.

/// The spec of an Ethereum network
#[cfg(feature = "chainspec")]
pub mod chainspec;
/// Extension Traits
pub mod ext;
/// Ethereum fork types
#[cfg(feature = "hardforks")]
pub mod hardforks;
/// Predeployed contracts
#[cfg(feature = "scroll-pre-deployed")]
pub mod predeployed;

pub use alloy_primitives::{
    self, Address, B64, B256, BlockHash, BlockNumber, Bloom, Bytes, ChainId, Signature,
    SignatureError, TxHash, U8, U256, address, b256, keccak256,
};

mod block_header;
mod transaction;
mod witness;

pub use block_header::BlockHeader;
pub use transaction::Transaction;
pub use witness::{BlockWitness, ExecutionWitness};

/// re-export types from alloy_consensus
#[cfg(feature = "consensus-types")]
pub mod consensus;
#[cfg(feature = "consensus-types")]
pub use consensus::{Header as AlloyHeader, TypedTransaction as AlloyTypedTransaction};

/// re-export types from alloy_eips
pub use alloy_eips as eips;

pub use eips::eip4895::{Withdrawal, Withdrawals};

pub use eips::eip2930::{AccessList, AccessListItem};

mod auth_list;
pub use auth_list::{Authorization, SignedAuthorization};

/// re-export types from alloy-evm
#[cfg(feature = "evm-types")]
pub mod evm {
    pub use alloy_evm::precompiles;

    #[cfg(feature = "scroll-evm-types")]
    pub use scroll_alloy_evm::{
        ScrollBlockExecutor, ScrollPrecompilesFactory, ScrollTxCompressionRatios,
    };

    #[cfg(feature = "scroll-compress-ratio")]
    pub use scroll_alloy_evm::compute_compression_ratio;
}

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
pub mod revm {
    pub use revm::{bytecode::Bytecode, database, precompile, state::AccountInfo};

    #[cfg(not(feature = "scroll"))]
    pub use revm::primitives::hardfork::SpecId;

    #[cfg(feature = "scroll-revm-types")]
    pub use revm_scroll::{ScrollSpecId as SpecId, precompile::ScrollPrecompileProvider};
}

/// re-export types from reth_primitives
#[cfg(feature = "reth-types")]
pub mod reth;

/// re-export types from alloy_rpc_types_eth
#[cfg(feature = "rpc-types")]
pub mod rpc;

/// Scroll types
#[cfg(feature = "scroll")]
pub mod scroll;
