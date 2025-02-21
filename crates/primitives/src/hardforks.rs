pub use reth_ethereum_forks::*;

#[cfg(feature = "scroll-hardforks")]
pub use reth_scroll_forks::{
    DEV_HARDFORKS as SCROLL_DEV_HARDFORKS, ScrollHardfork, ScrollHardforks,
};
