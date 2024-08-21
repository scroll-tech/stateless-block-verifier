//! Stateless Block Verifier

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[cfg(feature = "dev")]
#[macro_use]
extern crate tracing;

mod macros;

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
