use crate::{
    SignatureError,
    types::{
        Transaction,
        consensus::{SignableTransaction, TxEip1559, TxEip2930, TxEip7702, TxLegacy},
    },
};

pub use reth_primitives::RecoveredBlock;

#[cfg(not(feature = "scroll"))]
pub use reth_primitives::{Block, BlockBody, EthPrimitives, Receipt, TransactionSigned};
#[cfg(feature = "scroll-reth-primitives-types")]
pub use reth_scroll_primitives::{
    ScrollBlock as Block, ScrollBlockBody as BlockBody, ScrollPrimitives as EthPrimitives,
    ScrollReceipt as Receipt, ScrollTransactionSigned as TransactionSigned,
};

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
            #[cfg(feature = "scroll-consensus-types")]
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

                TransactionSigned::from(tx)
            }
            #[cfg(all(feature = "scroll", not(feature = "scroll-consensus-types")))]
            0x7e => compile_error!("unreachable"),
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}

#[cfg(feature = "rkyv")]
impl TryFrom<&crate::types::ArchivedTransaction> for TransactionSigned {
    type Error = SignatureError;

    fn try_from(tx: &crate::types::ArchivedTransaction) -> Result<Self, Self::Error> {
        let tx_type = tx.transaction_type;
        let input = crate::Bytes::copy_from_slice(tx.input.as_slice());
        let to = tx.to.as_ref().map(|to| crate::Address::from(*to));

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
            #[cfg(not(feature = "consensus-types"))]
            0x03 => {
                let sig = tx.signature.as_ref().expect("missing signature").into();
                let tx = super::consensus::TxEip4844 {
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
                        .map(|x| crate::B256::from(*x))
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
                let tx = TxEip7702 {
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
            #[cfg(feature = "scroll-consensus-types")]
            0x7e => {
                let tx = crate::types::consensus::TxL1Message {
                    queue_index: tx
                        .queue_index
                        .as_ref()
                        .expect("missing queue_index")
                        .to_native(),
                    gas_limit: tx.gas.to_native(),
                    to: to.expect("missing to"),
                    value: tx.value.into(),
                    sender: crate::Address::from(tx.from),
                    input,
                };

                TransactionSigned::from(tx)
            }
            #[cfg(all(feature = "scroll", not(feature = "scroll-consensus-types")))]
            0x7e => compile_error!("unreachable"),
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}
