use alloy_primitives::Address;

/// Withdrawal represents a validator withdrawal from the consensus layer.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub struct Withdrawal {
    /// Monotonically increasing identifier issued by consensus layer.
    #[rkyv(attr(doc = "Monotonically increasing identifier issued by consensus layer."))]
    #[serde(with = "alloy_serde::quantity")]
    pub index: u64,
    /// Index of validator associated with withdrawal.
    #[rkyv(attr(doc = "Index of validator associated with withdrawal."))]
    #[serde(with = "alloy_serde::quantity", rename = "validatorIndex")]
    pub validator_index: u64,
    /// Target address for withdrawn ether.
    #[rkyv(attr(doc = "Target address for withdrawn ether."))]
    pub address: Address,
    /// Value of the withdrawal in gwei.
    #[rkyv(attr(doc = "Value of the withdrawal in gwei."))]
    #[serde(with = "alloy_serde::quantity")]
    pub amount: u64,
}

impl From<&alloy_eips::eip4895::Withdrawal> for Withdrawal {
    fn from(withdrawal: &alloy_eips::eip4895::Withdrawal) -> Self {
        Self {
            index: withdrawal.index,
            validator_index: withdrawal.validator_index,
            address: withdrawal.address,
            amount: withdrawal.amount,
        }
    }
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
