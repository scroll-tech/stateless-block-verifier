//! `ecrecover` precompile.
use sbv_primitives::types::revm::precompile::{self, PrecompileWithAddress, secp256k1};

#[cfg(feature = "openvm-secp256k1")]
mod openvm;

#[cfg(feature = "openvm-secp256k1")]
pub use openvm::ecrecover;
#[cfg(not(feature = "openvm-secp256k1"))]
pub use secp256k1::{ec_recover_run, ecrecover};

/// `ecrecover` precompile, containing address and function to run.
pub const ECRECOVER: PrecompileWithAddress =
    PrecompileWithAddress(secp256k1::ECRECOVER.0, ec_recover_run);

// Copied from https://github.com/bluealloy/revm/blob/v75/crates/precompile/src/secp256k1.rs Under MIT License

/// `ecrecover` precompile function with openvm precompiles.
#[cfg(feature = "openvm-secp256k1")]
pub fn ec_recover_run(input: &[u8], gas_limit: u64) -> precompile::PrecompileResult {
    use sbv_primitives::{
        B256, Bytes,
        alloy_primitives::B512,
        types::revm::precompile::{PrecompileError, PrecompileOutput, utilities::right_pad},
    };

    const ECRECOVER_BASE: u64 = 3_000;

    if ECRECOVER_BASE > gas_limit {
        return Err(PrecompileError::OutOfGas);
    }

    let input = right_pad::<128>(input);

    // `v` must be a 32-byte big-endian integer equal to 27 or 28.
    if !(input[32..63].iter().all(|&b| b == 0) && matches!(input[63], 27 | 28)) {
        return Ok(PrecompileOutput::new(ECRECOVER_BASE, Bytes::new()));
    }

    let msg = <&B256>::try_from(&input[0..32]).unwrap();
    let recid = input[63] - 27;
    let sig = <&B512>::try_from(&input[64..128]).unwrap();

    let res = ecrecover(sig, recid, msg);

    let out = res.map(|o| o.to_vec().into()).unwrap_or_default();
    Ok(PrecompileOutput::new(ECRECOVER_BASE, out))
}
