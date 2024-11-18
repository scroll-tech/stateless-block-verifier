use crate::alloy_primitives::{BlockHash, TxHash};
use crate::TxTrace;
use alloy::{
    consensus::{Transaction, TxEnvelope, TxType},
    eips::eip2718::Encodable2718,
    eips::{eip2930::AccessList, eip7702::SignedAuthorization},
    primitives::{Address, Bytes, ChainId, Signature, SignatureError, TxKind, B256, U256, U64},
    rlp::{BufMut, BytesMut, Encodable, Header},
};
use rkyv::rancor;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnNull};

/// Wrapped Ethereum Transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedTransaction {
    /// Normal enveloped ethereum transaction
    Enveloped(TxEnvelope),
    /// Layer1 Message Transaction
    L1Msg(TxL1Msg),
}

/// Layer1 Message Transaction
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TxL1Msg {
    /// The 32-byte hash of the transaction.
    pub tx_hash: B256,
    /// The 160-bit address of the message call’s sender.
    pub from: Address,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    pub nonce: u64,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    pub gas_limit: u64,
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    pub to: TxKind,
    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    pub value: U256,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
}

/// Transaction Trace
#[serde_as]
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    Clone,
)]
#[rkyv(attr(doc = "Archived `TransactionTrace`"))]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct TransactionTrace {
    /// tx hash
    #[rkyv(attr(doc = "tx hash"))]
    #[serde(default, rename = "txHash")]
    pub tx_hash: B256,
    /// tx type (in raw from)
    #[rkyv(attr(doc = "tx type (in raw from)"))]
    #[serde(rename = "type")]
    pub ty: u8,
    /// nonce
    #[rkyv(attr(doc = "nonce"))]
    pub nonce: u64,
    /// gas limit
    #[rkyv(attr(doc = "gas limit"))]
    pub gas: u64,
    /// gas price
    #[rkyv(attr(doc = "gas price"))]
    #[serde(rename = "gasPrice")]
    pub gas_price: U256,
    /// gas tip cap
    #[rkyv(attr(doc = "gas tip cap"))]
    #[serde(rename = "gasTipCap")]
    pub gas_tip_cap: Option<U256>,
    /// gas fee cap
    #[rkyv(attr(doc = "gas fee cap"))]
    #[serde(rename = "gasFeeCap")]
    pub gas_fee_cap: Option<U256>,
    /// from
    #[rkyv(attr(doc = "from"))]
    pub from: Address,
    /// to, NONE for creation (0 addr)
    #[rkyv(attr(doc = "to, NONE for creation (0 addr)"))]
    pub to: Option<Address>,
    /// chain id
    #[rkyv(attr(doc = "chain id"))]
    #[serde(rename = "chainId")]
    pub chain_id: U64,
    /// value amount
    #[rkyv(attr(doc = "value amount"))]
    pub value: U256,
    /// call data
    #[rkyv(attr(doc = "call data"))]
    pub data: Bytes,
    /// is creation
    #[rkyv(attr(doc = "is creation"))]
    #[serde(rename = "isCreate")]
    pub is_create: bool,
    /// access list
    #[rkyv(attr(doc = "access list"))]
    #[serde(rename = "accessList")]
    #[serde_as(as = "DefaultOnNull")]
    pub access_list: AccessList,
    /// signature v
    #[rkyv(attr(doc = "signature v"))]
    pub v: U64,
    /// signature r
    #[rkyv(attr(doc = "signature r"))]
    pub r: U256,
    /// signature s
    #[rkyv(attr(doc = "signature s"))]
    pub s: U256,
}

impl TxTrace for TransactionTrace {
    fn tx_hash(&self) -> B256 {
        self.tx_hash
    }

    fn ty(&self) -> u8 {
        self.ty
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u64 {
        self.gas
    }

    fn gas_price(&self) -> u128 {
        self.gas_price.to()
    }

    fn max_fee_per_gas(&self) -> Option<u128> {
        self.gas_fee_cap.map(|v| v.to())
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.gas_tip_cap.map(|v| v.to())
    }

    unsafe fn get_from_unchecked(&self) -> Address {
        self.from
    }

    fn to(&self) -> TxKind {
        if self.is_create {
            TxKind::Create
        } else {
            TxKind::Call(self.to.expect("to address must be present"))
        }
    }

    fn chain_id(&self) -> Option<ChainId> {
        let chain_id: ChainId = self.chain_id.to();
        if self.ty == 0 && self.v() < 35 {
            None
        } else {
            Some(chain_id)
        }
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn data(&self) -> Bytes {
        self.data.clone()
    }

    fn access_list(&self) -> AccessList {
        self.access_list.clone()
    }

    fn v(&self) -> u64 {
        self.v.to()
    }

    fn signature(&self) -> Result<Signature, SignatureError> {
        Signature::from_rs_and_parity(self.r, self.s, self.v)
    }
}

impl TxTrace for ArchivedTransactionTrace {
    fn tx_hash(&self) -> B256 {
        self.tx_hash.into()
    }

    fn ty(&self) -> u8 {
        self.ty
    }

    fn nonce(&self) -> u64 {
        self.nonce.into()
    }

    fn gas_limit(&self) -> u64 {
        u64::from(self.gas)
    }

    fn gas_price(&self) -> u128 {
        let gas_price: U256 = self.gas_price.into();
        gas_price.to()
    }

    fn max_fee_per_gas(&self) -> Option<u128> {
        self.gas_fee_cap.as_ref().map(|g| {
            let gas_fee_cap: U256 = g.into();
            gas_fee_cap.to()
        })
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.gas_tip_cap.as_ref().map(|g| {
            let gas_tip_cap: U256 = g.into();
            gas_tip_cap.to()
        })
    }

    unsafe fn get_from_unchecked(&self) -> Address {
        self.from.into()
    }

    fn to(&self) -> TxKind {
        if self.is_create {
            TxKind::Create
        } else {
            let to: Address = self.to.as_ref().expect("to address must be present").into();
            debug_assert!(!to.is_zero());
            TxKind::Call(to)
        }
    }

    fn chain_id(&self) -> Option<ChainId> {
        let chain_id: U64 = self.chain_id.into();
        if self.ty == 0 && self.v() < 35 {
            None
        } else {
            Some(chain_id.to())
        }
    }

    fn value(&self) -> U256 {
        self.value.into()
    }

    fn data(&self) -> Bytes {
        Bytes::copy_from_slice(self.data.as_ref())
    }

    fn access_list(&self) -> AccessList {
        rkyv::deserialize::<_, rancor::Error>(&self.access_list).unwrap()
    }

    fn v(&self) -> u64 {
        let v: U64 = self.v.into();
        v.to()
    }

    fn signature(&self) -> Result<Signature, SignatureError> {
        let v: U64 = self.v.into();
        Signature::from_rs_and_parity(self.r.into(), self.s.into(), v)
    }
}

/// Transaction object used in RPC
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlloyTransaction {
    /// Hash
    pub hash: TxHash,
    /// Nonce
    #[serde(with = "alloy::serde::quantity")]
    pub nonce: u64,
    /// Block hash
    #[serde(default)]
    pub block_hash: Option<BlockHash>,
    /// Block number
    #[serde(default, with = "alloy::serde::quantity::opt")]
    pub block_number: Option<u64>,
    /// Transaction Index
    #[serde(default, with = "alloy::serde::quantity::opt")]
    pub transaction_index: Option<u64>,
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
        with = "alloy::serde::quantity::opt"
    )]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[serde(with = "alloy::serde::quantity")]
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy::serde::quantity::opt"
    )]
    pub max_fee_per_gas: Option<u128>,
    /// The miner's tip.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy::serde::quantity::opt"
    )]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy::serde::quantity::opt"
    )]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub signature: Option<alloy::rpc::types::Signature>,
    /// The chain id of the transaction, if any.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy::serde::quantity::opt"
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
        with = "alloy::serde::quantity::opt"
    )]
    #[doc(alias = "tx_type")]
    pub transaction_type: Option<u8>,
    /// The signed authorization list is a list of tuples that store the address to code which the
    /// signer desires to execute in the context of their EOA and their signature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_list: Option<Vec<SignedAuthorization>>,

    /// L1Msg queueIndex
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy::serde::quantity::opt"
    )]
    pub queue_index: Option<u64>,
}

impl TxTrace for AlloyTransaction {
    fn tx_hash(&self) -> B256 {
        self.hash
    }

    fn ty(&self) -> u8 {
        self.transaction_type.unwrap_or(0)
    }

    fn nonce(&self) -> u64 {
        if self.ty() != 0x7e {
            self.nonce
        } else {
            self.queue_index.unwrap()
        }
    }

    fn gas_limit(&self) -> u64 {
        self.gas
    }

    fn gas_price(&self) -> u128 {
        self.gas_price.unwrap_or_default()
    }

    fn max_fee_per_gas(&self) -> Option<u128> {
        self.max_fee_per_gas
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.max_priority_fee_per_gas
    }

    unsafe fn get_from_unchecked(&self) -> Address {
        self.from
    }

    fn to(&self) -> TxKind {
        match self.to {
            Some(addr) => TxKind::Call(addr),
            None => TxKind::Create,
        }
    }

    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn data(&self) -> Bytes {
        self.input.clone()
    }

    fn access_list(&self) -> AccessList {
        self.access_list.clone().unwrap_or_default()
    }

    fn v(&self) -> u64 {
        self.signature.unwrap().v.to()
    }

    fn signature(&self) -> Result<Signature, SignatureError> {
        let sig = self.signature.unwrap();
        Signature::from_rs_and_parity(sig.r, sig.s, sig.v.to::<u64>())
    }
}

impl Transaction for TypedTransaction {
    fn chain_id(&self) -> Option<ChainId> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.chain_id(),
            TypedTransaction::L1Msg(tx) => tx.chain_id(),
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.nonce(),
            TypedTransaction::L1Msg(tx) => tx.nonce(),
        }
    }

    fn gas_limit(&self) -> u64 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.gas_limit(),
            TypedTransaction::L1Msg(tx) => tx.gas_limit(),
        }
    }

    fn gas_price(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.gas_price(),
            TypedTransaction::L1Msg(tx) => tx.gas_price(),
        }
    }

    fn max_fee_per_gas(&self) -> u128 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_fee_per_gas(),
            TypedTransaction::L1Msg(tx) => tx.max_fee_per_gas(),
        }
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_priority_fee_per_gas(),
            TypedTransaction::L1Msg(tx) => tx.max_priority_fee_per_gas(),
        }
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_fee_per_blob_gas(),
            TypedTransaction::L1Msg(tx) => tx.max_fee_per_blob_gas(),
        }
    }

    fn priority_fee_or_price(&self) -> u128 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.priority_fee_or_price(),
            TypedTransaction::L1Msg(tx) => tx.priority_fee_or_price(),
        }
    }

    fn kind(&self) -> TxKind {
        match self {
            TypedTransaction::Enveloped(tx) => tx.kind(),
            TypedTransaction::L1Msg(tx) => tx.kind(),
        }
    }

    fn value(&self) -> U256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.value(),
            TypedTransaction::L1Msg(tx) => tx.value(),
        }
    }

    fn input(&self) -> &Bytes {
        match self {
            TypedTransaction::Enveloped(tx) => tx.input(),
            TypedTransaction::L1Msg(tx) => tx.input(),
        }
    }

    fn ty(&self) -> u8 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.ty(),
            TypedTransaction::L1Msg(tx) => tx.ty(),
        }
    }

    fn access_list(&self) -> Option<&AccessList> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.access_list(),
            TypedTransaction::L1Msg(tx) => tx.access_list(),
        }
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.blob_versioned_hashes(),
            TypedTransaction::L1Msg(tx) => tx.blob_versioned_hashes(),
        }
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.authorization_list(),
            TypedTransaction::L1Msg(tx) => tx.authorization_list(),
        }
    }
}

impl TxL1Msg {
    /// Outputs the length of the transaction's fields.
    #[doc(hidden)]
    pub fn fields_len(&self) -> usize {
        let mut len = 0;
        len += self.nonce.length();
        len += self.gas_limit.length();
        len += self.to.length();
        len += self.value.length();
        len += self.input.0.length();
        len += self.from.length();
        len
    }
}

impl Transaction for TxL1Msg {
    fn chain_id(&self) -> Option<ChainId> {
        None
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn gas_price(&self) -> Option<u128> {
        Some(0)
    }

    fn max_fee_per_gas(&self) -> u128 {
        0
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        None
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        None
    }

    fn priority_fee_or_price(&self) -> u128 {
        0
    }

    fn kind(&self) -> TxKind {
        self.to
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn input(&self) -> &Bytes {
        &self.input
    }

    fn ty(&self) -> u8 {
        0x7e
    }

    fn access_list(&self) -> Option<&AccessList> {
        None
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        None
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        None
    }
}

impl Encodable for TxL1Msg {
    fn encode(&self, out: &mut dyn BufMut) {
        self.nonce.encode(out);
        self.gas_limit.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.from.encode(out);
    }
}

impl Encodable2718 for TxL1Msg {
    fn type_flag(&self) -> Option<u8> {
        Some(0x7e)
    }

    fn encode_2718_len(&self) -> usize {
        let payload_length = self.fields_len();
        1 + Header {
            list: true,
            payload_length,
        }
        .length()
            + payload_length
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        0x7eu8.encode(out);
        let header = Header {
            list: true,
            payload_length: self.fields_len(),
        };
        header.encode(out);
        self.encode(out)
    }
}

impl TypedTransaction {
    /// Return the hash of the inner transaction.
    pub fn tx_hash(&self) -> &B256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.tx_hash(),
            TypedTransaction::L1Msg(tx) => &tx.tx_hash,
        }
    }

    /// Get the caller of the transaction, recover the signer if the transaction is enveloped.
    ///
    /// Fails if the transaction is enveloped and recovering the signer fails.
    pub fn get_or_recover_signer(&self) -> Result<Address, SignatureError> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.recover_signer(),
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
            TypedTransaction::L1Msg(tx) => tx.input.clone(),
        }
    }

    /// Check if the transaction is an L1 transaction
    pub fn is_l1_msg(&self) -> bool {
        matches!(self, TypedTransaction::L1Msg(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACE: &str = include_str!("../../../../testdata/mainnet_blocks/8370400.json");

    #[test]
    fn test_transaction_trace_deserialize() {
        let trace = serde_json::from_str::<serde_json::Value>(TRACE).unwrap()["result"].clone();
        let txs = trace["transactions"].clone();
        for tx in txs.as_array().unwrap() {
            let tx: TransactionTrace = serde_json::from_value(tx.clone()).unwrap();
            let _ = tx.try_build_typed_tx().unwrap();
        }
    }
}
