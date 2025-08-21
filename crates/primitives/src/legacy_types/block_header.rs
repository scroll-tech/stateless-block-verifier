use crate::{
    Address, B256, BlockNumber, Bytes, U256,
    alloy_primitives::{B64, Bloom},
};

/// Block header representation.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockHeader {
    /// The Keccak 256-bit hash of the parent
    /// block’s header, in its entirety; formally Hp.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Keccak 256-bit hash of the parent block’s header, in its entirety; formally Hp."
        ))
    )]
    pub parent_hash: B256,
    /// The Keccak 256-bit hash of the ommers list portion of this block; formally Ho.
    #[cfg_attr(feature = "serde", serde(rename = "sha3Uncles", alias = "ommersHash"))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Keccak 256-bit hash of the ommers list portion of this block; formally Ho."
        ))
    )]
    pub ommers_hash: B256,
    /// The 160-bit address to which all fees collected from the successful mining of this block
    /// be transferred; formally Hc.
    #[cfg_attr(feature = "serde", serde(rename = "miner", alias = "beneficiary"))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The 160-bit address to which all fees collected from the successful mining of this block be transferred; formally Hc."
        ))
    )]
    pub beneficiary: Address,
    /// The Keccak 256-bit hash of the root node of the state trie, after all transactions are
    /// executed and finalisations applied; formally Hr.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Keccak 256-bit hash of the root node of the state trie, after all transactions are executed and finalisations applied; formally Hr."
        ))
    )]
    pub state_root: B256,
    /// The Keccak 256-bit hash of the root node of the trie structure populated with each
    /// transaction in the transactions list portion of the block; formally Ht.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Keccak 256-bit hash of the root node of the trie structure populated with each transaction in the transactions list portion of the block; formally Ht."
        ))
    )]
    pub transactions_root: B256,
    /// The Keccak 256-bit hash of the root node of the trie structure populated with the receipts
    /// of each transaction in the transactions list portion of the block; formally He.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Keccak 256-bit hash of the root node of the trie structure populated with the receipts of each transaction in the transactions list portion of the block; formally He."
        ))
    )]
    pub receipts_root: B256,
    /// The Bloom filter composed from indexable information (logger address and log topics)
    /// contained in each log entry from the receipt of each transaction in the transactions list;
    /// formally Hb.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Bloom filter composed from indexable information (logger address and log topics) contained in each log entry from the receipt of each transaction in the transactions list; formally Hb."
        ))
    )]
    pub logs_bloom: Bloom,
    /// A scalar value corresponding to the difficulty level of this block. This can be calculated
    /// from the previous block’s difficulty level and the timestamp; formally Hd.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A scalar value corresponding to the difficulty level of this block. This can be calculated from the previous block’s difficulty level and the timestamp; formally Hd."
        ))
    )]
    pub difficulty: U256,
    /// A scalar value equal to the number of ancestor blocks. The genesis block has a number of
    /// zero; formally Hi.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A scalar value equal to the number of ancestor blocks. The genesis block has a number of zero; formally Hi."
        ))
    )]
    pub number: BlockNumber,
    /// A scalar value equal to the current limit of gas expenditure per block; formally Hl.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A scalar value equal to the current limit of gas expenditure per block; formally Hl."
        ))
    )]
    pub gas_limit: u64,
    /// A scalar value equal to the total gas used in transactions in this block; formally Hg.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A scalar value equal to the total gas used in transactions in this block; formally Hg."
        ))
    )]
    pub gas_used: u64,
    /// A scalar value equal to the reasonable output of Unix’s time() at this block’s inception;
    /// formally Hs.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A scalar value equal to the reasonable output of Unix’s time() at this block’s inception; formally Hs."
        ))
    )]
    pub timestamp: u64,
    /// An arbitrary byte array containing data relevant to this block. This must be 32 bytes or
    /// fewer; formally Hx.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "An arbitrary byte array containing data relevant to this block. This must be 32 bytes or fewer; formally Hx."
        ))
    )]
    pub extra_data: Bytes,
    /// A 256-bit hash which, combined with the
    /// nonce, proves that a sufficient amount of computation has been carried out on this block;
    /// formally Hm.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A 256-bit hash which, combined with the nonce, proves that a sufficient amount of computation has been carried out on this block; formally Hm."
        ))
    )]
    pub mix_hash: B256,
    /// A 64-bit value which, combined with the mixhash, proves that a sufficient amount of
    /// computation has been carried out on this block; formally Hn.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A 64-bit value which, combined with the mixhash, proves that a sufficient amount of computation has been carried out on this block; formally Hn."
        ))
    )]
    pub nonce: B64,
    /// A scalar representing EIP1559 base fee which can move up or down each block according
    /// to a formula which is a function of gas used in parent block and gas target
    /// (block gas limit divided by elasticity multiplier) of parent block.
    /// The algorithm results in the base fee per gas increasing when blocks are
    /// above the gas target, and decreasing when blocks are below the gas target. The base fee per
    /// gas is burned.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt",)
    )]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A scalar representing EIP1559 base fee which can move up or down each block according to a formula which is a function of gas used in parent block and gas target (block gas limit divided by elasticity multiplier) of parent block. The algorithm results in the base fee per gas increasing when blocks are above the gas target, and decreasing when blocks are below the gas target. The base fee per gas is burned."
        ))
    )]
    pub base_fee_per_gas: Option<u64>,
    /// The Keccak 256-bit hash of the withdrawals list portion of this block.
    /// <https://eips.ethereum.org/EIPS/eip-4895>
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Keccak 256-bit hash of the withdrawals list portion of this block."
        ))
    )]
    pub withdrawals_root: Option<B256>,
    /// The total amount of blob gas consumed by the transactions within the block, added in
    /// EIP-4844.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt",)
    )]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The total amount of blob gas consumed by the transactions within the block, added in EIP-4844."
        ))
    )]
    pub blob_gas_used: Option<u64>,
    /// A running total of blob gas consumed in excess of the target, prior to the block. Blocks
    /// with above-target blob gas consumption increase this value, blocks with below-target blob
    /// gas consumption decrease it (bounded at 0). This was added in EIP-4844.
    #[cfg_attr(
        feature = "serde",
        serde(default, with = "alloy_serde::quantity::opt",)
    )]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "A running total of blob gas consumed in excess of the target, prior to the block. Blocks with above-target blob gas consumption increase this value, blocks with below-target blob gas consumption decrease it (bounded at 0). This was added in EIP-4844."
        ))
    )]
    pub excess_blob_gas: Option<u64>,
    /// The hash of the parent beacon block's root is included in execution blocks, as proposed by
    /// EIP-4788.
    ///
    /// This enables trust-minimized access to consensus state, supporting staking pools, bridges,
    /// and more.
    ///
    /// The beacon roots contract handles root storage, enhancing Ethereum's functionalities.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The hash of the parent beacon block's root is included in execution blocks, as proposed by EIP-4788. This enables trust-minimized access to consensus state, supporting staking pools, bridges, and more. The beacon roots contract handles root storage, enhancing Ethereum's functionalities."
        ))
    )]
    pub parent_beacon_block_root: Option<B256>,
    /// The Keccak 256-bit hash of the an RLP encoded list with each
    /// [EIP-7685] request in the block body.
    ///
    /// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The Keccak 256-bit hash of the an RLP encoded list with each [EIP-7685] request in the block body."
        ))
    )]
    pub requests_hash: Option<B256>,
}

impl From<crate::types::Header> for BlockHeader {
    fn from(header: crate::types::Header) -> Self {
        Self {
            parent_hash: header.parent_hash,
            ommers_hash: header.ommers_hash,
            beneficiary: header.beneficiary,
            state_root: header.state_root,
            transactions_root: header.transactions_root,
            receipts_root: header.receipts_root,
            logs_bloom: header.logs_bloom,
            difficulty: header.difficulty,
            number: header.number,
            gas_limit: header.gas_limit,
            gas_used: header.gas_used,
            timestamp: header.timestamp,
            extra_data: header.extra_data,
            mix_hash: header.mix_hash,
            nonce: header.nonce,
            base_fee_per_gas: header.base_fee_per_gas,
            withdrawals_root: header.withdrawals_root,
            blob_gas_used: header.blob_gas_used,
            excess_blob_gas: header.excess_blob_gas,
            parent_beacon_block_root: header.parent_beacon_block_root,
            requests_hash: header.requests_hash,
        }
    }
}
