use crate::{Address, U8, U256};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Authorization {
    /// The chain ID of the authorization.
    pub chain_id: U256,
    /// The address of the authorization.
    pub address: Address,
    /// The nonce for the authorization.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
}

/// A signed EIP-7702 authorization.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedAuthorization {
    /// Inner authorization.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: Authorization,
    /// Signature parity value. We allow any [`U8`] here, however, the only valid values are `0`
    /// and `1` and anything else will result in error during recovery.
    #[cfg_attr(feature = "serde", serde(rename = "yParity", alias = "v"))]
    pub y_parity: U8,
    /// Signature `r` value.
    pub r: U256,
    /// Signature `s` value.
    pub s: U256,
}
