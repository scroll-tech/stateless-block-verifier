//! Stateless Block Verifier primitives library.

use auto_impl::auto_impl;
use std::fmt;

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
/// Types definition
pub mod types;

pub use alloy_primitives::{
    self, Address, B64, B256, BlockHash, BlockNumber, Bloom, Bytes, ChainId, Signature,
    SignatureError, TxHash, U8, U256, address, b256, keccak256,
};

/// BlockWitness trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitness: fmt::Debug {
    /// Chain id
    fn chain_id(&self) -> ChainId;
    /// Block number
    fn number(&self) -> BlockNumber;
    /// Pre-state root
    fn pre_state_root(&self) -> B256;
    /// Pre-state root
    fn post_state_root(&self) -> B256;
    /// Withdrawal root
    fn withdrawals_root(&self) -> Option<B256>;
    /// Number of transactions
    fn num_transactions(&self) -> usize;
    /// Block hashes
    #[must_use]
    #[cfg(not(feature = "scroll"))]
    fn block_hashes_iter(&self) -> impl ExactSizeIterator<Item = B256>;
    /// Withdrawals
    #[must_use]
    fn withdrawals_iter(&self) -> Option<impl ExactSizeIterator<Item = impl Withdrawal>>;
    /// States
    #[must_use]
    fn states_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>>;
    /// Codes
    #[must_use]
    fn codes_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>>;

    // provided methods

    /// Number of states
    fn num_states(&self) -> usize {
        self.states_iter().len()
    }
    /// Number of codes
    fn num_codes(&self) -> usize {
        self.codes_iter().len()
    }
}

/// Withdrawal trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Withdrawal: fmt::Debug {
    /// Monotonically increasing identifier issued by consensus layer.
    fn index(&self) -> u64;
    /// Index of validator associated with withdrawal.
    fn validator_index(&self) -> u64;
    /// Target address for withdrawn ether.
    fn address(&self) -> Address;
    /// Value of the withdrawal in gwei.
    fn amount(&self) -> u64;
}
