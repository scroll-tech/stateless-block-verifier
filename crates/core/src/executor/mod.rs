#[cfg(not(feature = "scroll"))]
mod ethereum;
#[cfg(not(feature = "scroll"))]
pub use ethereum::{EvmConfig, EvmExecutor, SbvEthEvmFactory};

#[cfg(feature = "scroll")]
mod scroll;
#[cfg(feature = "scroll")]
pub use scroll::{EvmConfig, EvmExecutor};
