//! Stateless Block Verifier core library.

#[macro_use]
extern crate sbv_utils;
extern crate core;

mod chunk;
pub use chunk::ChunkInfo;

mod database;
pub use database::EvmDatabase;

mod error;
pub use error::{DatabaseError, VerificationError};

mod executor;
pub use executor::{BlockExecutionResult, EvmExecutor, EvmExecutorBuilder};

mod genesis;
pub use genesis::GenesisConfig;

mod hardfork;
pub use hardfork::HardforkConfig;

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
