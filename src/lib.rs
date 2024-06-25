#![feature(lazy_cell)]
#![feature(slice_group_by)]
//! Stateless Block Verifier
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate log;

mod database;
mod executor;
mod utils;

pub use database::ReadOnlyDB;
pub use executor::EvmExecutor;
pub use utils::HardforkConfig;
