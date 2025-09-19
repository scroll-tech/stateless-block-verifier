/// Verify KZG proof with openvm precompile.
#[inline]
pub fn verify_kzg_proof(
    commitment: &[u8; 48],
    z: &[u8; 32],
    y: &[u8; 32],
    proof: &[u8; 48],
) -> bool {
    let commitment = as_bytes48(commitment);
    let z = as_bytes32(z);
    let y = as_bytes32(y);
    let proof = as_bytes48(proof);

    let env = openvm_kzg::EnvKzgSettings::default();
    let kzg_settings = env.get();
    openvm_kzg::KzgProof::verify_kzg_proof(commitment, z, y, proof, kzg_settings).unwrap_or(false)
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
