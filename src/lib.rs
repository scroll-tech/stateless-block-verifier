#![feature(lazy_cell)]
#![feature(slice_group_by)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
//! Stateless Block Verifier
#[macro_use]
extern crate log;

mod database;
mod executor;
mod hardfork;
/// Utilities
pub mod utils;

pub use database::ReadOnlyDB;
pub use executor::{hooks, EvmExecutor, EvmExecutorBuilder};
pub use hardfork::HardforkConfig;
