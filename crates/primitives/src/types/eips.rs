#[cfg(feature = "rkyv")]
use crate::types::{
    access_list::{ArchivedAccessList, ArchivedAccessListItem},
    auth_list::ArchivedSignedAuthorization,
    withdrawal::ArchivedWithdrawal,
};
use crate::{
    U8,
    types::{
        access_list::{AccessList, AccessListItem},
        auth_list::{Authorization, SignedAuthorization},
        withdrawal::Withdrawal,
    },
};

pub use alloy_eips::*;

impl From<&eip2930::AccessListItem> for AccessListItem {
    fn from(item: &eip2930::AccessListItem) -> Self {
        Self {
            address: item.address,
            storage_keys: item.storage_keys.clone(),
        }
    }
}

impl From<AccessListItem> for eip2930::AccessListItem {
    fn from(item: AccessListItem) -> Self {
        Self {
            address: item.address,
            storage_keys: item.storage_keys,
        }
    }
}

impl From<&eip2930::AccessList> for AccessList {
    fn from(list: &eip2930::AccessList) -> Self {
        Self(list.0.iter().map(Into::into).collect())
    }
}

impl From<AccessList> for eip2930::AccessList {
    fn from(list: AccessList) -> Self {
        Self(list.0.into_iter().map(Into::into).collect())
    }
}

#[cfg(feature = "rkyv")]
impl From<&ArchivedAccessListItem> for eip2930::AccessListItem {
    fn from(item: &ArchivedAccessListItem) -> Self {
        Self {
            address: crate::Address::from(item.address),
            storage_keys: item
                .storage_keys
                .iter()
                .map(|key| crate::B256::from(*key))
                .collect(),
        }
    }
}

#[cfg(feature = "rkyv")]
impl From<&ArchivedAccessList> for eip2930::AccessList {
    fn from(list: &ArchivedAccessList) -> Self {
        Self(
            list.0
                .iter()
                .map(alloy_eips::eip2930::AccessListItem::from)
                .collect(),
        )
    }
}

impl From<&eip7702::Authorization> for Authorization {
    fn from(auth: &eip7702::Authorization) -> Self {
        Self {
            chain_id: auth.chain_id,
            address: auth.address,
            nonce: auth.nonce,
        }
    }
}

impl From<Authorization> for eip7702::Authorization {
    fn from(auth: Authorization) -> Self {
        Self {
            chain_id: auth.chain_id,
            address: auth.address,
            nonce: auth.nonce,
        }
    }
}

impl From<&eip7702::SignedAuthorization> for SignedAuthorization {
    fn from(auth: &eip7702::SignedAuthorization) -> Self {
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

impl From<SignedAuthorization> for eip7702::SignedAuthorization {
    fn from(auth: SignedAuthorization) -> Self {
        eip7702::SignedAuthorization::new_unchecked(
            auth.inner.into(),
            auth.y_parity.to(),
            auth.r,
            auth.s,
        )
    }
}

#[cfg(feature = "rkyv")]
impl From<&ArchivedSignedAuthorization> for eip7702::SignedAuthorization {
    fn from(auth: &ArchivedSignedAuthorization) -> Self {
        let y_parity: U8 = From::from(&auth.y_parity);
        eip7702::SignedAuthorization::new_unchecked(
            eip7702::Authorization {
                chain_id: auth.inner.chain_id.into(),
                address: crate::Address::from(auth.inner.address),
                nonce: auth.inner.nonce.to_native(),
            },
            y_parity.to(),
            auth.r.into(),
            auth.s.into(),
        )
    }
}

impl From<&eip4895::Withdrawal> for Withdrawal {
    fn from(withdrawal: &eip4895::Withdrawal) -> Self {
        Self {
            index: withdrawal.index,
            validator_index: withdrawal.validator_index,
            address: withdrawal.address,
            amount: withdrawal.amount,
        }
    }
}

impl From<&Withdrawal> for eip4895::Withdrawal {
    fn from(withdrawal: &Withdrawal) -> Self {
        Self {
            index: withdrawal.index,
            validator_index: withdrawal.validator_index,
            address: withdrawal.address,
            amount: withdrawal.amount,
        }
    }
}

#[cfg(feature = "rkyv")]
impl From<&ArchivedWithdrawal> for eip4895::Withdrawal {
    fn from(withdrawal: &ArchivedWithdrawal) -> Self {
        Self {
            index: withdrawal.index.to_native(),
            validator_index: withdrawal.validator_index.to_native(),
            address: withdrawal.address.into(),
            amount: withdrawal.amount.to_native(),
        }
    }
}
