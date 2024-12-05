use alloy_primitives::{Address, B256};

/// A list of addresses and storage keys that the transaction plans to access.
/// Accesses outside the list are possible, but become more expensive.
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct AccessListItem {
    /// Account addresses that would be loaded at the start of execution
    #[rkyv(attr(doc = "Account addresses that would be loaded at the start of execution"))]
    pub address: Address,
    /// Keys of storage that would be loaded at the start of execution
    #[rkyv(attr(doc = "Keys of storage that would be loaded at the start of execution"))]
    pub storage_keys: Vec<B256>,
}

/// AccessList as defined in EIP-2930
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct AccessList(pub Vec<AccessListItem>);

impl From<alloy_eips::eip2930::AccessListItem> for AccessListItem {
    fn from(item: alloy_eips::eip2930::AccessListItem) -> Self {
        Self {
            address: item.address,
            storage_keys: item.storage_keys,
        }
    }
}

impl From<AccessListItem> for alloy_eips::eip2930::AccessListItem {
    fn from(item: AccessListItem) -> Self {
        Self {
            address: item.address,
            storage_keys: item.storage_keys,
        }
    }
}

impl From<alloy_eips::eip2930::AccessList> for AccessList {
    fn from(list: alloy_eips::eip2930::AccessList) -> Self {
        Self(list.0.into_iter().map(Into::into).collect())
    }
}

impl From<AccessList> for alloy_eips::eip2930::AccessList {
    fn from(list: AccessList) -> Self {
        Self(list.0.into_iter().map(Into::into).collect())
    }
}

impl From<&ArchivedAccessListItem> for alloy_eips::eip2930::AccessListItem {
    fn from(item: &ArchivedAccessListItem) -> Self {
        Self {
            address: Address::from(item.address),
            storage_keys: item
                .storage_keys
                .iter()
                .map(|key| B256::from(*key))
                .collect(),
        }
    }
}

impl From<&ArchivedAccessList> for alloy_eips::eip2930::AccessList {
    fn from(list: &ArchivedAccessList) -> Self {
        Self(
            list.0
                .iter()
                .map(|item| alloy_eips::eip2930::AccessListItem::from(item))
                .collect(),
        )
    }
}
