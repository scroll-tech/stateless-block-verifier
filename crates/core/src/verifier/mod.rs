//! Standard block witness verifier implementation.

#[cfg(feature = "scroll")]
mod scroll;
#[cfg(feature = "scroll")]
pub use scroll::*;

#[cfg(not(feature = "scroll"))]
mod ethereum;
#[cfg(not(feature = "scroll"))]
pub use ethereum::*;
