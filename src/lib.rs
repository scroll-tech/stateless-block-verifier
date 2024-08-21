//! Stateless Block Verifier

#![feature(error_in_core)]
#![feature(lazy_cell)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[cfg(feature = "dev")]
#[macro_use]
extern crate tracing;

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

mod utils;
pub use utils::{post_check, BlockTraceExt};

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
