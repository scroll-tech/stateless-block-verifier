//! Hash precompiles, it contains SHA-256 hash precompile
//! More details in [`sha256_run`]
use sbv_primitives::types::revm::precompile::{self, PrecompileWithAddress, hash};

#[cfg(not(feature = "openvm-sha256"))]
pub use hash::sha256_run;

/// The homestead SHA256 precompile implementation with address.
pub const HOMESTEAD: PrecompileWithAddress = PrecompileWithAddress(hash::SHA256.0, sha256_run);

/// The bernoulli SHA256 precompile implementation with address.
#[cfg(feature = "scroll")]
#[cfg_attr(docsrs, doc(cfg(feature = "scroll")))]
pub const BERNOULLI: PrecompileWithAddress = PrecompileWithAddress(hash::SHA256.0, sha256_run);

/// Computes the SHA-256 hash of the input data with openvm-sha2 precompile
///
/// See [`hash::sha256_run`] for more details.
#[cfg(feature = "openvm-sha256")]
pub fn sha256_run(input: &[u8], gas_limit: u64) -> precompile::PrecompileResult {
    use precompile::{PrecompileError, PrecompileOutput, calc_linear_cost_u32};

    let cost = calc_linear_cost_u32(input.len(), 60, 12);
    if cost > gas_limit {
        Err(PrecompileError::OutOfGas)
    } else {
        let output = openvm_sha2::sha256(input);
        Ok(PrecompileOutput::new(cost, output.to_vec().into()))
    }
}
