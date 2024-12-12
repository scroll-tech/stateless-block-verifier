//! The spec of an Ethereum network
use alloy_chains::{Chain, NamedChain};
use reth_chainspec::{ChainSpec, ChainSpecProvider};
use sbv_primitives::BlockHeader;
use std::sync::Arc;

/// scroll chain spec
pub mod scroll;

/// The name of an Ethereum hardfork.
#[cfg(not(feature = "scroll"))]
pub type Hardfork = reth_ethereum_forks::EthereumHardfork;
/// The name of an Ethereum hardfork.
#[cfg(feature = "scroll")]
pub type Hardfork = scroll::ScrollHardfork;

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

/// Map the latest active hardfork at the given block to a revm [`SpecId`](revm_primitives::SpecId).
#[cfg(not(feature = "scroll"))]
pub use reth_evm_ethereum::revm_spec;

/// Map the latest active hardfork at the given block to a revm [`SpecId`](revm_primitives::SpecId).
#[cfg(feature = "scroll")]
pub fn revm_spec(
    chain_spec: &ChainSpec,
    block: &reth_ethereum_forks::Head,
) -> revm::primitives::SpecId {
    use revm::primitives::SpecId::*;

    if chain_spec
        .fork(Hardfork::PreBernoulli)
        .active_at_head(block)
    {
        PRE_BERNOULLI
    } else if chain_spec.fork(Hardfork::Bernoulli).active_at_head(block) {
        BERNOULLI
    } else if chain_spec.fork(Hardfork::Curie).active_at_head(block) {
        CURIE
    } else if chain_spec.fork(Hardfork::Euclid).active_at_head(block) {
        EUCLID
    } else {
        reth_evm_ethereum::revm_spec(chain_spec, block)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use reth_ethereum_forks::Head;
    use revm::primitives::SpecId;

    #[cfg(feature = "scroll")]
    #[test]
    fn test_scroll_chain_spec() {
        let provider = WellKnownChainSpecProvider::new(scroll::SCROLL_MAINNET_CHAIN_ID).unwrap();
        let spec = provider.chain_spec();
        assert_eq!(
            SpecId::PRE_BERNOULLI,
            revm_spec(
                &spec,
                &Head {
                    number: 10000,
                    ..Default::default()
                },
            )
        );
        assert_eq!(
            SpecId::BERNOULLI,
            revm_spec(
                &spec,
                &Head {
                    number: 5220340,
                    ..Default::default()
                },
            )
        );
        assert_eq!(
            SpecId::BERNOULLI,
            revm_spec(
                &spec,
                &Head {
                    number: 5220341,
                    ..Default::default()
                },
            )
        );
        assert_eq!(
            SpecId::CURIE,
            revm_spec(
                &spec,
                &Head {
                    number: 7096836,
                    ..Default::default()
                },
            )
        );
        assert_eq!(
            SpecId::CURIE,
            revm_spec(
                &spec,
                &Head {
                    number: 7096837,
                    ..Default::default()
                },
            )
        );
    }

    #[test]
    fn test_holesky_chain_spec() {
        let provider =
            WellKnownChainSpecProvider::new(Chain::from_named(NamedChain::Holesky)).unwrap();
        let spec = provider.chain_spec();
        assert_eq!(
            SpecId::CANCUN,
            revm_spec(
                &spec,
                &Head {
                    number: 0x2ba60c,
                    timestamp: 0x674e72a4,
                    ..Default::default()
                },
            )
        );
    }
}
