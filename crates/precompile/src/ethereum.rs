use super::PrecompileProvider;
use sbv_primitives::types::{
    evm::precompiles::PrecompilesMap,
    revm::precompile::{PrecompileSpecId, Precompiles},
};

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
    pub fn with_spec(spec: PrecompileSpecId) -> PrecompilesMap {
        use crate::imps::{bn128, kzg_point_evaluation, secp256k1, sha256};

        let mut precompiles = Precompiles::new(spec).to_owned();

        precompiles.extend([secp256k1::ECRECOVER, sha256::HOMESTEAD]);

        if spec >= PrecompileSpecId::ISTANBUL {
            precompiles.extend([bn128::add::ISTANBUL, bn128::mul::ISTANBUL]);
        } else if spec >= PrecompileSpecId::BYZANTIUM {
            precompiles.extend([bn128::add::BYZANTIUM, bn128::mul::BYZANTIUM]);
        }

        if spec >= PrecompileSpecId::CANCUN {
            precompiles.extend([kzg_point_evaluation::POINT_EVALUATION]);
        }

        PrecompilesMap::new(std::borrow::Cow::Owned(precompiles))
    }
}
