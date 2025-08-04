use crate::{Address, U8, U256};

/// An unsigned EIP-7702 authorization.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SignedAuthorization {
    /// Inner authorization.
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

impl From<&crate::eips::eip7702::Authorization> for Authorization {
    fn from(auth: &crate::eips::eip7702::Authorization) -> Self {
        Self {
            chain_id: auth.chain_id,
            address: auth.address,
            nonce: auth.nonce,
        }
    }
}

impl From<&Authorization> for crate::eips::eip7702::Authorization {
    fn from(auth: &Authorization) -> Self {
        Self {
            chain_id: auth.chain_id,
            address: auth.address,
            nonce: auth.nonce,
        }
    }
}

impl From<Authorization> for crate::eips::eip7702::Authorization {
    fn from(auth: Authorization) -> Self {
        (&auth).into()
    }
}

impl From<&crate::eips::eip7702::SignedAuthorization> for SignedAuthorization {
    fn from(auth: &crate::eips::eip7702::SignedAuthorization) -> Self {
        Self {
            inner: Authorization {
                chain_id: auth.chain_id,
                address: auth.address,
                nonce: auth.nonce,
            },
            y_parity: U8::from(auth.y_parity()),
            r: auth.r(),
            s: auth.s(),
        }
    }
}

impl From<&SignedAuthorization> for crate::eips::eip7702::SignedAuthorization {
    fn from(auth: &SignedAuthorization) -> Self {
        crate::eips::eip7702::SignedAuthorization::new_unchecked(
            (&auth.inner).into(),
            auth.y_parity.to(),
            auth.r,
            auth.s,
        )
    }
}

impl From<SignedAuthorization> for crate::eips::eip7702::SignedAuthorization {
    fn from(auth: SignedAuthorization) -> Self {
        (&auth).into()
    }
}
