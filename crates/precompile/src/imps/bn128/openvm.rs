// Copied from https://github.com/axiom-crypto/revm/blob/v75-openvm/crates/precompile/src/bn128.rs under MIT License
use sbv_primitives::revm::precompile::PrecompileError;
use std::vec::Vec;

use openvm_ecc_guest::{
    AffinePoint,
    algebra::IntMod,
    weierstrass::{IntrinsicCurve, WeierstrassPoint},
};
use openvm_pairing::{
    PairingCheck,
    bn254::{Bn254, Fp, Fp2, G1Affine, G2Affine, Scalar},
};

/// FQ_LEN specifies the number of bytes needed to represent an
/// Fq element. This is an element in the base field of BN254.
///
/// Note: The base field is used to define G1 and G2 elements.
const FQ_LEN: usize = 32;

/// SCALAR_LEN specifies the number of bytes needed to represent an Fr element.
/// This is an element in the scalar field of BN254.
const SCALAR_LEN: usize = 32;

/// FQ2_LEN specifies the number of bytes needed to represent an
/// Fq^2 element.
///
/// Note: This is the quadratic extension of Fq, and by definition
/// means we need 2 Fq elements.
const FQ2_LEN: usize = 2 * FQ_LEN;

/// G1_LEN specifies the number of bytes needed to represent a G1 element.
///
/// Note: A G1 element contains 2 Fq elements.
const G1_LEN: usize = 2 * FQ_LEN;

#[inline]
fn read_fq(input: &[u8]) -> Result<Fp, PrecompileError> {
    if input.len() < FQ_LEN {
        Err(PrecompileError::Bn128FieldPointNotAMember)
    } else {
        Fp::from_be_bytes(&input[..32]).ok_or(PrecompileError::Bn128FieldPointNotAMember)
    }
}

/// Reads a Fq2 (quadratic extension field element) from the input slice.
///
/// Parses two consecutive Fq field elements as the real and imaginary parts
/// of an Fq2 element.
/// The second component is parsed before the first, ie if a we represent an
/// element in Fq2 as (x,y) -- `y` is parsed before `x`
///
/// # Panics
///
/// Panics if the input is not at least 64 bytes long.
#[inline]
fn read_fq2(input: &[u8]) -> Result<Fp2, PrecompileError> {
    let y = read_fq(&input[..FQ_LEN])?;
    let x = read_fq(&input[FQ_LEN..2 * FQ_LEN])?;
    Ok(Fp2::new(x, y))
}

#[inline]
fn new_g1_affine_point(px: Fp, py: Fp) -> Result<G1Affine, PrecompileError> {
    G1Affine::from_xy(px, py).ok_or(PrecompileError::Bn128AffineGFailedToCreate)
}

/// Reads a G1 point from the input slice.
///
/// Parses a G1 point from a byte slice by reading two consecutive field elements
/// representing the x and y coordinates.
///
/// # Panics
///
/// Panics if the input is not at least 64 bytes long.
#[inline]
pub(super) fn read_g1_point(input: &[u8]) -> Result<G1Affine, PrecompileError> {
    let px = read_fq(&input[0..FQ_LEN])?;
    let py = read_fq(&input[FQ_LEN..2 * FQ_LEN])?;
    new_g1_affine_point(px, py)
}

/// Encodes a G1 point into a byte array.
///
/// Converts a G1 point in Jacobian coordinates to affine coordinates and
/// serializes the x and y coordinates as big-endian byte arrays.
///
/// Note: If the point is the point at infinity, this function returns
/// all zeroes.
#[inline]
pub(super) fn encode_g1_point(point: G1Affine) -> [u8; G1_LEN] {
    let mut output = [0u8; G1_LEN];

    // manually reverse to avoid allocation
    let x_bytes: &[u8] = point.x().as_le_bytes();
    let y_bytes: &[u8] = point.y().as_le_bytes();
    for i in 0..FQ_LEN {
        output[i] = x_bytes[FQ_LEN - 1 - i];
        output[i + FQ_LEN] = y_bytes[FQ_LEN - 1 - i];
    }
    output
}

/// Reads a G2 point from the input slice.
///
/// Parses a G2 point from a byte slice by reading four consecutive Fq field elements
/// representing the two Fq2 coordinates (x and y) of the G2 point.
///
/// # Panics
///
/// Panics if the input is not at least 128 bytes long.
#[inline]
pub(super) fn read_g2_point(input: &[u8]) -> Result<G2Affine, PrecompileError> {
    let ba = read_fq2(&input[0..FQ2_LEN])?;
    let bb = read_fq2(&input[FQ2_LEN..2 * FQ2_LEN])?;

    G2Affine::from_xy(ba, bb).ok_or(PrecompileError::Bn128AffineGFailedToCreate)
}

/// Reads a scalar from the input slice
///
/// Note: The scalar does not need to be canonical.
///
/// # Panics
///
/// If `input.len()` is not equal to [`SCALAR_LEN`].
#[inline]
pub(super) fn read_scalar(input: &[u8]) -> Scalar {
    assert_eq!(
        input.len(),
        SCALAR_LEN,
        "unexpected scalar length. got {}, expected {SCALAR_LEN}",
        input.len()
    );
    Scalar::from_be_bytes_unchecked(input)
}

/// Performs point addition on two G1 points.
#[inline]
pub(super) fn g1_point_add(p1: G1Affine, p2: G1Affine) -> G1Affine {
    p1 + p2
}

/// Performs a G1 scalar multiplication.
#[inline]
pub(super) fn g1_point_mul(p: G1Affine, fr: Scalar) -> G1Affine {
    Bn254::msm(&[fr], &[p])
}

/// pairing_check performs a pairing check on a list of G1 and G2 point pairs and
/// returns true if the result is equal to the identity element.
///
/// Note: If the input is empty, this function returns true.
/// This is different to EIP2537 which disallows the empty input.
#[inline]
pub(super) fn pairing_check(pairs: &[(G1Affine, G2Affine)]) -> bool {
    if pairs.is_empty() {
        return true;
    }
    let (g1_points, g2_points): (Vec<_>, Vec<_>) = pairs
        .iter()
        .cloned()
        .map(|(g1, g2)| {
            let (g1_x, g1_y) = g1.into_coords();
            let g1 = AffinePoint::new(g1_x, g1_y);

            let (g2_x, g2_y) = g2.into_coords();
            let g2 = AffinePoint::new(g2_x, g2_y);
            (g1, g2)
        })
        .unzip();

    Bn254::pairing_check(&g1_points, &g2_points).is_ok()
}
