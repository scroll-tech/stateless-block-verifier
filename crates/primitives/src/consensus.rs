use crate::{Address, B64, B256, BlockNumber, Bloom, Bytes, Signature, U256};
use auto_impl::auto_impl;

pub use alloy_consensus::{
    BlockHeader, Header, SignableTransaction, Transaction, TxEip1559, TxEip2930, TxEip4844,
    TxEip4844Variant, TxEip4844WithSidecar, TxEip7702, TxLegacy, Typed2718,
    transaction::SignerRecoverable,
};

#[cfg(not(feature = "scroll"))]
pub use alloy_consensus::{TxEnvelope, TxType, TypedTransaction};
#[cfg(feature = "scroll-consensus-types")]
pub use scroll_alloy_consensus::{
    ScrollReceiptEnvelope as ReceiptEnvelope, ScrollTxEnvelope as TxEnvelope,
    ScrollTxType as TxType, ScrollTypedTransaction as TypedTransaction, TxL1Message,
};

/// Extension trait for `TxEnvelope`
pub trait TxEnvelopeExt {
    /// get the signature of the transaction
    fn signature(&self) -> Option<&Signature>;

    /// get the index of the transaction in the queue
    fn queue_index(&self) -> Option<u64> {
        None
    }
}

/// BlockWitnessConsensusExt trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitnessConsensusExt {
    /// Header
    fn header(&self) -> impl BlockHeader;
    /// Build alloy header
    #[must_use]
    fn build_alloy_header(&self) -> Header;
}

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub(crate) trait FromHelper: BlockHeader {}

#[auto_impl(&, &mut, Box, Rc, Arc)]
pub(crate) trait ToHelper: BlockHeader {
    fn to_alloy(&self) -> Header {
        Header {
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

#[cfg(not(feature = "scroll"))]
impl TxEnvelopeExt for TxEnvelope {
    fn signature(&self) -> Option<&Signature> {
        Some(TxEnvelope::signature(self))
    }
}

#[cfg(feature = "scroll-consensus-types")]
impl TxEnvelopeExt for TxEnvelope {
    fn signature(&self) -> Option<&Signature> {
        match self {
            TxEnvelope::Legacy(tx) => Some(tx.signature()),
            TxEnvelope::Eip2930(tx) => Some(tx.signature()),
            TxEnvelope::Eip1559(tx) => Some(tx.signature()),
            TxEnvelope::Eip7702(tx) => Some(tx.signature()),
            _ => None,
        }
    }

    fn queue_index(&self) -> Option<u64> {
        match self {
            TxEnvelope::L1Message(tx) => Some(tx.queue_index),
            _ => None,
        }
    }
}

impl BlockWitnessConsensusExt for super::BlockWitness {
    fn header(&self) -> impl BlockHeader {
        &self.header
    }
    fn build_alloy_header(&self) -> Header {
        self.header.to_alloy()
    }
}

#[cfg(feature = "rpc-types")]
impl FromHelper for alloy_rpc_types_eth::Header {}
impl FromHelper for Header {}

impl<T: FromHelper> From<T> for super::BlockHeader {
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

impl ToHelper for super::BlockHeader {}

impl BlockHeader for super::BlockHeader {
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
