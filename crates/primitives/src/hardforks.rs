pub use reth_ethereum_forks::{Hardfork as HardforkTrait, *};

#[cfg(not(feature = "scroll"))]
pub use reth_ethereum_forks::EthereumHardfork as Hardfork;

#[cfg(feature = "scroll-hardforks")]
pub use reth_scroll_forks::{
    DEV_HARDFORKS as SCROLL_DEV_HARDFORKS, ScrollHardfork as Hardfork, ScrollHardforks,
};
