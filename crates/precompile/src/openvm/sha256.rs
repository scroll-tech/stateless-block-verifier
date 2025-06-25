use sbv_primitives::types::revm::precompile::{
    PrecompileError, PrecompileOutput, PrecompileResult, PrecompileWithAddress,
    calc_linear_cost_u32, hash,
};

/// The bernoulli SHA256 precompile implementation with address.
pub const BERNOULLI: PrecompileWithAddress = PrecompileWithAddress(hash::SHA256.0, run);

fn run(input: &[u8], gas_limit: u64) -> PrecompileResult {
    let cost = calc_linear_cost_u32(input.len(), 60, 12);
    if cost > gas_limit {
        Err(PrecompileError::OutOfGas)
    } else {
        let output = openvm_sha2::sha256(input);
        Ok(PrecompileOutput::new(cost, output.to_vec().into()))
    }
}
