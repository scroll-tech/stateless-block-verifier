use super::{access_list::AccessList, signature::Signature};
use alloy_consensus::{
    SignableTransaction, Transaction as _, TxEip1559, TxEip2930, TxEnvelope, TxLegacy, TxType,
    Typed2718,
};
use alloy_eips::eip2718::Encodable2718;
use alloy_eips::eip7702::SignedAuthorization;
use alloy_primitives::bytes::BytesMut;
use alloy_primitives::{Address, Bytes, ChainId, SignatureError, TxHash, TxKind, B256, U256};

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
    #[rkyv(attr(doc = ""))]
    pub hash: TxHash,
    /// Nonce
    #[rkyv(attr(doc = ""))]
    pub nonce: u64,
    /// Sender
    #[rkyv(attr(doc = ""))]
    pub from: Address,
    /// Recipient
    #[rkyv(attr(doc = ""))]
    pub to: Option<Address>,
    /// Transferred value
    #[rkyv(attr(doc = ""))]
    pub value: U256,
    /// Gas Price
    #[rkyv(attr(doc = ""))]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[rkyv(attr(doc = ""))]
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    #[rkyv(attr(doc = ""))]
    pub max_fee_per_gas: u128,
    /// The miner's tip.
    #[rkyv(attr(doc = ""))]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[rkyv(attr(doc = ""))]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    #[rkyv(attr(doc = ""))]
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[rkyv(attr(doc = ""))]
    pub signature: Signature,
    /// The chain id of the transaction, if any.
    #[rkyv(attr(doc = ""))]
    pub chain_id: Option<ChainId>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[rkyv(attr(doc = ""))]
    pub access_list: Option<AccessList>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[rkyv(attr(doc = ""))]
    pub transaction_type: u8,
    /// L1Msg queueIndex
    #[cfg(feature = "scroll")]
    #[rkyv(attr(doc = ""))]
    pub queue_index: Option<u64>,
}

impl Transaction {
    /// Create a transaction from an alloy transaction
    pub fn from_alloy(
        tx: alloy_rpc_types_eth::Transaction,
        #[cfg(feature = "scroll")] queue_index: Option<u64>,
    ) -> Self {
        Self {
            hash: *tx.inner.tx_hash(),
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
            signature: tx.inner.signature().into(),
            chain_id: tx.chain_id(),
            access_list: tx.access_list().map(Into::into),
            transaction_type: tx.ty(),
            #[cfg(feature = "scroll")]
            queue_index,
        }
    }
}

/// Wrapped Ethereum Transaction
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypedTransaction {
    /// Normal enveloped ethereum transaction
    Enveloped(TxEnvelope),
    #[cfg(feature = "scroll")]
    /// Layer1 Message Transaction
    L1Msg(super::TxL1Msg),
}

impl TypedTransaction {
    /// Return the chain id of the inner transaction.
    pub fn chain_id(&self) -> Option<ChainId> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.chain_id(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => None,
        }
    }

    /// Return the nonce of the inner transaction.
    pub fn nonce(&self) -> Option<u64> {
        match self {
            TypedTransaction::Enveloped(tx) => Some(tx.nonce()),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => None,
        }
    }

    /// Return the hash of the inner transaction.
    pub fn tx_hash(&self) -> &B256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.tx_hash(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => &tx.tx_hash,
        }
    }

    /// Return the gas limit of the inner transaction.
    pub fn gas_limit(&self) -> u64 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.gas_limit(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.gas_limit,
        }
    }

    /// Returns the EIP-1559 Priority fee the caller is paying to the block author.
    ///
    /// This will return `None` for non-EIP1559 transactions
    pub fn max_priority_fee_per_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_priority_fee_per_gas(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => None,
        }
    }

    /// Max fee per blob gas for EIP-4844 transaction.
    ///
    /// Returns `None` for non-eip4844 transactions.
    ///
    /// This is also commonly referred to as the "Blob Gas Fee Cap".
    pub fn max_fee_per_blob_gas(&self) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.max_fee_per_blob_gas(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => None,
        }
    }

    /// Get the effective gas price of the transaction.
    pub fn effective_gas_price(&self, base_fee_per_gas: u64) -> Option<u128> {
        match self {
            TypedTransaction::Enveloped(TxEnvelope::Eip1559(ref tx)) => {
                let priority_fee_per_gas = tx.tx().effective_tip_per_gas(base_fee_per_gas)?;
                Some(priority_fee_per_gas + base_fee_per_gas as u128)
            }
            TypedTransaction::Enveloped(tx) => tx.gas_price(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => Some(0),
        }
    }

    /// Return the transaction kind of the inner transaction.
    pub fn kind(&self) -> TxKind {
        match self {
            TypedTransaction::Enveloped(tx) => tx.kind(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.to.into(),
        }
    }

    /// Return the value of the inner transaction.
    pub fn value(&self) -> U256 {
        match self {
            TypedTransaction::Enveloped(tx) => tx.value(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(tx) => tx.value,
        }
    }

    /// Return the input of the inner transaction.
    pub fn input(&self) -> Bytes {
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

    /// Returns the EIP-2930 `access_list` for the particular transaction type. Returns `None` for
    /// older transaction types.
    pub fn access_list(&self) -> Option<&alloy_eips::eip2930::AccessList> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.access_list(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => None,
        }
    }

    /// Blob versioned hashes for eip4844 transaction. For previous transaction types this is
    /// `None`.
    pub fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.blob_versioned_hashes(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => None,
        }
    }

    /// Returns the [`SignedAuthorization`] list of the transaction.
    ///
    /// Returns `None` if this transaction is not EIP-7702.
    pub fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        match self {
            TypedTransaction::Enveloped(tx) => tx.authorization_list(),
            #[cfg(feature = "scroll")]
            TypedTransaction::L1Msg(_) => None,
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

    /// Check if the transaction is an L1 transaction
    #[cfg(feature = "scroll")]
    pub fn is_l1_msg(&self) -> bool {
        matches!(self, TypedTransaction::L1Msg(_))
    }
}

impl TryFrom<&Transaction> for TypedTransaction {
    type Error = SignatureError;

    fn try_from(tx: &Transaction) -> Result<Self, Self::Error> {
        let tx_type = tx.transaction_type;

        let tx = match tx_type {
            0x0 => {
                let sig = tx.signature.try_into()?;
                let tx = TxLegacy {
                    chain_id: tx.chain_id,
                    nonce: tx.nonce,
                    gas_price: tx.gas_price.unwrap(),
                    gas_limit: tx.gas,
                    to: tx.to.into(),
                    value: tx.value,
                    input: tx.input.clone(),
                };

                TypedTransaction::Enveloped(TxEnvelope::from(tx.into_signed(sig)))
            }
            0x1 => {
                let sig = tx.signature.try_into()?;
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

                TypedTransaction::Enveloped(TxEnvelope::from(tx.into_signed(sig)))
            }
            0x02 => {
                let sig = tx.signature.try_into()?;
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

                TypedTransaction::Enveloped(TxEnvelope::from(tx.into_signed(sig)))
            }
            #[cfg(feature = "scroll")]
            0x7e => {
                let tx = super::TxL1Msg {
                    tx_hash: tx.hash,
                    from: tx.from,
                    nonce: tx.queue_index.unwrap(),
                    gas_limit: tx.gas,
                    to: tx.to.into(),
                    value: tx.value,
                    input: tx.input.clone(),
                };

                TypedTransaction::L1Msg(tx)
            }
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}

impl TryFrom<&ArchivedTransaction> for TypedTransaction {
    type Error = SignatureError;

    fn try_from(tx: &ArchivedTransaction) -> Result<Self, Self::Error> {
        let tx_type = tx.transaction_type;
        let input = Bytes::copy_from_slice(tx.input.as_slice());
        let to = tx.to.as_ref().map(|to| Address::from(*to)).into();

        let tx = match tx_type {
            0x0 => {
                let sig = (&tx.signature).try_into()?;
                let tx = TxLegacy {
                    chain_id: tx.chain_id.as_ref().map(|x| x.to_native()),
                    nonce: tx.nonce.to_native(),
                    gas_price: tx.gas_price.unwrap().to_native(),
                    gas_limit: tx.gas.to_native(),
                    to,
                    value: tx.value.into(),
                    input,
                };

                TypedTransaction::Enveloped(TxEnvelope::from(tx.into_signed(sig)))
            }
            0x1 => {
                let sig = (&tx.signature).try_into()?;
                let tx = TxEip2930 {
                    chain_id: tx.chain_id.unwrap().to_native(),
                    nonce: tx.nonce.to_native(),
                    gas_price: tx.gas_price.unwrap().to_native(),
                    gas_limit: tx.gas.to_native(),
                    to,
                    value: tx.value.into(),
                    access_list: tx.access_list.as_ref().expect("missing access_list").into(),
                    input,
                };

                TypedTransaction::Enveloped(TxEnvelope::from(tx.into_signed(sig)))
            }
            0x02 => {
                let sig = (&tx.signature).try_into()?;
                let tx = TxEip1559 {
                    chain_id: tx.chain_id.unwrap().to_native(),
                    nonce: tx.nonce.to_native(),
                    max_fee_per_gas: tx.max_fee_per_gas.to_native(),
                    max_priority_fee_per_gas: tx
                        .max_priority_fee_per_gas
                        .as_ref()
                        .expect("missing max_priority_fee_per_gas")
                        .to_native(),
                    gas_limit: tx.gas.to_native(),
                    to,
                    value: tx.value.into(),
                    access_list: tx.access_list.as_ref().expect("missing access_list").into(),
                    input,
                };

                TypedTransaction::Enveloped(TxEnvelope::from(tx.into_signed(sig)))
            }
            #[cfg(feature = "scroll")]
            0x7e => {
                let tx = super::TxL1Msg {
                    tx_hash: tx.hash.into(),
                    from: tx.from.into(),
                    nonce: tx.queue_index.unwrap().to_native(),
                    gas_limit: tx.gas.to_native(),
                    to,
                    value: tx.value.into(),
                    input,
                };

                TypedTransaction::L1Msg(tx)
            }
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}
