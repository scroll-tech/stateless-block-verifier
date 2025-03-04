use crate::{PrimitiveSignature, U256};

/// An Ethereum ECDSA signature.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Signature {
    /// The R field of the signature; the point on the curve.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The R field of the signature; the point on the curve."))
    )]
    pub r: U256,
    /// The S field of the signature; the point on the curve.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The S field of the signature; the point on the curve."))
    )]
    pub s: U256,
    /// The parity of the Y coordinate of the public key.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The parity of the Y coordinate of the public key."))
    )]
    pub y_parity: bool,
}

impl From<&PrimitiveSignature> for Signature {
    fn from(sig: &PrimitiveSignature) -> Self {
        Self {
            r: sig.r(),
            s: sig.s(),
            y_parity: sig.v(),
        }
    }
}

impl From<Signature> for PrimitiveSignature {
    fn from(sig: Signature) -> Self {
        Self::new(sig.r, sig.s, sig.y_parity)
    }
}

#[cfg(feature = "rkyv")]
impl From<&ArchivedSignature> for PrimitiveSignature {
    fn from(sig: &ArchivedSignature) -> Self {
        Self::new(sig.r.into(), sig.s.into(), sig.y_parity)
    }
}
