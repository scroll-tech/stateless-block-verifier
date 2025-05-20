use super::*;
use crate::hardforks::*;
use std::sync::{Arc, LazyLock};

pub use reth_scroll_chainspec as scroll;
pub use reth_scroll_chainspec::ScrollChainConfig;

/// Scroll chain spec type.
pub type ChainSpec = reth_scroll_chainspec::ScrollChainSpec;

/// The Scroll Mainnet spec without genesis.
///
/// Use [`scroll::SCROLL_MAINNET`] if you need full chain spec.
pub static SCROLL_MAINNET: LazyLock<Arc<ChainSpec>> = LazyLock::new(|| {
    let mut hardforks = ScrollHardfork::scroll_mainnet();
    // see: https://github.com/scroll-tech/sre-helm-charts/blob/main/config/genesis/genesis.mainnet.json
    // FIXME: when scroll-reth is up-to-date, we could remove this
    hardforks.insert(ScrollHardfork::Euclid, ForkCondition::Timestamp(1744815600));
    hardforks.insert(
        ScrollHardfork::EuclidV2,
        ForkCondition::Timestamp(1745305200),
    );
    Arc::new(build_chain_spec_cheap(
        Chain::from_named(NamedChain::Scroll),
        hardforks,
        ScrollChainConfig::mainnet(),
    ))
});

/// The Scroll Sepolia spec without genesis.
///
/// Use [`scroll::SCROLL_SEPOLIA`] if you need full chain spec.
pub static SCROLL_SEPOLIA: LazyLock<Arc<ChainSpec>> = LazyLock::new(|| {
    let mut hardforks = ScrollHardfork::scroll_sepolia();
    // see: https://github.com/scroll-tech/sre-helm-charts/blob/main/config/genesis/genesis.sepolia.json
    // FIXME: when scroll-reth is up-to-date, we could remove this
    hardforks.insert(ScrollHardfork::Euclid, ForkCondition::Timestamp(1741680000));
    hardforks.insert(
        ScrollHardfork::EuclidV2,
        ForkCondition::Timestamp(1741852800),
    );
    Arc::new(build_chain_spec_cheap(
        Chain::from_named(NamedChain::ScrollSepolia),
        ScrollHardfork::scroll_sepolia(),
        ScrollChainConfig::sepolia(),
    ))
});

/// The Scroll Dev spec without genesis.
///
/// Use [`scroll::SCROLL_DEV`] if you need full chain spec.
pub static SCROLL_DEV: LazyLock<Arc<ChainSpec>> = LazyLock::new(|| {
    Arc::new(build_chain_spec_cheap(
        Chain::dev(),
        (*SCROLL_DEV_HARDFORKS).clone(),
        ScrollChainConfig::dev(),
    ))
});

/// Get chain spec.
///
/// Returns [`None`] if the chain is mainnet, sepolia or dev.
///
/// Use [`get_chain_spec_or_build`] if you need a chain spec for other chains.
pub fn get_chain_spec(chain: Chain) -> Option<Arc<ChainSpec>> {
    if chain == Chain::from_named(NamedChain::Scroll) {
        Some(SCROLL_MAINNET.clone())
    } else if chain == Chain::from_named(NamedChain::ScrollSepolia) {
        Some(SCROLL_SEPOLIA.clone())
    } else if chain == Chain::dev() {
        Some(SCROLL_DEV.clone())
    } else {
        None
    }
}

/// Build a chainspec from dev chainspecc as blueprint.
pub fn build_chain_spec<F>(chain: Chain, f: F) -> Arc<ChainSpec>
where
    F: Fn(&mut ChainSpec),
{
    let mut spec = (**SCROLL_DEV).clone();
    spec.inner.chain = chain;
    f(&mut spec);
    Arc::new(spec)
}

fn build_chain_spec_cheap(
    chain: Chain,
    hardforks: ChainHardforks,
    config: ScrollChainConfig,
) -> ChainSpec {
    let inner = reth_chainspec::ChainSpec {
        chain,
        genesis_hash: Default::default(),
        genesis: Default::default(),
        genesis_header: Default::default(),
        paris_block_and_final_difficulty: Default::default(),
        hardforks,
        deposit_contract: Default::default(),
        base_fee_params: BaseFeeParamsKind::Constant(BaseFeeParams::ethereum()),
        prune_delete_limit: 20000,
        blob_params: Default::default(),
        // using `..Default::default()` would trigger the mainnet genesis deserialization
    };
    let spec = ChainSpec { inner, config };
    // We don't want to deserialize the genesis in scroll mode, so we check if it is already
    if reth_primitives_traits::sync::LazyLock::get(&MAINNET).is_some() {
        sbv_helpers::dev_warn!(
            "MAINNET genesis got deserialized, this should not happen in scroll mode"
        );
    }
    spec
}
