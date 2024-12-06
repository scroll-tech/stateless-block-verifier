use alloy_primitives::{BlockHash, B256, U256};

/// Block header representation.
#[derive(Clone, Debug, Hash, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct BlockHeader {
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
    pub number: u64,
    /// Gas Limit
    #[rkyv(attr(doc = "Gas Limit"))]
    pub gas_limit: u64,
    /// Gas Used
    #[rkyv(attr(doc = "Gas Used"))]
    pub gas_used: u64,
    /// Timestamp
    #[rkyv(attr(doc = "Timestamp"))]
    pub timestamp: u64,
    /// Mix Hash
    ///
    /// Before the merge this proves, combined with the nonce, that a sufficient amount of
    /// computation has been carried out on this block: the Proof-of-Work (PoF).
    ///
    /// After the merge this is `prevRandao`: Randomness value for the generated payload.
    ///
    /// This is an Option because it is not always set by non-ethereum networks.
    ///
    /// See also <https://eips.ethereum.org/EIPS/eip-4399>
    #[rkyv(attr(doc = r#"Mix Hash

Before the merge this proves, combined with the nonce, that a sufficient amount of
computation has been carried out on this block: the Proof-of-Work (PoF).

After the merge this is `prevRandao`: Randomness value for the generated payload.

This is an Option because it is not always set by non-ethereum networks.

See also <https://eips.ethereum.org/EIPS/eip-4399>"#))]
    pub prevrandao: Option<B256>,
    /// Base fee per unit of gas (if past London)
    #[rkyv(attr(doc = "Base fee per unit of gas (if past London)"))]
    pub base_fee_per_gas: Option<u64>,
    /// Withdrawals root hash added by EIP-4895 and is ignored in legacy headers.
    #[rkyv(attr(
        doc = "Withdrawals root hash added by EIP-4895 and is ignored in legacy headers."
    ))]
    pub withdrawals_root: B256,
}

impl From<alloy_rpc_types_eth::Header> for BlockHeader {
    fn from(header: alloy_rpc_types_eth::Header) -> Self {
        Self {
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
        }
    }
}

impl crate::BlockHeader for BlockHeader {
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

    fn prevrandao(&self) -> Option<B256> {
        self.prevrandao
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.base_fee_per_gas
    }

    fn withdraw_root(&self) -> B256 {
        self.withdrawals_root
    }
}

impl crate::BlockHeader for ArchivedBlockHeader {
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

    fn prevrandao(&self) -> Option<B256> {
        self.prevrandao.as_ref().map(|x| B256::from(*x))
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.base_fee_per_gas.as_ref().map(|x| x.to_native())
    }

    fn withdraw_root(&self) -> B256 {
        B256::from(self.withdrawals_root)
    }
}
