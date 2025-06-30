//! sbv precompiles provider
#![cfg_attr(docsrs, feature(doc_cfg))]
#[cfg(not(feature = "scroll"))]
mod ethereum;
mod imps;
#[cfg(feature = "scroll")]
#[cfg_attr(docsrs, doc(cfg(feature = "scroll")))]
mod scroll;

#[allow(unused_imports)]
pub use imps::*;

/// A precompile provider that patches the precompiles when runs on zkVM with OpenVM enabled.
#[derive(Debug, Default, Copy, Clone)]
pub struct PrecompileProvider;
