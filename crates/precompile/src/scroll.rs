use super::PrecompileProvider;
use sbv_primitives::types::evm::precompiles::PrecompilesMap;
use sbv_primitives::types::revm::{ScrollPrecompileProvider, SpecId};
impl PrecompileProvider {
    #[cfg(not(target_os = "zkvm"))]
    pub fn new_with_spec(spec_id: SpecId) -> PrecompilesMap {
        PrecompilesMap::from_static(ScrollPrecompileProvider::new_with_spec(spec_id).precompiles())
    }

    #[cfg(target_os = "zkvm")]
    pub fn new_with_spec(spec_id: SpecId) -> PrecompilesMap {
        use crate::openvm::{bn128, sha256};

        let mut precompiles = ScrollPrecompileProvider::new_with_spec(spec_id)
            .precompiles()
            .to_owned();

        precompiles.extend([bn128::add::ISTANBUL, bn128::mul::ISTANBUL]);
        if spec_id.is_enabled_in(SpecId::BERNOULLI) {
            precompiles.extend([bn128::pair::BERNOULLI, sha256::BERNOULLI]);
        }
        if spec_id.is_enabled_in(SpecId::FEYNMAN) {
            precompiles.extend([bn128::pair::FEYNMAN]);
        }

        PrecompilesMap::new(precompiles.into())
    }
}
