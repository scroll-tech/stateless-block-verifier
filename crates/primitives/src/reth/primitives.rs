use crate::{
    SignatureError, Transaction,
    consensus::{SignableTransaction, TxEip1559, TxEip2930, TxEip7702, TxLegacy},
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
                let sig = tx.signature.expect("missing signature");
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
                let sig = tx.signature.expect("missing signature");
                let tx = TxEip2930 {
                    chain_id: tx.chain_id.expect("missing chain_id"),
                    nonce: tx.nonce,
                    gas_price: tx.gas_price.unwrap(),
                    gas_limit: tx.gas,
                    to: tx.to.into(),
                    value: tx.value,
                    access_list: tx.access_list.clone().expect("missing access_list"),
                    input: tx.input.clone(),
                };

                tx.into_signed(sig).into()
            }
            0x02 => {
                let sig = tx.signature.expect("missing signature");
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
                    access_list: tx.access_list.clone().expect("missing access_list"),
                    input: tx.input.clone(),
                };

                tx.into_signed(sig).into()
            }
            #[cfg(not(feature = "scroll"))]
            0x03 => {
                use crate::consensus::TxEip4844;
                let sig = tx.signature.expect("missing signature");
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
                    access_list: tx.access_list.clone().expect("missing access_list"),
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
                let sig = tx.signature.expect("missing signature");
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
                    access_list: tx.access_list.clone().expect("missing access_list"),
                    authorization_list: tx
                        .authorization_list
                        .as_ref()
                        .expect("missing authorization_list")
                        .iter()
                        .map(Into::into)
                        .collect(),
                    input: tx.input.clone(),
                };
                tx.into_signed(sig).into()
            }
            #[cfg(feature = "scroll-consensus-types")]
            0x7e => {
                use crate::consensus::TxL1Message;
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
