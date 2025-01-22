//! Stateless Block Verifier core library.

#[macro_use]
extern crate sbv_helpers;
extern crate core;

#[cfg(feature = "scroll")]
mod chunk;
#[cfg(feature = "scroll")]
pub use chunk::ChunkInfoBuilder;

mod database;
pub use database::{DatabaseError, DatabaseRef, EvmDatabase};

mod error;
pub use error::VerificationError;

mod executor;
pub use executor::EvmExecutor;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}
