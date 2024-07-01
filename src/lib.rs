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
mod utils;

pub use database::ReadOnlyDB;
pub use executor::EvmExecutor;
pub use hardfork::HardforkConfig;
