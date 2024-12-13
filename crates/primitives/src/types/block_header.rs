use alloy_primitives::{Address, BlockHash, B256, U256};

/// Block header representation.
#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct BlockHeader {
    /// The 160-bit address to which all fees collected from the successful mining of this block
    /// be transferred; formally Hc.
    #[rkyv(attr(
        doc = "The 160-bit address to which all fees collected from the successful mining of this block be transferred; formally Hc."
    ))]
    #[serde(rename = "miner", alias = "beneficiary")]
    pub beneficiary: Address,
    /// Hash of the block
    #[rkyv(attr(doc = "Hash of the block"))]
    pub hash: BlockHash,
    /// State root hash
    #[rkyv(attr(doc = "State root hash"))]
    pub state_root: B256,
    /// Difficulty
    #[rkyv(attr(doc = "Difficulty"))]
    pub difficulty: U256,
    /// Block number
    #[rkyv(attr(doc = "Block number"))]
    #[serde(with = "alloy_serde::quantity")]
    pub number: u64,
    /// Gas Limit
    #[rkyv(attr(doc = "Gas Limit"))]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_limit: u64,
    /// Gas Used
    #[rkyv(attr(doc = "Gas Used"))]
    #[serde(with = "alloy_serde::quantity")]
    pub gas_used: u64,
    /// Timestamp
    #[rkyv(attr(doc = "Timestamp"))]
    #[serde(with = "alloy_serde::quantity")]
    pub timestamp: u64,
    /// A 256-bit hash which, combined with the
    /// nonce, proves that a sufficient amount of computation has been carried out on this block;
    /// formally Hm.
    #[rkyv(attr(doc = r#"A 256-bit hash which, combined with the
nonce, proves that a sufficient amount of computation has been carried out on this block;
formally Hm."#))]
    pub prevrandao: B256,
    /// Base fee per unit of gas (if past London)
    #[rkyv(attr(doc = "Base fee per unit of gas (if past London)"))]
    #[serde(
        default,
        with = "alloy_serde::quantity::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub base_fee_per_gas: Option<u64>,
    /// Withdrawals root hash added by EIP-4895 and is ignored in legacy headers.
    #[rkyv(attr(
        doc = "Withdrawals root hash added by EIP-4895 and is ignored in legacy headers."
    ))]
    pub withdrawals_root: B256,
    /// Blob gas used
    #[rkyv(attr(doc = "Blob gas used"))]
    #[serde(
        default,
        with = "alloy_serde::quantity::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub blob_gas_used: Option<u64>,
    /// Excess blob gas
    #[rkyv(attr(doc = "Excess blob gas"))]
    #[serde(
        default,
        with = "alloy_serde::quantity::opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub excess_blob_gas: Option<u64>,
}

impl From<alloy_rpc_types_eth::Header> for BlockHeader {
    fn from(header: alloy_rpc_types_eth::Header) -> Self {
        Self {
            beneficiary: header.beneficiary,
            hash: header.hash,
            state_root: header.state_root,
            difficulty: header.difficulty,
            number: header.number,
            gas_limit: header.gas_limit,
            gas_used: header.gas_used,
            timestamp: header.timestamp,
            prevrandao: header.mix_hash,
            base_fee_per_gas: header.base_fee_per_gas,
            withdrawals_root: header
                .withdrawals_root
                .expect("legacy headers have no withdrawals"),
            blob_gas_used: header.blob_gas_used,
            excess_blob_gas: header.excess_blob_gas,
        }
    }
}

impl crate::BlockHeader for BlockHeader {
    fn beneficiary(&self) -> Address {
        self.beneficiary
    }
    fn hash(&self) -> BlockHash {
        self.hash
    }

    fn state_root(&self) -> B256 {
        self.state_root
    }

    fn difficulty(&self) -> U256 {
        self.difficulty
    }

    fn number(&self) -> u64 {
        self.number
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn gas_used(&self) -> u64 {
        self.gas_used
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn prevrandao(&self) -> B256 {
        self.prevrandao
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.base_fee_per_gas
    }

    fn withdraw_root(&self) -> B256 {
        self.withdrawals_root
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.blob_gas_used
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.excess_blob_gas
    }
}

impl crate::BlockHeader for ArchivedBlockHeader {
    fn beneficiary(&self) -> Address {
        self.beneficiary.into()
    }
    fn hash(&self) -> BlockHash {
        B256::from(self.hash)
    }

    fn state_root(&self) -> B256 {
        B256::from(self.state_root)
    }

    fn difficulty(&self) -> U256 {
        self.difficulty.into()
    }

    fn number(&self) -> u64 {
        u64::from(self.number)
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit.to_native()
    }

    fn gas_used(&self) -> u64 {
        self.gas_used.to_native()
    }

    fn timestamp(&self) -> u64 {
        self.timestamp.to_native()
    }

    fn prevrandao(&self) -> B256 {
        B256::from(self.prevrandao)
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.base_fee_per_gas.as_ref().map(|x| x.to_native())
    }

    fn withdraw_root(&self) -> B256 {
        B256::from(self.withdrawals_root)
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.blob_gas_used.as_ref().map(|x| x.to_native())
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.excess_blob_gas.as_ref().map(|x| x.to_native())
    }
}
