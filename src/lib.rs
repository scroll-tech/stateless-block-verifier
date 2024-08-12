//! Stateless Block Verifier

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate log;

mod macros;

mod database;
pub use database::ReadOnlyDB;

mod executor;
pub use executor::{hooks, EvmExecutor, EvmExecutorBuilder};

mod hardfork;
pub use hardfork::HardforkConfig;

mod utils;
pub use utils::{post_check, BlockTraceExt};
