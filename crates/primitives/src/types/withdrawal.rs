use crate::Address;

/// Withdrawal represents a validator withdrawal from the consensus layer.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Withdrawal {
    /// Monotonically increasing identifier issued by consensus layer.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Monotonically increasing identifier issued by consensus layer."))
    )]
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub index: u64,
    /// Index of validator associated with withdrawal.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Index of validator associated with withdrawal."))
    )]
    #[cfg_attr(
        feature = "serde",
        serde(with = "alloy_serde::quantity", rename = "validatorIndex")
    )]
    pub validator_index: u64,
    /// Target address for withdrawn ether.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Target address for withdrawn ether."))
    )]
    pub address: Address,
    /// Value of the withdrawal in gwei.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Value of the withdrawal in gwei.")))]
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub amount: u64,
}

impl crate::Withdrawal for Withdrawal {
    fn index(&self) -> u64 {
        self.index
    }
    fn validator_index(&self) -> u64 {
        self.validator_index
    }
    fn address(&self) -> Address {
        self.address
    }

    fn amount(&self) -> u64 {
        self.amount
    }
}

#[cfg(feature = "rkyv")]
impl crate::Withdrawal for ArchivedWithdrawal {
    fn index(&self) -> u64 {
        self.index.to_native()
    }
    fn validator_index(&self) -> u64 {
        self.validator_index.to_native()
    }
    fn address(&self) -> Address {
        self.address.into()
    }

    fn amount(&self) -> u64 {
        self.amount.to_native()
    }
}
