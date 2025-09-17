#[cfg(not(feature = "scroll"))]
mod ethereum;
#[cfg(not(feature = "scroll"))]
pub use ethereum::EvmExecutor;

#[cfg(feature = "scroll")]
mod scroll;
#[cfg(feature = "scroll")]
pub use scroll::EvmExecutor;
