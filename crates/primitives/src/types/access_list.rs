use crate::{Address, B256};

/// A list of addresses and storage keys that the transaction plans to access.
/// Accesses outside the list are possible, but become more expensive.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccessListItem {
    /// Account addresses that would be loaded at the start of execution
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Account addresses that would be loaded at the start of execution"))
    )]
    pub address: Address,
    /// Keys of storage that would be loaded at the start of execution
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Keys of storage that would be loaded at the start of execution"))
    )]
    pub storage_keys: Vec<B256>,
}

/// AccessList as defined in EIP-2930
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccessList(pub Vec<AccessListItem>);
