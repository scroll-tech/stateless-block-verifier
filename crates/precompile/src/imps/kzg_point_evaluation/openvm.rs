use sbv_primitives::types::revm::precompile::{
    PrecompileError, PrecompileOutput, PrecompileResult, kzg_point_evaluation,
};

/// Verify KZG proof with openvm precompile.
#[inline]
pub fn verify_kzg_proof(
    commitment: &openvm_kzg::Bytes48,
    z: &openvm_kzg::Bytes32,
    y: &openvm_kzg::Bytes32,
    proof: &openvm_kzg::Bytes48,
) -> bool {
    let env = openvm_kzg::EnvKzgSettings::default();
    let kzg_settings = env.get();
    openvm_kzg::KzgProof::verify_kzg_proof(commitment, z, y, proof, kzg_settings).unwrap_or(false)
}

/// Run kzg point evaluation precompile.
pub fn run(input: &[u8], gas_limit: u64) -> PrecompileResult {
    use kzg_point_evaluation::{GAS_COST, RETURN_VALUE, kzg_to_versioned_hash};

    if gas_limit < GAS_COST {
        return Err(PrecompileError::OutOfGas);
    }

    // Verify input length.
    if input.len() != 192 {
        return Err(PrecompileError::BlobInvalidInputLength);
    }

    // Verify commitment matches versioned_hash
    let versioned_hash = &input[..32];
    let commitment = &input[96..144];
    if kzg_to_versioned_hash(commitment) != versioned_hash {
        return Err(PrecompileError::BlobMismatchedVersion);
    }

    // Verify KZG proof with z and y in big endian format
    let commitment = as_bytes48(commitment);
    let z = as_bytes32(&input[32..64]);
    let y = as_bytes32(&input[64..96]);
    let proof = as_bytes48(&input[144..192]);
    if !verify_kzg_proof(commitment, z, y, proof) {
        return Err(PrecompileError::BlobVerifyKzgProofFailed);
    }

    // Return FIELD_ELEMENTS_PER_BLOB and BLS_MODULUS as padded 32 byte big endian values
    Ok(PrecompileOutput::new(GAS_COST, RETURN_VALUE.into()))
}

/// Convert a slice to an array of a specific size.
#[inline]
#[track_caller]
fn as_array<const N: usize>(bytes: &[u8]) -> &[u8; N] {
    bytes.try_into().expect("slice with incorrect length")
}

/// Convert a slice to a 32 byte big endian array.
#[inline]
#[track_caller]
fn as_bytes32(bytes: &[u8]) -> &openvm_kzg::Bytes32 {
    // SAFETY: `#[repr(C)] Bytes32([u8; 32])`
    unsafe { &*as_array::<32>(bytes).as_ptr().cast() }
}

/// Convert a slice to a 48 byte big endian array.
#[inline]
#[track_caller]
fn as_bytes48(bytes: &[u8]) -> &openvm_kzg::Bytes48 {
    // SAFETY: `#[repr(C)] Bytes48([u8; 48])`
    unsafe { &*as_array::<48>(bytes).as_ptr().cast() }
}
