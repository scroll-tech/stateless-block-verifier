use super::PrecompileProvider;
use sbv_primitives::types::{
    evm::{ScrollPrecompilesFactory, precompiles::PrecompilesMap},
    revm::{ScrollPrecompileProvider, SpecId},
};

#[cfg(not(feature = "scroll-openvm"))]
impl ScrollPrecompilesFactory for PrecompileProvider {
    fn with_spec(spec: SpecId) -> PrecompilesMap {
        PrecompilesMap::from_static(ScrollPrecompileProvider::new_with_spec(spec).precompiles())
    }
}

#[cfg(feature = "scroll-openvm")]
impl ScrollPrecompilesFactory for PrecompileProvider {
    fn with_spec(spec: SpecId) -> PrecompilesMap {
        let _ = ScrollPrecompileProvider::new_with_spec(spec)
            .precompiles()
            .to_owned();
        todo!()
    }
}
