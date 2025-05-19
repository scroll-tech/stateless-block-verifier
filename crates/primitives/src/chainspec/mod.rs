pub use reth_chainspec::{
    self, BaseFeeParams, BaseFeeParamsKind, Chain, ChainHardforks, ChainInfo, ChainKind, DEV,
    DEV_HARDFORKS, ForkCondition, HOLESKY, MAINNET, NamedChain, SEPOLIA,
};

#[cfg(feature = "scroll")]
mod scroll_chain;
#[cfg(feature = "scroll")]
pub use scroll_chain::*;

#[cfg(not(feature = "scroll"))]
mod ethereum_chain;
#[cfg(not(feature = "scroll"))]
pub use ethereum_chain::*;

#[cfg(test)]
mod tests {

    #[cfg(feature = "scroll")]
    #[test]
    fn test_build_chain_spec() {
        use super::*;
        use crate::hardforks::ScrollHardfork;

        let chain_spec = build_chain_spec(Chain::from_id(42424242), |spec| {
            spec.inner
                .hardforks
                .insert(ScrollHardfork::DarwinV2, ForkCondition::Block(10));
        });
        assert_eq!(chain_spec.chain, Chain::from_id(42424242));
        assert!(!chain_spec.is_fork_active_at_block(ScrollHardfork::DarwinV2, 0));
        assert!(chain_spec.is_fork_active_at_block(ScrollHardfork::DarwinV2, 10));
    }
}
