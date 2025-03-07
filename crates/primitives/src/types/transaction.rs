use crate::{
    Address, B256, Bytes, ChainId, TxHash, U256,
    alloy_primitives::SignatureError,
    eips::Encodable2718,
    types::{
        access_list::AccessList,
        auth_list::SignedAuthorization,
        consensus::{
            SignableTransaction, Transaction as _, TxEip1559, TxEip2930, TxEnvelope, TxEnvelopeExt,
            TxLegacy, Typed2718,
        },
        reth::TransactionSigned,
        rpc::AlloyRpcTransaction,
        signature::Signature,
    },
};

/// Transaction object used in RPC
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct Transaction {
    /// Hash
    #[rkyv(attr(doc = "Hash"))]
    pub hash: TxHash,
    /// Nonce
    #[rkyv(attr(doc = "Nonce"))]
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    /// Sender
    #[rkyv(attr(doc = "Sender"))]
    pub from: Address,
    /// Recipient
    #[rkyv(attr(doc = "Recipient"))]
    pub to: Option<Address>,
    /// Transferred value
    #[rkyv(attr(doc = "Transferred value"))]
    pub value: U256,
    /// Gas Price
    #[rkyv(attr(doc = "Gas Price"))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[rkyv(attr(doc = "Gas amount"))]
    #[serde(with = "alloy_serde::quantity")]
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    #[rkyv(attr(doc = "Max BaseFeePerGas the user is willing to pay."))]
    #[serde(with = "alloy_serde::quantity")]
    pub max_fee_per_gas: u128,
    /// The miner's tip.
    #[rkyv(attr(doc = "The miner's tip."))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[rkyv(attr(doc = "Configured max fee per blob gas for eip-4844 transactions"))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    #[rkyv(attr(doc = "Data"))]
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[rkyv(attr(
        doc = "All _flattened_ fields of the transaction signature. Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported."
    ))]
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    #[rkyv(attr(doc = "The chain id of the transaction, if any."))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub chain_id: Option<ChainId>,
    /// Contains the blob hashes for eip-4844 transactions.
    #[rkyv(attr(doc = "Contains the blob hashes for eip-4844 transactions."))]
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[rkyv(attr(doc = "EIP2930 Pre-pay to warm storage access."))]
    pub access_list: Option<AccessList>,
    /// EIP7702
    ///
    /// Authorizations are used to temporarily set the code of its signer to
    /// the code referenced by `address`. These also include a `chain_id` (which
    /// can be set to zero and not evaluated) as well as an optional `nonce`.
    #[rkyv(attr(doc = "EIP7702 Authorizations"))]
    pub authorization_list: Option<Vec<SignedAuthorization>>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[rkyv(attr(
        doc = "EIP2718 Transaction type, Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559 transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy"
    ))]
    #[doc(alias = "tx_type")]
    pub transaction_type: u8,
    /// L1Msg queueIndex
    #[cfg(feature = "scroll")]
    #[rkyv(attr(doc = "L1Msg queueIndex"))]
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub queue_index: Option<u64>,
}

impl Transaction {
    /// Create a transaction from a rpc transaction
    #[cfg(feature = "scroll")]
    pub fn from_rpc(tx: crate::types::rpc::Transaction) -> Self {
        Transaction::from_rpc_inner(tx.inner)
    }

    /// Create a transaction from a rpc transaction
    #[cfg(not(feature = "scroll"))]
    pub fn from_rpc(tx: crate::types::rpc::Transaction) -> Self {
        Transaction::from_rpc_inner(tx)
    }

    fn from_rpc_inner(tx: AlloyRpcTransaction<TxEnvelope>) -> Self {
        Self {
            hash: tx.inner.trie_hash(),
            nonce: tx.nonce(),
            from: tx.from,
            to: tx.to(),
            value: tx.value(),
            gas_price: tx.gas_price(),
            gas: tx.gas_limit(),
            max_fee_per_gas: tx.max_fee_per_gas(),
            max_priority_fee_per_gas: tx.max_priority_fee_per_gas(),
            max_fee_per_blob_gas: tx.max_fee_per_blob_gas(),
            input: tx.input().clone(),
            signature: TxEnvelopeExt::signature(&tx.inner).map(Into::into),
            chain_id: tx.chain_id(),
            blob_versioned_hashes: tx.blob_versioned_hashes().map(Vec::from),
            access_list: tx.access_list().map(Into::into),
            transaction_type: tx.ty(),
            authorization_list: tx
                .authorization_list()
                .map(|list| list.iter().map(Into::<SignedAuthorization>::into).collect()),
            #[cfg(feature = "scroll")]
            queue_index: tx.inner.queue_index(), // FIXME: scroll mode
        }
    }
}

impl TryFrom<&Transaction> for TransactionSigned {
    type Error = SignatureError;

    fn try_from(tx: &Transaction) -> Result<Self, Self::Error> {
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
                    input: tx.input.clone(),
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
                    access_list: tx.access_list.clone().expect("missing access_list").into(),
                    input: tx.input.clone(),
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
                    access_list: tx.access_list.clone().expect("missing access_list").into(),
                    input: tx.input.clone(),
                };

                tx.into_signed(sig).into()
            }
            #[cfg(not(feature = "scroll"))]
            0x03 => {
                let sig = tx.signature.expect("missing signature").into();
                let tx = alloy_consensus::TxEip4844 {
                    chain_id: tx.chain_id.expect("missing chain_id"),
                    nonce: tx.nonce,
                    max_fee_per_gas: tx.max_fee_per_gas,
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .expect("missing max_priority_fee_per_gas"),
                    gas_limit: tx.gas,
                    to: tx.to.expect("missing to"),
                    value: tx.value,
                    input: tx.input.clone(),
                    access_list: tx.access_list.clone().expect("missing access_list").into(),
                    blob_versioned_hashes: tx
                        .blob_versioned_hashes
                        .clone()
                        .expect("missing blob_versioned_hashes"),
                    max_fee_per_blob_gas: tx
                        .max_fee_per_blob_gas
                        .expect("missing max_fee_per_blob_gas"),
                };
                tx.into_signed(sig).into()
            }
            0x04 => {
                let sig = tx.signature.expect("missing signature").into();
                let tx = alloy_consensus::TxEip7702 {
                    chain_id: tx.chain_id.expect("missing chain_id"),
                    nonce: tx.nonce,
                    gas_limit: tx.gas,
                    max_fee_per_gas: tx.max_fee_per_gas,
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .expect("missing max_priority_fee_per_gas"),
                    to: tx.to.expect("missing to"),
                    value: tx.value,
                    access_list: tx.access_list.clone().expect("missing access_list").into(),
                    authorization_list: tx
                        .authorization_list
                        .as_ref()
                        .expect("missing authorization_list")
                        .iter()
                        .cloned()
                        .map(|x| x.into())
                        .collect(),
                    input: tx.input.clone(),
                };
                tx.into_signed(sig).into()
            }
            #[cfg(feature = "scroll")]
            0x7e => {
                use scroll_alloy_consensus::TxL1Message;
                let tx = TxL1Message {
                    queue_index: tx.queue_index.expect("missing queue_index"),
                    gas_limit: tx.gas,
                    to: tx.to.expect("missing to"),
                    value: tx.value,
                    sender: tx.from,
                    input: tx.input.clone(),
                };

                TransactionSigned::new_unhashed(tx.into(), TxL1Message::signature())
            }
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}

impl TryFrom<&ArchivedTransaction> for TransactionSigned {
    type Error = SignatureError;

    fn try_from(tx: &ArchivedTransaction) -> Result<Self, Self::Error> {
        let tx_type = tx.transaction_type;
        let input = Bytes::copy_from_slice(tx.input.as_slice());
        let to = tx.to.as_ref().map(|to| Address::from(*to));

        let tx = match tx_type {
            0x00 => {
                let sig = tx.signature.as_ref().expect("missing signature").into();
                let tx = TxLegacy {
                    chain_id: tx.chain_id.as_ref().map(|x| x.to_native()),
                    nonce: tx.nonce.to_native(),
                    gas_price: tx.gas_price.unwrap().to_native(),
                    gas_limit: tx.gas.to_native(),
                    to: to.into(),
                    value: tx.value.into(),
                    input,
                };

                tx.into_signed(sig).into()
            }
            0x01 => {
                let sig = tx.signature.as_ref().expect("missing signature").into();
                let tx = TxEip2930 {
                    chain_id: tx.chain_id.as_ref().expect("missing chain_id").to_native(),
                    nonce: tx.nonce.to_native(),
                    gas_price: tx.gas_price.unwrap().to_native(),
                    gas_limit: tx.gas.to_native(),
                    to: to.into(),
                    value: tx.value.into(),
                    access_list: tx.access_list.as_ref().expect("missing access_list").into(),
                    input,
                };

                tx.into_signed(sig).into()
            }
            0x02 => {
                let sig = tx.signature.as_ref().expect("missing signature").into();
                let tx = TxEip1559 {
                    chain_id: tx.chain_id.as_ref().expect("missing chain_id").to_native(),
                    nonce: tx.nonce.to_native(),
                    max_fee_per_gas: tx.max_fee_per_gas.to_native(),
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .as_ref()
                        .expect("missing max_priority_fee_per_gas")
                        .to_native(),
                    gas_limit: tx.gas.to_native(),
                    to: to.into(),
                    value: tx.value.into(),
                    access_list: tx.access_list.as_ref().expect("missing access_list").into(),
                    input,
                };

                tx.into_signed(sig).into()
            }
            #[cfg(not(feature = "scroll"))]
            0x03 => {
                let sig = tx.signature.as_ref().expect("missing signature").into();
                let tx = alloy_consensus::TxEip4844 {
                    chain_id: tx.chain_id.as_ref().expect("missing chain_id").to_native(),
                    nonce: tx.nonce.to_native(),
                    max_fee_per_gas: tx.max_fee_per_gas.to_native(),
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .as_ref()
                        .expect("missing max_priority_fee_per_gas")
                        .to_native(),
                    gas_limit: tx.gas.to_native(),
                    to: to.expect("missing to"),
                    value: tx.value.into(),
                    input,
                    access_list: tx.access_list.as_ref().expect("missing access_list").into(),
                    blob_versioned_hashes: tx
                        .blob_versioned_hashes
                        .as_ref()
                        .expect("missing blob_versioned_hashes")
                        .iter()
                        .map(|x| B256::from(*x))
                        .collect(),
                    max_fee_per_blob_gas: tx
                        .max_fee_per_blob_gas
                        .as_ref()
                        .expect("missing max_fee_per_blob_gas")
                        .to_native(),
                };
                tx.into_signed(sig).into()
            }
            0x04 => {
                let sig = tx.signature.as_ref().expect("missing signature").into();
                let tx = alloy_consensus::TxEip7702 {
                    chain_id: tx.chain_id.as_ref().expect("missing chain_id").to_native(),
                    nonce: tx.nonce.to_native(),
                    gas_limit: tx.gas.to_native(),
                    max_fee_per_gas: tx.max_fee_per_gas.to_native(),
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .as_ref()
                        .expect("missing max_priority_fee_per_gas")
                        .to_native(),
                    to: to.expect("missing to"),
                    value: tx.value.into(),
                    access_list: tx.access_list.as_ref().expect("missing access_list").into(),
                    authorization_list: tx
                        .authorization_list
                        .as_ref()
                        .expect("missing authorization_list")
                        .iter()
                        .map(|x| x.into())
                        .collect(),
                    input,
                };
                tx.into_signed(sig).into()
            }
            #[cfg(feature = "scroll")]
            0x7e => {
                use scroll_alloy_consensus::TxL1Message;
                let tx = TxL1Message {
                    queue_index: tx
                        .queue_index
                        .as_ref()
                        .expect("missing queue_index")
                        .to_native(),
                    gas_limit: tx.gas.to_native(),
                    to: to.expect("missing to"),
                    value: tx.value.into(),
                    sender: Address::from(tx.from),
                    input,
                };

                TransactionSigned::new_unhashed(tx.into(), TxL1Message::signature())
            }
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}
