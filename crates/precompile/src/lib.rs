//! sbv precompiles provider
#![cfg_attr(docsrs, feature(doc_cfg))]
#[cfg(any(
    feature = "openvm-bn128",
    feature = "openvm-kzg",
    feature = "openvm-secp256k1",
))]
use sbv_primitives::types::revm::precompile::PrecompileError;
use sbv_primitives::types::revm::precompile::{Crypto as CryptoInterface, install_crypto};

#[cfg(feature = "openvm-bn128")]
mod bn128;
#[cfg(feature = "openvm-kzg")]
mod kzg_point_evaluation;
#[cfg(feature = "openvm-secp256k1")]
mod secp256k1;

/// revm precompile crypto operations provider
#[derive(Debug)]
pub struct Crypto;

impl Crypto {
    /// Install this as the global crypto provider.
    ///
    /// # Panics
    ///
    /// Panics if a crypto provider has already been installed.
    pub fn install() {
        assert!(install_crypto(Self));
    }
}

impl CryptoInterface for Crypto {
    #[cfg(feature = "openvm-sha256")]
    #[inline]
    fn sha256(&self, input: &[u8]) -> [u8; 32] {
        openvm_sha2::sha256(input)
    }

    #[cfg(feature = "openvm-bn128")]
    #[inline]
    fn bn254_g1_add(&self, p1: &[u8], p2: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let p1 = bn128::read_g1_point(p1)?;
        let p2 = bn128::read_g1_point(p2)?;
        let result = bn128::g1_point_add(p1, p2);
        Ok(bn128::encode_g1_point(result))
    }

    #[cfg(feature = "openvm-bn128")]
    #[inline]
    fn bn254_g1_mul(&self, point: &[u8], scalar: &[u8]) -> Result<[u8; 64], PrecompileError> {
        let p = bn128::read_g1_point(point)?;
        let fr = bn128::read_scalar(scalar);
        let result = bn128::g1_point_mul(p, fr);
        Ok(bn128::encode_g1_point(result))
    }

    #[cfg(feature = "openvm-bn128")]
    #[inline]
    fn bn254_pairing_check(&self, pairs: &[(&[u8], &[u8])]) -> Result<bool, PrecompileError> {
        bn128::pairing_check(pairs)
    }

    #[cfg(feature = "openvm-secp256k1")]
    #[inline]
    fn secp256k1_ecrecover(
        &self,
        sig: &[u8; 64],
        recid: u8,
        msg: &[u8; 32],
    ) -> Result<[u8; 32], PrecompileError> {
        secp256k1::ecrecover(sig, recid, msg)
            .ok()
            .ok_or_else(|| PrecompileError::other("ecrecover failed"))
    }

    #[cfg(feature = "openvm-kzg")]
    #[inline]
    fn verify_kzg_proof(
        &self,
        z: &[u8; 32],
        y: &[u8; 32],
        commitment: &[u8; 48],
        proof: &[u8; 48],
    ) -> Result<(), PrecompileError> {
        if !kzg_point_evaluation::verify_kzg_proof(commitment, z, y, proof) {
            return Err(PrecompileError::BlobVerifyKzgProofFailed);
        }
        Ok(())
    }
}
