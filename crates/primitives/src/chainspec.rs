use std::sync::Arc;

pub use reth_chainspec::{self, *};

#[cfg(feature = "scroll-chainspec")]
pub use reth_scroll_chainspec as scroll;
#[cfg(feature = "scroll-chainspec")]
pub use reth_scroll_chainspec::{SCROLL_DEV, SCROLL_MAINNET, SCROLL_SEPOLIA};

/// An Ethereum chain specification.
///
/// A chain specification describes:
///
/// - Meta-information about the chain (the chain ID)
/// - The genesis block of the chain (Genesis)
/// - What hardforks are activated, and under which conditions
#[cfg(not(feature = "scroll-chainspec"))]
pub type ChainSpec = reth_chainspec::ChainSpec;
/// Scroll chain spec type.
#[cfg(feature = "scroll-chainspec")]
pub type ChainSpec = scroll::ScrollChainSpec;

/// Get chain spec
#[cfg(not(feature = "scroll-chainspec"))]
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
#[cfg(feature = "scroll-chainspec")]
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
        #[cfg(not(feature = "scroll-chainspec"))]
        let mut spec = {
            let mut spec = (**DEV).clone();
            spec.chain = chain;
            spec
        };
        #[cfg(feature = "scroll-chainspec")]
        let mut spec = {
            let mut spec = (**SCROLL_DEV).clone();
            spec.inner.chain = chain;
            spec
        };

        f(&mut spec);
        Arc::new(spec)
    })
}

/// Build a chain spec with a hardfork, enabling all hardforks up to the specified one.
#[cfg(feature = "scroll-chainspec")]
pub fn build_chain_spec_force_hardfork(
    chain: Chain,
    hardfork: crate::hardforks::Hardfork,
) -> Arc<ChainSpec> {
    use crate::hardforks::Hardfork;
    use reth_scroll_chainspec::{ScrollChainConfig, ScrollChainSpec};
    use std::sync::{Arc, LazyLock};

    static BASE_HARDFORKS: LazyLock<ChainHardforks> = LazyLock::new(|| {
        ChainHardforks::new(vec![
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (
                EthereumHardfork::SpuriousDragon.boxed(),
                ForkCondition::Block(0),
            ),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (
                EthereumHardfork::Constantinople.boxed(),
                ForkCondition::Block(0),
            ),
            (
                EthereumHardfork::Petersburg.boxed(),
                ForkCondition::Block(0),
            ),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
        ])
    });

    let mut hardforks = BASE_HARDFORKS.clone();

    if hardfork >= Hardfork::Archimedes {
        hardforks.insert(Hardfork::Archimedes, ForkCondition::Timestamp(0));
    }
    if hardfork >= Hardfork::Bernoulli {
        hardforks.insert(EthereumHardfork::Shanghai, ForkCondition::Timestamp(0));
        hardforks.insert(Hardfork::Bernoulli, ForkCondition::Block(0));
    }
    if hardfork >= Hardfork::Curie {
        hardforks.insert(Hardfork::Curie, ForkCondition::Block(0));
    }
    if hardfork >= Hardfork::Darwin {
        hardforks.insert(Hardfork::Darwin, ForkCondition::Timestamp(0));
    }
    if hardfork >= Hardfork::DarwinV2 {
        hardforks.insert(Hardfork::DarwinV2, ForkCondition::Timestamp(0));
    }
    if hardfork >= Hardfork::Euclid {
        hardforks.insert(Hardfork::Euclid, ForkCondition::Timestamp(0));
    }
    if hardfork >= Hardfork::EuclidV2 {
        hardforks.insert(Hardfork::EuclidV2, ForkCondition::Timestamp(0));
    }
    if hardfork >= Hardfork::Feynman {
        hardforks.insert(Hardfork::Feynman, ForkCondition::Timestamp(0));
    }
    sbv_helpers::dev_info!(
        "Building chain spec for chain {} with hardfork {:?}",
        chain,
        hardforks
    );

    Arc::new(ScrollChainSpec {
        inner: reth_chainspec::ChainSpec {
            chain,
            hardforks,
            ..Default::default()
        },
        config: ScrollChainConfig::mainnet(),
    })
}

/// Build a chain spec with a hardfork, enabling all hardforks up to the specified one.
#[cfg(not(feature = "scroll"))]
pub fn build_chain_spec_force_hardfork(
    chain: Chain,
    hardfork: crate::hardforks::Hardfork,
) -> Arc<ChainSpec> {
    use crate::{U256, hardforks::Hardfork};
    use std::sync::{Arc, LazyLock};

    static BASE_HARDFORKS: LazyLock<ChainHardforks> = LazyLock::new(|| {
        ChainHardforks::new(vec![(
            EthereumHardfork::Frontier.boxed(),
            ForkCondition::Block(0),
        )])
    });

    let mut hardforks = BASE_HARDFORKS.clone();

    if hardfork >= Hardfork::Homestead {
        hardforks.insert(hardfork, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Dao {
        hardforks.insert(Hardfork::Dao, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Tangerine {
        hardforks.insert(Hardfork::Tangerine, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::SpuriousDragon {
        hardforks.insert(Hardfork::SpuriousDragon, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Byzantium {
        hardforks.insert(Hardfork::Byzantium, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Constantinople {
        hardforks.insert(Hardfork::Constantinople, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Petersburg {
        hardforks.insert(Hardfork::Petersburg, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Istanbul {
        hardforks.insert(Hardfork::Istanbul, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Berlin {
        hardforks.insert(Hardfork::Berlin, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::London {
        hardforks.insert(Hardfork::London, ForkCondition::Block(0));
    }

    if hardfork >= Hardfork::Paris {
        hardforks.insert(
            Hardfork::Paris,
            ForkCondition::TTD {
                activation_block_number: 0,
                fork_block: Some(0),
                total_difficulty: U256::ZERO,
            },
        );
    }

    if hardfork >= Hardfork::Shanghai {
        hardforks.insert(Hardfork::Shanghai, ForkCondition::Timestamp(0));
    }

    if hardfork >= Hardfork::Cancun {
        hardforks.insert(Hardfork::Cancun, ForkCondition::Timestamp(0));
    }

    if hardfork >= Hardfork::Prague {
        hardforks.insert(Hardfork::Prague, ForkCondition::Timestamp(0));
    }

    if hardfork >= Hardfork::Osaka {
        hardforks.insert(Hardfork::Osaka, ForkCondition::Timestamp(0));
    }

    Arc::new(ChainSpec {
        chain,
        hardforks,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "scroll-chainspec")]
    #[test]
    fn test_build_chain_spec() {
        use super::*;
        use crate::hardforks::Hardfork;

        let chain_spec = get_chain_spec_or_build(Chain::from_id(42424242), |spec| {
            spec.inner
                .hardforks
                .insert(Hardfork::DarwinV2, ForkCondition::Block(10));
        });
        assert_eq!(chain_spec.chain, Chain::from_id(42424242));
        assert!(!chain_spec.is_fork_active_at_block(Hardfork::DarwinV2, 0));
        assert!(chain_spec.is_fork_active_at_block(Hardfork::DarwinV2, 10));
    }
}
