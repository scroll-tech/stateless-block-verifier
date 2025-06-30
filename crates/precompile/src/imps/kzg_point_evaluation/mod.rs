//! KZG point evaluation precompile added in [`EIP-4844`](https://eips.ethereum.org/EIPS/eip-4844)

#[cfg(not(any(feature = "c-kzg", feature = "kzg-rs")))]
compile_error!(
    "KZG point evaluation precompile requires either the `c-kzg` or `kzg-rs` feature to be enabled."
);

use sbv_primitives::types::revm::precompile::{PrecompileWithAddress, kzg_point_evaluation};

pub use kzg_point_evaluation::{
    ADDRESS, GAS_COST, RETURN_VALUE, VERSIONED_HASH_VERSION_KZG, as_array, kzg_to_versioned_hash,
};

#[cfg(not(feature = "openvm-kzg"))]
pub use kzg_point_evaluation::{as_bytes32, as_bytes32, run, verify_kzg_proof};

#[cfg(feature = "openvm-kzg")]
mod openvm;
#[cfg(feature = "openvm-kzg")]
pub use openvm::{as_bytes32, as_bytes48, run, verify_kzg_proof};

/// KZG point evaluation precompile, containing address and function to run.
pub const POINT_EVALUATION: PrecompileWithAddress = PrecompileWithAddress(ADDRESS, run);
