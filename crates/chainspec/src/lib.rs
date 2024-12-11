//! The spec of an Ethereum network
use alloy_chains::{Chain, ChainKind, NamedChain};
use reth_chainspec::{ChainSpec, ChainSpecProvider};
use reth_ethereum_forks::ChainHardforks;
use revm::primitives::SpecId;
use sbv_primitives::BlockHeader;
use std::sync::Arc;

/// scroll chain spec
pub mod scroll;

/// The name of an Ethereum hardfork.
#[cfg(feature = "scroll")]
pub type Hardfork = scroll::ScrollHardfork;
/// The name of an Ethereum hardfork.
#[cfg(not(feature = "scroll"))]
pub type Hardfork = reth_ethereum_forks::EthereumHardfork;

/// DefaultChainSpecProvider
#[derive(Debug, Clone)]
pub struct WellKnownChainSpecProvider(Arc<ChainSpec>);

impl WellKnownChainSpecProvider {
    /// Creat a new `DefaultChainSpecProvider`

    pub fn new(chain: Chain) -> Option<Self> {
        #[cfg(feature = "scroll")]
        if chain == scroll::SCROLL_MAINNET_CHAIN_ID {
            return Some(Self(scroll::SCROLL_MAINNET.clone()));
        }
        #[cfg(feature = "scroll")]
        if chain == scroll::SCROLL_SEPOLIA_CHAIN_ID {
            return Some(Self(scroll::SCROLL_SEPOLIA.clone()));
        }
        if chain == Chain::from_named(NamedChain::Mainnet) {
            return Some(Self(reth_chainspec::MAINNET.clone()));
        }
        if chain == Chain::from_named(NamedChain::Sepolia) {
            return Some(Self(reth_chainspec::SEPOLIA.clone()));
        }
        if chain == Chain::from_named(NamedChain::Holesky) {
            return Some(Self(reth_chainspec::HOLESKY.clone()));
        }
        None
    }
}

impl ChainSpecProvider for WellKnownChainSpecProvider {
    type ChainSpec = ChainSpec;

    fn chain_spec(&self) -> Arc<Self::ChainSpec> {
        self.0.clone()
    }
}
