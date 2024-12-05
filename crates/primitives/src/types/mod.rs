use alloy_consensus::{Transaction as _, TxEnvelope, TxType};
use alloy_eips::{eip2718::Encodable2718, eip2930::AccessList, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, ChainId, SignatureError, TxHash, TxKind, B256, U256};
use alloy_rlp::BytesMut;
use serde::{Deserialize, Serialize};

#[cfg(feature = "scroll")]
mod scroll;
use crate::Block;
#[cfg(feature = "scroll")]
pub use scroll::TxL1Msg;

impl<T: alloy_consensus::Transaction> Block
    for alloy_rpc_types_eth::Block<T, alloy_rpc_types_eth::Header>
{
    type Tx = T;

    #[inline(always)]
    fn block_hash(&self) -> B256 {
        self.header.hash
    }
    #[inline(always)]
    fn state_root(&self) -> B256 {
        self.header.state_root
    }
    #[inline(always)]
    fn difficulty(&self) -> U256 {
        self.header.difficulty
    }
    #[inline(always)]
    fn number(&self) -> u64 {
        self.header.number
    }
    #[inline(always)]
    fn gas_limit(&self) -> u64 {
        self.header.gas_limit
    }
    #[inline(always)]
    fn gas_used(&self) -> u64 {
        self.header.gas_used
    }
    #[inline(always)]
    fn timestamp(&self) -> u64 {
        self.header.timestamp
    }
    #[inline(always)]
    fn prevrandao(&self) -> Option<B256> {
        self.header.mix_hash
    }
    #[inline(always)]
    fn base_fee_per_gas(&self) -> Option<u64> {
        self.header.base_fee_per_gas
    }
    #[inline(always)]
    fn withdraw_root(&self) -> B256 {
        self.header.withdrawals_root.expect("legacy block")
    }
    #[inline(always)]
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.txns()
    }
    #[inline(always)]
    fn num_txs(&self) -> usize {
        self.transactions.len()
    }
}

/// Wrapped Ethereum Transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedTransaction {
    /// Normal enveloped ethereum transaction
    Enveloped(TxEnvelope),
    #[cfg(feature = "scroll")]
    /// Layer1 Message Transaction
    L1Msg(TxL1Msg),
}

/// Transaction object used in RPC
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Hash
    pub hash: TxHash,
    /// Nonce
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    // Those fields exist, but we don't need them
    // /// Block hash
    // #[serde(default)]
    // pub block_hash: Option<BlockHash>,
    // /// Block number
    // #[serde(default, with = "alloy_serde::quantity::opt")]
    // pub block_number: Option<u64>,
    // /// Transaction Index
    // #[serde(default, with = "alloy_serde::quantity::opt")]
    // pub transaction_index: Option<u64>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[serde(with = "alloy_serde::quantity")]
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    pub max_fee_per_gas: Option<u128>,
    /// The miner's tip.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub signature: Option<alloy_rpc_types_eth::Signature>,
    /// The chain id of the transaction, if any.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    pub chain_id: Option<ChainId>,
    /// Contains the blob hashes for eip-4844 transactions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_list: Option<AccessList>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[serde(
        default,
        rename = "type",
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    #[doc(alias = "tx_type")]
    pub transaction_type: Option<u8>,
    /// The signed authorization list is a list of tuples that store the address to code which the
    /// signer desires to execute in the context of their EOA and their signature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_list: Option<Vec<SignedAuthorization>>,

    /// L1Msg queueIndex
    #[cfg(feature = "scroll")]
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    pub queue_index: Option<u64>,
}

// copied from alloy_rpc_types_eth
impl alloy_consensus::Transaction for Transaction {
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    #[cfg(not(feature = "scroll"))]
    fn nonce(&self) -> u64 {
        self.nonce
    }

    #[cfg(feature = "scroll")]
    fn nonce(&self) -> u64 {
        if self.ty() == 0x7e {
            self.queue_index.expect("queue_index is required for L1Msg")
        } else {
            self.nonce
        }
    }

    fn gas_limit(&self) -> u64 {
        self.gas
    }

    fn gas_price(&self) -> Option<u128> {
        self.gas_price
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.max_fee_per_gas
            .unwrap_or_else(|| self.gas_price.unwrap_or_default())
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.max_priority_fee_per_gas
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.max_fee_per_blob_gas
    }

    fn priority_fee_or_price(&self) -> u128 {
        debug_assert!(
            self.max_fee_per_gas.is_some() || self.gas_price.is_some(),
            "mutually exclusive fields"
        );
        self.max_fee_per_gas
            .unwrap_or_else(|| self.gas_price.unwrap_or_default())
    }

    fn kind(&self) -> TxKind {
        self.to.into()
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn input(&self) -> &Bytes {
        &self.input
    }

    fn ty(&self) -> u8 {
        self.transaction_type.unwrap_or_default()
    }

    fn access_list(&self) -> Option<&AccessList> {
        self.access_list.as_ref()
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.blob_versioned_hashes.as_deref()
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.authorization_list.as_deref()
    }
}

impl alloy_consensus::Transaction for TypedTransaction {
    fn chain_id(&self) -> Option<ChainId> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.chain_id(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.chain_id(),
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.nonce(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.nonce(),
        }
    }

    fn gas_limit(&self) -> u64 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.gas_limit(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.gas_limit(),
        }
    }

    fn gas_price(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.gas_price(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.gas_price(),
        }
    }

    fn max_fee_per_gas(&self) -> u128 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_fee_per_gas(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.max_fee_per_gas(),
        }
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_priority_fee_per_gas(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.max_priority_fee_per_gas(),
        }
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_fee_per_blob_gas(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.max_fee_per_blob_gas(),
        }
    }

    fn priority_fee_or_price(&self) -> u128 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.priority_fee_or_price(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.priority_fee_or_price(),
        }
    }

    fn kind(&self) -> TxKind {
        match self {
            TypedTransaction::Enveloped(tx) => tx.kind(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.kind(),
        }
    }

    fn value(&self) -> U256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.value(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.value(),
        }
    }

    fn input(&self) -> &Bytes {
        match self {
            TypedTransaction::Enveloped(tx) => tx.input(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.input(),
        }
    }

    fn ty(&self) -> u8 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.ty(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.ty(),
        }
    }

    fn access_list(&self) -> Option<&AccessList> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.access_list(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.access_list(),
        }
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.blob_versioned_hashes(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.blob_versioned_hashes(),
        }
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.authorization_list(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.authorization_list(),
        }
    }
}

impl TypedTransaction {
    /// Return the hash of the inner transaction.
    pub fn tx_hash(&self) -> &B256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.tx_hash(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => &tx.tx_hash,
        }
    }

    /// Get the caller of the transaction, recover the signer if the transaction is enveloped.
    ///
    /// Fails if the transaction is enveloped and recovering the signer fails.
    pub fn get_or_recover_signer(&self) -> Result<Address, SignatureError> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.recover_signer(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => Ok(tx.from),
        }
    }

    /// Get the effective gas price of the transaction.
    pub fn effective_gas_price(&self, base_fee_per_gas: u64) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(TxEnvelope::Eip1559(ref tx)) => {
                let priority_fee_per_gas = tx.tx().effective_tip_per_gas(base_fee_per_gas)?;
                Some(priority_fee_per_gas + base_fee_per_gas as u128)
            }
            _ => self.gas_price(),
        }
    }

    /// Encode the transaction according to [EIP-2718] rules. First a 1-byte
    /// type flag in the range 0x0-0x7f, then the body of the transaction.
    pub fn rlp(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        match self {
            TypedTransaction::Enveloped(tx) => tx.encode_2718(&mut bytes),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.encode_2718(&mut bytes),
        }
        Bytes(bytes.freeze())
    }

    /// Get `data`
    pub fn data(&self) -> Bytes {
        match self {
            TypedTransaction::Enveloped(tx) => match tx.tx_type() {
                TxType::Legacy => tx.as_legacy().unwrap().tx().input.clone(),
                TxType::Eip1559 => tx.as_eip1559().unwrap().tx().input.clone(),
                TxType::Eip2930 => tx.as_eip2930().unwrap().tx().input.clone(),
                _ => unimplemented!("unsupported tx type {:?}", tx.tx_type()),
            },
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.input.clone(),
        }
    }

    /// Check if the transaction is an L1 transaction
    #[cfg(feature = "scroll")]
    pub fn is_l1_msg(&self) -> bool {
        matches!(self, TypedTransaction::L1Msg(_))
    }
}
