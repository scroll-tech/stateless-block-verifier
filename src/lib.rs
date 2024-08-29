//! Stateless Block Verifier

#![feature(lazy_cell)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[cfg(feature = "dev")]
#[doc(hidden)]
pub use tracing;

#[macro_use]
mod macros;

mod chunk;

pub use chunk::ChunkInfo;

mod database;
pub use database::ReadOnlyDB;

mod error;
pub use error::VerificationError;

mod executor;
pub use executor::{hooks, EvmExecutor, EvmExecutorBuilder};

mod hardfork;
pub use hardfork::HardforkConfig;

/// Module for utilities.
pub mod utils;
pub use utils::{post_check, BlockTraceExt};

/// Metrics module
#[cfg(feature = "metrics")]
#[doc(hidden)]
pub mod metrics;

#[cfg(all(feature = "dev", test))]
#[ctor::ctor]
fn init() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}
