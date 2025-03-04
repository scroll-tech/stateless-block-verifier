use std::sync::Arc;

pub use reth_chainspec;
pub use reth_chainspec::*;

#[cfg(feature = "scroll")]
pub use reth_scroll_chainspec as scroll;
#[cfg(feature = "scroll")]
pub use reth_scroll_chainspec::{SCROLL_DEV, SCROLL_MAINNET, SCROLL_SEPOLIA};

/// An Ethereum chain specification.
///
/// A chain specification describes:
///
/// - Meta-information about the chain (the chain ID)
/// - The genesis block of the chain ([`Genesis`])
/// - What hardforks are activated, and under which conditions
#[cfg(not(feature = "scroll"))]
pub type ChainSpec = reth_chainspec::ChainSpec;
/// Scroll chain spec type.
#[cfg(feature = "scroll")]
pub type ChainSpec = scroll::ScrollChainSpec;

/// Get chain spec
#[cfg(not(feature = "scroll"))]
pub fn get_chain_spec(chain: Chain) -> Option<Arc<ChainSpec>> {
    if chain == Chain::from_named(NamedChain::Mainnet) {
        return Some(MAINNET.clone());
    }
    if chain == Chain::from_named(NamedChain::Sepolia) {
        return Some(SEPOLIA.clone());
    }
    if chain == Chain::from_named(NamedChain::Holesky) {
        return Some(HOLESKY.clone());
    }
    if chain == Chain::dev() {
        return Some(DEV.clone());
    }
    None
}

/// Get chain spec
#[cfg(feature = "scroll")]
pub fn get_chain_spec(chain: Chain) -> Option<Arc<ChainSpec>> {
    if chain == Chain::from_named(NamedChain::Scroll) {
        return Some(SCROLL_MAINNET.clone());
    }
    if chain == Chain::from_named(NamedChain::ScrollSepolia) {
        return Some(SCROLL_SEPOLIA.clone());
    }
    if chain == Chain::dev() {
        return Some(SCROLL_DEV.clone());
    }
    None
}

/// Get chain spec or build one from dev config as blueprint
pub fn get_chain_spec_or_build<F>(chain: Chain, f: F) -> Arc<ChainSpec>
where
    F: Fn(&mut ChainSpec),
{
    get_chain_spec(chain).unwrap_or_else(|| {
        #[cfg(not(feature = "scroll"))]
        let mut spec = {
            let mut spec = (**DEV).clone();
            spec.chain = chain;
            spec
        };
        #[cfg(feature = "scroll")]
        let mut spec = {
            let mut spec = (**SCROLL_DEV).clone();
            spec.inner.chain = chain;
            spec
        };

        f(&mut spec);
        Arc::new(spec)
    })
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "scroll")]
    #[test]
    fn test_build_chain_spec() {
        use super::*;
        use crate::hardforks::ScrollHardfork;

        let chain_spec = get_chain_spec_or_build(Chain::from_id(42424242), |spec| {
            spec.inner
                .hardforks
                .insert(ScrollHardfork::DarwinV2, ForkCondition::Block(10));
        });
        assert_eq!(chain_spec.chain, Chain::from_id(42424242));
        assert!(!chain_spec.is_fork_active_at_block(ScrollHardfork::DarwinV2, 0));
        assert!(chain_spec.is_fork_active_at_block(ScrollHardfork::DarwinV2, 10));
    }
}
