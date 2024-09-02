//! Stateless Block Verifier utils library.

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[cfg(any(feature = "dev", test))]
#[doc(hidden)]
pub use tracing;

#[macro_use]
mod macros;

mod utils;
#[cfg(any(feature = "debug-account", feature = "debug-storage"))]
pub use utils::debug::DebugRecorder;
pub use utils::post_check;

/// Metrics module
#[cfg(feature = "metrics")]
#[doc(hidden)]
pub mod metrics;
