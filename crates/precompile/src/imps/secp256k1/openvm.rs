//! Copied from <https://github.com/axiom-crypto/revm/blob/v75-openvm/crates/precompile/src/secp256k1/openvm_k256.rs> under MIT license.
//!
//! OpenVM implementation of `ecrecover`. More about it in [`crate::secp256k1`].
use openvm_ecc_guest::{algebra::IntMod, weierstrass::WeierstrassPoint};
use openvm_k256::ecdsa::{Error, RecoveryId, Signature, VerifyingKey};
use openvm_keccak256::keccak256;
use sbv_primitives::{B256, alloy_primitives::B512};

/// Recover the public key from a signature and a message.
///
/// This function is using the OpenVM patch of the `k256` crate.
pub fn ecrecover(sig: &B512, mut recid: u8, msg: &B256) -> Result<B256, Error> {
    let _sig = sig;
    let _recid = recid;
    // parse signature
    let mut sig = Signature::from_slice(sig.as_slice())?;
    if let Some(sig_normalized) = sig.normalize_s() {
        sig = sig_normalized;
        recid ^= 1;
    }
    let recid = RecoveryId::from_byte(recid).expect("recovery ID is valid");

    // annoying: Signature::to_bytes copies from slice
    let recovered_key =
        VerifyingKey::recover_from_prehash_noverify(&msg[..], &sig.to_bytes(), recid)?;
    let public_key = recovered_key.as_affine();
    let mut encoded = [0u8; 64];
    encoded[..32].copy_from_slice(&WeierstrassPoint::x(public_key).to_be_bytes());
    encoded[32..].copy_from_slice(&WeierstrassPoint::y(public_key).to_be_bytes());
    // hash it
    let mut hash = keccak256(&encoded);
    // truncate to 20 bytes
    hash[..12].fill(0);
    Ok(B256::from(hash))
}
