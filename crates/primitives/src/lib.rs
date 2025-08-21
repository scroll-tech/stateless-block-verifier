//! Stateless Block Verifier primitives library.

/// The spec of an Ethereum network
#[cfg(feature = "chainspec")]
pub mod chainspec;

/// Extension Traits
pub mod ext;

/// Ethereum fork types
#[cfg(feature = "hardforks")]
pub mod hardforks {
    pub use reth_ethereum_forks::{Hardfork as HardforkTrait, *};

    #[cfg(not(feature = "scroll"))]
    pub use reth_ethereum_forks::EthereumHardfork as Hardfork;

    #[cfg(feature = "scroll-hardforks")]
    pub use reth_scroll_forks::{
        DEV_HARDFORKS as SCROLL_DEV_HARDFORKS, ScrollHardfork as Hardfork, ScrollHardforks,
    };
}

/// Legacy Types definition leave for backward compatibility
pub mod legacy_types;

/// Types definition
pub mod types;

pub use alloy_primitives::{
    self, Address, B64, B256, BlockHash, BlockNumber, Bloom, Bytes, ChainId, Signature,
    SignatureError, TxHash, U8, U256, address, b256, keccak256,
};
