use super::*;
use std::sync::Arc;

/// An Ethereum chain specification.
///
/// A chain specification describes:
///
/// - Meta-information about the chain (the chain ID)
/// - The genesis block of the chain ([`Genesis`])
/// - What hardforks are activated, and under which conditions
pub type ChainSpec = reth_chainspec::ChainSpec;

/// Get chain spec
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

/// Get chain spec or build one from mainnet config as blueprint
pub fn get_chain_spec_or_build<F>(chain: Chain, f: F) -> Arc<ChainSpec>
where
    F: Fn(&mut ChainSpec),
{
    crate::chainspec::get_chain_spec(chain).unwrap_or_else(|| {
        let mut spec = {
            let mut spec = (**MAINNET).clone();
            spec.chain = chain;
            spec
        };
        f(&mut spec);
        Arc::new(spec)
    })
}
