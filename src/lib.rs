//! Stateless Block Verifier
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate log;

mod database;
mod executor;
mod utils;

pub use database::EvmDatabase;
pub use executor::EvmExecutor;
