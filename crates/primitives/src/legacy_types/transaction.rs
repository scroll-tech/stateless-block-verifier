use crate::{
    Address, B256, Bytes, ChainId, SignatureError, TxHash, U256,
    legacy_types::{access_list::AccessList, auth_list::SignedAuthorization, signature::Signature},
    types::{
        consensus::{
            SignableTransaction, SignerRecoverable, Transaction as _, TxEip1559, TxEip2930,
            TxEip7702, TxEnvelope, TxLegacy,
        },
        eips::Typed2718,
    },
};
#[cfg(feature = "scroll")]
use scroll_alloy_consensus::ScrollTransaction;

/// Transaction object used in RPC
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Transaction {
    /// Hash
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Hash")))]
    pub hash: TxHash,
    /// Nonce
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Nonce")))]
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    /// Sender
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Sender")))]
    pub from: Address,
    /// Recipient
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Recipient")))]
    pub to: Option<Address>,
    /// Transferred value
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Transferred value")))]
    pub value: U256,
    /// Gas Price
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Gas Price")))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Gas amount")))]
    #[serde(with = "alloy_serde::quantity")]
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Max BaseFeePerGas the user is willing to pay."))
    )]
    #[serde(with = "alloy_serde::quantity")]
    pub max_fee_per_gas: u128,
    /// The miner's tip.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "The miner's tip.")))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Configured max fee per blob gas for eip-4844 transactions"))
    )]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "Data")))]
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "All _flattened_ fields of the transaction signature. Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported."
        ))
    )]
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The chain id of the transaction, if any."))
    )]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub chain_id: Option<ChainId>,
    /// Contains the blob hashes for eip-4844 transactions.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Contains the blob hashes for eip-4844 transactions."))
    )]
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "EIP2930 Pre-pay to warm storage access."))
    )]
    pub access_list: Option<AccessList>,
    /// EIP7702
    ///
    /// Authorizations are used to temporarily set the code of its signer to
    /// the code referenced by `address`. These also include a `chain_id` (which
    /// can be set to zero and not evaluated) as well as an optional `nonce`.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "EIP7702 Authorizations")))]
    pub authorization_list: Option<Vec<SignedAuthorization>>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "EIP2718 Transaction type, Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559 transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy"
        ))
    )]
    #[doc(alias = "tx_type")]
    pub transaction_type: u8,
    /// L1Msg queueIndex
    #[cfg(feature = "scroll")]
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "L1Msg queueIndex")))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub queue_index: Option<u64>,
}

impl From<TxEnvelope> for Transaction {
    fn from(tx: TxEnvelope) -> Self {
        #[cfg(feature = "scroll")]
        use crate::types::reth::primitives::SignedTransaction;

        Self {
            hash: *tx.tx_hash(),
            nonce: tx.nonce(),
            from: tx.recover_signer().expect("invalid signature"),
            to: tx.to(),
            value: tx.value(),
            gas_price: tx.gas_price(),
            gas: tx.gas_limit(),
            max_fee_per_gas: tx.max_fee_per_gas(),
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas(),
            max_fee_per_blob_gas: tx.max_fee_per_blob_gas(),
            input: tx.input().clone(),
            #[cfg(feature = "scroll")]
            signature: tx.signature().map(Into::into),
            #[cfg(not(feature = "scroll"))]
            signature: Some((*tx.signature()).into()),
            chain_id: tx.chain_id(),
            blob_versioned_hashes: tx.blob_versioned_hashes().map(ToOwned::to_owned),
            access_list: tx.access_list().cloned().map(Into::into),
            authorization_list: tx
                .authorization_list()
                .map(|list| list.iter().cloned().map(Into::into).collect()),
            transaction_type: tx.ty(),
            #[cfg(feature = "scroll")]
            queue_index: tx.queue_index(),
        }
    }
}

impl TryFrom<Transaction> for TxEnvelope {
    type Error = SignatureError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let tx_type = tx.transaction_type;

        let tx = match tx_type {
            0x00 => {
                let sig = tx.signature.expect("missing signature").into();
                let tx = TxLegacy {
                    chain_id: tx.chain_id,
                    nonce: tx.nonce,
                    gas_price: tx.gas_price.unwrap(),
                    gas_limit: tx.gas,
                    to: tx.to.into(),
                    value: tx.value,
                    input: tx.input,
                };

                tx.into_signed(sig).into()
            }
            0x01 => {
                let sig = tx.signature.expect("missing signature").into();
                let tx = TxEip2930 {
                    chain_id: tx.chain_id.expect("missing chain_id"),
                    nonce: tx.nonce,
                    gas_price: tx.gas_price.unwrap(),
                    gas_limit: tx.gas,
                    to: tx.to.into(),
                    value: tx.value,
                    access_list: tx.access_list.expect("missing access_list").into(),
                    input: tx.input,
                };

                tx.into_signed(sig).into()
            }
            0x02 => {
                let sig = tx.signature.expect("missing signature").into();
                let tx = TxEip1559 {
                    chain_id: tx.chain_id.expect("missing chain_id"),
                    nonce: tx.nonce,
                    max_fee_per_gas: tx.max_fee_per_gas,
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .expect("missing max_priority_fee_per_gas"),
                    gas_limit: tx.gas,
                    to: tx.to.into(),
                    value: tx.value,
                    access_list: tx.access_list.expect("missing access_list").into(),
                    input: tx.input,
                };

                tx.into_signed(sig).into()
            }
            #[cfg(not(feature = "scroll"))]
            0x03 => {
                use crate::types::consensus::TxEip4844;
                let sig = tx.signature.expect("missing signature").into();
                let tx = TxEip4844 {
                    chain_id: tx.chain_id.expect("missing chain_id"),
                    nonce: tx.nonce,
                    max_fee_per_gas: tx.max_fee_per_gas,
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .expect("missing max_priority_fee_per_gas"),
                    gas_limit: tx.gas,
                    to: tx.to.expect("missing to"),
                    value: tx.value,
                    input: tx.input,
                    access_list: tx.access_list.expect("missing access_list").into(),
                    blob_versioned_hashes: tx
                        .blob_versioned_hashes
                        .expect("missing blob_versioned_hashes"),
                    max_fee_per_blob_gas: tx
                        .max_fee_per_blob_gas
                        .expect("missing max_fee_per_blob_gas"),
                };
                tx.into_signed(sig).into()
            }
            0x04 => {
                let sig = tx.signature.expect("missing signature").into();
                let tx = TxEip7702 {
                    chain_id: tx.chain_id.expect("missing chain_id"),
                    nonce: tx.nonce,
                    gas_limit: tx.gas,
                    max_fee_per_gas: tx.max_fee_per_gas,
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .expect("missing max_priority_fee_per_gas"),
                    to: tx.to.expect("missing to"),
                    value: tx.value,
                    access_list: tx.access_list.expect("missing access_list").into(),
                    authorization_list: tx
                        .authorization_list
                        .expect("missing authorization_list")
                        .into_iter()
                        .map(|x| x.into())
                        .collect(),
                    input: tx.input,
                };
                tx.into_signed(sig).into()
            }
            #[cfg(feature = "scroll")]
            0x7e => {
                use crate::types::consensus::TxL1Message;
                let tx = TxL1Message {
                    queue_index: tx.queue_index.expect("missing queue_index"),
                    gas_limit: tx.gas,
                    to: tx.to.expect("missing to"),
                    value: tx.value,
                    sender: tx.from,
                    input: tx.input.clone(),
                };

                TxEnvelope::from(tx)
            }
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}
