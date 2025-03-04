use alloy_primitives::{Address, U8, U256};

#[derive(
    Debug,
    Clone,
    Hash,
    Eq,
    PartialEq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct Authorization {
    /// The chain ID of the authorization.
    pub chain_id: U256,
    /// The address of the authorization.
    pub address: Address,
    /// The nonce for the authorization.
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
}

/// A signed EIP-7702 authorization.
#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct SignedAuthorization {
    /// Inner authorization.
    inner: Authorization,
    /// Signature parity value. We allow any [`U8`] here, however, the only valid values are `0`
    /// and `1` and anything else will result in error during recovery.
    #[serde(rename = "yParity", alias = "v")]
    y_parity: U8,
    /// Signature `r` value.
    r: U256,
    /// Signature `s` value.
    s: U256,
}

impl From<&alloy_eips::eip7702::Authorization> for Authorization {
    fn from(auth: &alloy_eips::eip7702::Authorization) -> Self {
        Self {
            chain_id: auth.chain_id,
            address: auth.address,
            nonce: auth.nonce,
        }
    }
}

impl From<Authorization> for alloy_eips::eip7702::Authorization {
    fn from(auth: Authorization) -> Self {
        Self {
            chain_id: auth.chain_id,
            address: auth.address,
            nonce: auth.nonce,
        }
    }
}

impl From<&alloy_eips::eip7702::SignedAuthorization> for SignedAuthorization {
    fn from(auth: &alloy_eips::eip7702::SignedAuthorization) -> Self {
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

impl From<SignedAuthorization> for alloy_eips::eip7702::SignedAuthorization {
    fn from(auth: SignedAuthorization) -> Self {
        alloy_eips::eip7702::SignedAuthorization::new_unchecked(
            auth.inner.into(),
            auth.y_parity.to(),
            auth.r,
            auth.s,
        )
    }
}

impl From<&ArchivedSignedAuthorization> for alloy_eips::eip7702::SignedAuthorization {
    fn from(auth: &ArchivedSignedAuthorization) -> Self {
        let y_parity: U8 = From::from(&auth.y_parity);
        alloy_eips::eip7702::SignedAuthorization::new_unchecked(
            alloy_eips::eip7702::Authorization {
                chain_id: auth.inner.chain_id.into(),
                address: Address::from(auth.inner.address),
                nonce: auth.inner.nonce.to_native(),
            },
            y_parity.to(),
            auth.r.into(),
            auth.s.into(),
        )
    }
}
