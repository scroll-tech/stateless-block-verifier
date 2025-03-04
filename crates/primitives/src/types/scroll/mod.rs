use crate::B256;

mod chunk;
pub use chunk::*;
#[cfg(all(feature = "scroll-reth-types", feature = "scroll-hardforks"))]
mod chunk_builder;
#[cfg(all(feature = "scroll-reth-types", feature = "scroll-hardforks"))]
pub use chunk_builder::*;

/// RPC response of the `scroll_diskRoot` method.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiskRoot {
    /// MPT state root
    #[cfg_attr(feature = "serde", serde(rename = "diskRoot"))]
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "MPT state root")))]
    pub disk_root: B256,
    /// B-MPT state root
    #[cfg_attr(feature = "serde", serde(rename = "headerRoot"))]
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "B-MPT state root")))]
    pub header_root: B256,
}
