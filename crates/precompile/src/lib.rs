//! sbv precompiles provider
#[cfg(feature = "openvm")]
mod openvm;

/// A precompile provider that patches the precompiles when runs on zkVM with OpenVM enabled.
#[derive(Debug, Default, Copy, Clone)]
pub struct PrecompileProvider;

#[cfg(feature = "scroll")]
mod scroll {
    use super::PrecompileProvider;
    use sbv_primitives::types::{
        evm::{ScrollPrecompilesFactory, precompiles::PrecompilesMap},
        revm::{ScrollPrecompileProvider, SpecId},
    };

    #[cfg(not(feature = "openvm"))]
    impl ScrollPrecompilesFactory for PrecompileProvider {
        fn with_spec(spec: SpecId) -> PrecompilesMap {
            PrecompilesMap::from_static(ScrollPrecompileProvider::new_with_spec(spec).precompiles())
        }
    }

    #[cfg(feature = "openvm")]
    impl ScrollPrecompilesFactory for PrecompileProvider {
        fn with_spec(spec: SpecId) -> PrecompilesMap {
            use crate::openvm::{bn128, sha256};

            let mut precompiles = ScrollPrecompileProvider::new_with_spec(spec)
                .precompiles()
                .to_owned();

            precompiles.extend([bn128::add::ISTANBUL, bn128::mul::ISTANBUL]);
            if spec.is_enabled_in(SpecId::BERNOULLI) {
                precompiles.extend([bn128::pair::BERNOULLI, sha256::BERNOULLI]);
            }
            if spec.is_enabled_in(SpecId::FEYNMAN) {
                precompiles.extend([bn128::pair::FEYNMAN]);
            }

            PrecompilesMap::new(precompiles.into())
        }
    }
}

#[cfg(not(feature = "scroll"))]
mod ethereum {
    compile_error!("unimplemented");
}
