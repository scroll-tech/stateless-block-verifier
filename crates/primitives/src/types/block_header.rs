use crate::{
    Address, B256, BlockNumber, Bytes, U256,
    alloy_primitives::{B64, Bloom},
};
use auto_impl::auto_impl;
use std::sync::OnceLock;

/// Block header representation.
#[derive(
    Clone,
    Debug,
    Default,
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
    /// The Keccak 256-bit hash of the parent
    /// block’s header, in its entirety; formally Hp.
    #[rkyv(attr(
        doc = "The Keccak 256-bit hash of the parent block’s header, in its entirety; formally Hp."
    ))]
    pub parent_hash: B256,
    /// The Keccak 256-bit hash of the ommers list portion of this block; formally Ho.
    #[serde(rename = "sha3Uncles", alias = "ommersHash")]
    #[rkyv(attr(
        doc = "The Keccak 256-bit hash of the ommers list portion of this block; formally Ho."
    ))]
    pub ommers_hash: B256,
    /// The 160-bit address to which all fees collected from the successful mining of this block
    /// be transferred; formally Hc.
    #[serde(rename = "miner", alias = "beneficiary")]
    #[rkyv(attr(
        doc = "The 160-bit address to which all fees collected from the successful mining of this block be transferred; formally Hc."
    ))]
    pub beneficiary: Address,
    /// The Keccak 256-bit hash of the root node of the state trie, after all transactions are
    /// executed and finalisations applied; formally Hr.
    #[rkyv(attr(
        doc = "The Keccak 256-bit hash of the root node of the state trie, after all transactions are executed and finalisations applied; formally Hr."
    ))]
    pub state_root: B256,
    /// The Keccak 256-bit hash of the root node of the trie structure populated with each
    /// transaction in the transactions list portion of the block; formally Ht.
    #[rkyv(attr(
        doc = "The Keccak 256-bit hash of the root node of the trie structure populated with each transaction in the transactions list portion of the block; formally Ht."
    ))]
    pub transactions_root: B256,
    /// The Keccak 256-bit hash of the root node of the trie structure populated with the receipts
    /// of each transaction in the transactions list portion of the block; formally He.
    #[rkyv(attr(
        doc = "The Keccak 256-bit hash of the root node of the trie structure populated with the receipts of each transaction in the transactions list portion of the block; formally He."
    ))]
    pub receipts_root: B256,
    /// The Bloom filter composed from indexable information (logger address and log topics)
    /// contained in each log entry from the receipt of each transaction in the transactions list;
    /// formally Hb.
    #[rkyv(attr(
        doc = "The Bloom filter composed from indexable information (logger address and log topics) contained in each log entry from the receipt of each transaction in the transactions list; formally Hb."
    ))]
    pub logs_bloom: Bloom,
    /// A scalar value corresponding to the difficulty level of this block. This can be calculated
    /// from the previous block’s difficulty level and the timestamp; formally Hd.
    #[rkyv(attr(
        doc = "A scalar value corresponding to the difficulty level of this block. This can be calculated from the previous block’s difficulty level and the timestamp; formally Hd."
    ))]
    pub difficulty: U256,
    /// A scalar value equal to the number of ancestor blocks. The genesis block has a number of
    /// zero; formally Hi.
    #[serde(with = "alloy_serde::quantity")]
    #[rkyv(attr(
        doc = "A scalar value equal to the number of ancestor blocks. The genesis block has a number of zero; formally Hi."
    ))]
    pub number: BlockNumber,
    /// A scalar value equal to the current limit of gas expenditure per block; formally Hl.
    #[serde(with = "alloy_serde::quantity")]
    #[rkyv(attr(
        doc = "A scalar value equal to the current limit of gas expenditure per block; formally Hl."
    ))]
    pub gas_limit: u64,
    /// A scalar value equal to the total gas used in transactions in this block; formally Hg.
    #[serde(with = "alloy_serde::quantity")]
    #[rkyv(attr(
        doc = "A scalar value equal to the total gas used in transactions in this block; formally Hg."
    ))]
    pub gas_used: u64,
    /// A scalar value equal to the reasonable output of Unix’s time() at this block’s inception;
    /// formally Hs.
    #[serde(with = "alloy_serde::quantity")]
    #[rkyv(attr(
        doc = "A scalar value equal to the reasonable output of Unix’s time() at this block’s inception; formally Hs."
    ))]
    pub timestamp: u64,
    /// An arbitrary byte array containing data relevant to this block. This must be 32 bytes or
    /// fewer; formally Hx.
    #[rkyv(attr(
        doc = "An arbitrary byte array containing data relevant to this block. This must be 32 bytes or fewer; formally Hx."
    ))]
    pub extra_data: Bytes,
    /// A 256-bit hash which, combined with the
    /// nonce, proves that a sufficient amount of computation has been carried out on this block;
    /// formally Hm.
    #[rkyv(attr(
        doc = "A 256-bit hash which, combined with the nonce, proves that a sufficient amount of computation has been carried out on this block; formally Hm."
    ))]
    pub mix_hash: B256,
    /// A 64-bit value which, combined with the mixhash, proves that a sufficient amount of
    /// computation has been carried out on this block; formally Hn.
    #[rkyv(attr(
        doc = "A 64-bit value which, combined with the mixhash, proves that a sufficient amount of computation has been carried out on this block; formally Hn."
    ))]
    pub nonce: B64,
    /// A scalar representing EIP1559 base fee which can move up or down each block according
    /// to a formula which is a function of gas used in parent block and gas target
    /// (block gas limit divided by elasticity multiplier) of parent block.
    /// The algorithm results in the base fee per gas increasing when blocks are
    /// above the gas target, and decreasing when blocks are below the gas target. The base fee per
    /// gas is burned.
    #[serde(
        default,
        with = "alloy_serde::quantity::opt",
    )]
    #[rkyv(attr(
        doc = "A scalar representing EIP1559 base fee which can move up or down each block according to a formula which is a function of gas used in parent block and gas target (block gas limit divided by elasticity multiplier) of parent block. The algorithm results in the base fee per gas increasing when blocks are above the gas target, and decreasing when blocks are below the gas target. The base fee per gas is burned."
    ))]
    pub base_fee_per_gas: Option<u64>,
    /// The Keccak 256-bit hash of the withdrawals list portion of this block.
    /// <https://eips.ethereum.org/EIPS/eip-4895>
    #[serde(default)]
    #[rkyv(attr(doc = "The Keccak 256-bit hash of the withdrawals list portion of this block."))]
    pub withdrawals_root: Option<B256>,
    /// The total amount of blob gas consumed by the transactions within the block, added in
    /// EIP-4844.
    #[serde(
        default,
        with = "alloy_serde::quantity::opt",
    )]
    #[rkyv(attr(
        doc = "The total amount of blob gas consumed by the transactions within the block, added in EIP-4844."
    ))]
    pub blob_gas_used: Option<u64>,
    /// A running total of blob gas consumed in excess of the target, prior to the block. Blocks
    /// with above-target blob gas consumption increase this value, blocks with below-target blob
    /// gas consumption decrease it (bounded at 0). This was added in EIP-4844.
    #[serde(
        default,
        with = "alloy_serde::quantity::opt",
    )]
    #[rkyv(attr(
        doc = "A running total of blob gas consumed in excess of the target, prior to the block. Blocks with above-target blob gas consumption increase this value, blocks with below-target blob gas consumption decrease it (bounded at 0). This was added in EIP-4844."
    ))]
    pub excess_blob_gas: Option<u64>,
    /// The hash of the parent beacon block's root is included in execution blocks, as proposed by
    /// EIP-4788.
    ///
    /// This enables trust-minimized access to consensus state, supporting staking pools, bridges,
    /// and more.
    ///
    /// The beacon roots contract handles root storage, enhancing Ethereum's functionalities.
    #[serde(default)]
    #[rkyv(attr(
        doc = "The hash of the parent beacon block's root is included in execution blocks, as proposed by EIP-4788. This enables trust-minimized access to consensus state, supporting staking pools, bridges, and more. The beacon roots contract handles root storage, enhancing Ethereum's functionalities."
    ))]
    pub parent_beacon_block_root: Option<B256>,
    /// The Keccak 256-bit hash of the an RLP encoded list with each
    /// [EIP-7685] request in the block body.
    ///
    /// [EIP-7685]: https://eips.ethereum.org/EIPS/eip-7685
    #[serde(default)]
    #[rkyv(attr(
        doc = "The Keccak 256-bit hash of the an RLP encoded list with each [EIP-7685] request in the block body."
    ))]
    pub requests_hash: Option<B256>,
}

#[auto_impl(&, &mut, Box, Rc, Arc)]
trait FromHelper: alloy_consensus::BlockHeader {}

impl FromHelper for alloy_rpc_types_eth::Header {}
impl FromHelper for alloy_consensus::Header {}

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub(crate) trait ToHelper: alloy_consensus::BlockHeader {
    fn to_alloy(&self) -> alloy_consensus::Header {
        alloy_consensus::Header {
            parent_hash: self.parent_hash(),
            ommers_hash: self.ommers_hash(),
            beneficiary: self.beneficiary(),
            state_root: self.state_root(),
            transactions_root: self.transactions_root(),
            receipts_root: self.receipts_root(),
            logs_bloom: self.logs_bloom(),
            difficulty: self.difficulty(),
            number: self.number(),
            gas_limit: self.gas_limit(),
            gas_used: self.gas_used(),
            timestamp: self.timestamp(),
            extra_data: self.extra_data().clone(),
            mix_hash: self.mix_hash().unwrap(),
            nonce: self.nonce().unwrap(),
            base_fee_per_gas: self.base_fee_per_gas(),
            withdrawals_root: self.withdrawals_root(),
            blob_gas_used: self.blob_gas_used(),
            excess_blob_gas: self.excess_blob_gas(),
            parent_beacon_block_root: self.parent_beacon_block_root(),
            requests_hash: self.requests_hash(),
        }
    }
}

impl ToHelper for BlockHeader {}
impl ToHelper for ArchivedBlockHeader {}

impl<T: FromHelper> From<T> for BlockHeader {
    fn from(header: T) -> Self {
        Self {
            parent_hash: header.parent_hash(),
            ommers_hash: header.ommers_hash(),
            beneficiary: header.beneficiary(),
            state_root: header.state_root(),
            transactions_root: header.transactions_root(),
            receipts_root: header.receipts_root(),
            logs_bloom: header.logs_bloom(),
            difficulty: header.difficulty(),
            number: header.number(),
            gas_limit: header.gas_limit(),
            gas_used: header.gas_used(),
            timestamp: header.timestamp(),
            extra_data: header.extra_data().clone(),
            mix_hash: header.mix_hash().expect("mix hash"),
            nonce: header.nonce().unwrap(),
            base_fee_per_gas: header.base_fee_per_gas(),
            withdrawals_root: header.withdrawals_root(),
            blob_gas_used: header.blob_gas_used(),
            excess_blob_gas: header.excess_blob_gas(),
            parent_beacon_block_root: header.parent_beacon_block_root(),
            requests_hash: header.requests_hash(),
        }
    }
}

impl alloy_consensus::BlockHeader for BlockHeader {
    fn parent_hash(&self) -> B256 {
        self.parent_hash
    }

    fn ommers_hash(&self) -> B256 {
        self.ommers_hash
    }

    fn beneficiary(&self) -> Address {
        self.beneficiary
    }

    fn state_root(&self) -> B256 {
        self.state_root
    }

    fn transactions_root(&self) -> B256 {
        self.transactions_root
    }

    fn receipts_root(&self) -> B256 {
        self.receipts_root
    }

    fn withdrawals_root(&self) -> Option<B256> {
        self.withdrawals_root
    }

    fn logs_bloom(&self) -> Bloom {
        self.logs_bloom
    }

    fn difficulty(&self) -> U256 {
        self.difficulty
    }

    fn number(&self) -> BlockNumber {
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

    fn mix_hash(&self) -> Option<B256> {
        Some(self.mix_hash)
    }

    fn nonce(&self) -> Option<B64> {
        Some(self.nonce)
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.base_fee_per_gas
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.blob_gas_used
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.excess_blob_gas
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.parent_beacon_block_root
    }

    fn requests_hash(&self) -> Option<B256> {
        self.requests_hash
    }

    fn extra_data(&self) -> &Bytes {
        &self.extra_data
    }
}

impl alloy_consensus::BlockHeader for ArchivedBlockHeader {
    fn parent_hash(&self) -> B256 {
        self.parent_hash.into()
    }

    fn ommers_hash(&self) -> B256 {
        self.ommers_hash.into()
    }

    fn beneficiary(&self) -> Address {
        self.beneficiary.into()
    }

    fn state_root(&self) -> B256 {
        self.state_root.into()
    }

    fn transactions_root(&self) -> B256 {
        self.transactions_root.into()
    }

    fn receipts_root(&self) -> B256 {
        self.receipts_root.into()
    }

    fn withdrawals_root(&self) -> Option<B256> {
        self.withdrawals_root.as_ref().map(|x| x.0.into())
    }

    fn logs_bloom(&self) -> Bloom {
        self.logs_bloom.into()
    }

    fn difficulty(&self) -> U256 {
        self.difficulty.into()
    }

    fn number(&self) -> BlockNumber {
        self.number.into()
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

    fn mix_hash(&self) -> Option<B256> {
        Some(self.mix_hash.into())
    }

    fn nonce(&self) -> Option<B64> {
        Some(self.nonce.into())
    }

    fn base_fee_per_gas(&self) -> Option<u64> {
        self.base_fee_per_gas.as_ref().map(|x| x.to_native())
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.blob_gas_used.as_ref().map(|x| x.to_native())
    }

    fn excess_blob_gas(&self) -> Option<u64> {
        self.excess_blob_gas.as_ref().map(|x| x.to_native())
    }

    fn parent_beacon_block_root(&self) -> Option<B256> {
        self.parent_beacon_block_root.as_ref().map(|x| x.0.into())
    }

    fn requests_hash(&self) -> Option<B256> {
        self.requests_hash.as_ref().map(|x| x.0.into())
    }

    fn extra_data(&self) -> &Bytes {
        static BYTES: OnceLock<Bytes> = OnceLock::new();
        BYTES.get_or_init(|| Bytes::copy_from_slice(self.extra_data.as_slice()))
    }
}
