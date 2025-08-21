use crate::U256;

/// An Ethereum ECDSA signature.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[derive(serde::Serialize, serde::Deserialize)]
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

impl From<crate::Signature> for Signature {
    fn from(sig: crate::Signature) -> Self {
        Self {
            r: sig.r(),
            s: sig.s(),
            y_parity: sig.v(),
        }
    }
}

impl From<Signature> for crate::Signature {
    fn from(sig: Signature) -> Self {
        crate::Signature::new(sig.r, sig.s, sig.y_parity)
    }
}
