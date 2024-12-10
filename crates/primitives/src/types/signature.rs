use alloy_primitives::U256;

/// Container type for all signature fields in RPC
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct Signature {
    /// The R field of the signature; the point on the curve.
    #[rkyv(attr(doc = ""))]
    pub r: U256,
    /// The S field of the signature; the point on the curve.
    #[rkyv(attr(doc = ""))]
    pub s: U256,
    /// For EIP-155, EIP-2930 and Blob transactions this is set to the parity (0 for even, 1 for
    /// odd) of the y-value of the secp256k1 signature.
    ///
    /// For legacy transactions, this is the recovery id
    ///
    /// See also <https://ethereum.github.io/execution-apis/api-documentation/> and <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_gettransactionbyhash>
    #[rkyv(attr(doc = ""))]
    pub v: u64,
}

impl From<alloy_rpc_types_eth::Signature> for Signature {
    fn from(sig: alloy_rpc_types_eth::Signature) -> Self {
        Self {
            r: sig.r,
            s: sig.s,
            v: sig.v.to(),
        }
    }
}

impl TryFrom<Signature> for alloy_primitives::Signature {
    type Error = alloy_primitives::SignatureError;

    fn try_from(sig: Signature) -> Result<Self, Self::Error> {
        Self::from_rs_and_parity(sig.r, sig.s, sig.v)
    }
}

impl TryFrom<&ArchivedSignature> for alloy_primitives::Signature {
    type Error = alloy_primitives::SignatureError;

    fn try_from(sig: &ArchivedSignature) -> Result<Self, Self::Error> {
        Self::from_rs_and_parity(sig.r.into(), sig.s.into(), sig.v.to_native())
    }
}
