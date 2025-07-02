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
        use crate::imps::{bn128, secp256k1, sha256};

        let mut precompiles = ScrollPrecompileProvider::new_with_spec(spec)
            .precompiles()
            .to_owned();

        #[cfg(feature = "openvm-secp256k1")]
        precompiles.extend([secp256k1::ECRECOVER]);

        #[cfg(feature = "openvm-bn128")]
        {
            precompiles.extend([bn128::add::ISTANBUL, bn128::mul::ISTANBUL]);
            if spec.is_enabled_in(SpecId::BERNOULLI) {
                precompiles.extend([bn128::pair::BERNOULLI, sha256::BERNOULLI]);
            }
            if spec.is_enabled_in(SpecId::FEYNMAN) {
                precompiles.extend([bn128::pair::FEYNMAN]);
            }
        }

        PrecompilesMap::new(std::borrow::Cow::Owned(precompiles))
    }
}
