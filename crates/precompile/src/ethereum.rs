use super::PrecompileProvider;
use sbv_primitives::types::{evm::precompiles::PrecompilesMap, revm::precompile::PrecompileSpecId};

#[cfg(not(feature = "ethereum-openvm"))]
impl PrecompileProvider {
    /// Returns the precompiles map for the given spec.
    #[inline]
    pub fn with_spec(spec: PrecompileSpecId) -> PrecompilesMap {
        PrecompilesMap::from_static(Precompiles::new(spec))
    }
}

#[cfg(feature = "ethereum-openvm")]
impl PrecompileProvider {
    /// Returns the precompiles map for the given spec.
    pub fn with_spec(_spec: PrecompileSpecId) -> PrecompilesMap {
        todo!()
    }
}
