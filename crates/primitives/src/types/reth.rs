use crate::{
    Withdrawal,
    alloy_primitives::SignatureError,
    types::{
        Transaction,
        consensus::{
            BlockWitnessConsensusExt, SignableTransaction, SignerRecoverable, TxEip1559, TxEip2930,
            TxLegacy,
        },
    },
};

use auto_impl::auto_impl;

pub use reth_primitives::RecoveredBlock;

#[cfg(not(feature = "scroll"))]
pub use reth_primitives::{Block, BlockBody, EthPrimitives, Receipt, TransactionSigned};
#[cfg(feature = "scroll")]
pub use reth_scroll_primitives::{
    ScrollBlock as Block, ScrollBlockBody as BlockBody, ScrollPrimitives as EthPrimitives,
    ScrollReceipt as Receipt, ScrollTransactionSigned as TransactionSigned,
};

/// BlockWitnessRethExt trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitnessRethExt: BlockWitnessConsensusExt {
    /// Transactions
    #[must_use]
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, SignatureError>>;

    /// Build a reth block
    fn build_reth_block(&self) -> Result<RecoveredBlock<Block>, SignatureError> {
        let header = self.build_alloy_header();
        let transactions = self
            .build_typed_transactions()
            .collect::<Result<Vec<_>, _>>()?;
        let senders = transactions
            .iter()
            .map(|tx| tx.recover_signer())
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to recover signer");

        let body = BlockBody {
            transactions,
            ommers: vec![],
            withdrawals: self.withdrawals_iter().map(|iter| {
                super::eips::eip4895::Withdrawals(
                    iter.map(|w| super::eips::eip4895::Withdrawal {
                        index: w.index(),
                        validator_index: w.validator_index(),
                        address: w.address(),
                        amount: w.amount(),
                    })
                    .collect(),
                )
            }),
        };

        Ok(RecoveredBlock::new_unhashed(
            Block { header, body },
            senders,
        ))
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
                let tx = super::consensus::TxEip4844 {
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
                let tx = super::consensus::TxEip7702 {
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
                use super::consensus::TxL1Message;
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

#[cfg(feature = "rkyv")]
impl TryFrom<&super::ArchivedTransaction> for TransactionSigned {
    type Error = SignatureError;

    fn try_from(tx: &super::ArchivedTransaction) -> Result<Self, Self::Error> {
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
            #[cfg(not(feature = "scroll"))]
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
                let tx = super::consensus::TxEip7702 {
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
                let tx = super::consensus::TxL1Message {
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

                TransactionSigned::new_unhashed(
                    tx.into(),
                    super::consensus::TxL1Message::signature(),
                )
            }
            _ => unimplemented!("unsupported tx type: {}", tx_type),
        };

        Ok(tx)
    }
}

impl BlockWitnessRethExt for super::BlockWitness {
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, SignatureError>> {
        self.transaction.iter().map(|tx| tx.try_into())
    }
}

#[cfg(feature = "rkyv")]
impl BlockWitnessRethExt for super::ArchivedBlockWitness {
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, SignatureError>> {
        self.transaction.iter().map(|tx| tx.try_into())
    }
}

#[cfg(feature = "rpc-types")]
impl super::BlockWitness {
    /// Creates a new block witness from a block, pre-state root, execution witness.
    pub fn new_from_block(
        chain_id: crate::ChainId,
        block: super::rpc::Block,
        pre_state_root: crate::B256,
        #[cfg(not(feature = "scroll"))] block_hashes: Vec<crate::B256>,
        witness: super::ExecutionWitness,
    ) -> Self {
        let header = super::BlockHeader::from(block.header);

        let transaction = block
            .transactions
            .into_transactions()
            .map(Transaction::from_rpc)
            .collect();
        let withdrawals = block
            .withdrawals
            .map(|w| w.iter().map(super::Withdrawal::from).collect());
        let states = witness.state.into_values().collect();
        let codes = witness.codes.into_values().collect();
        Self {
            chain_id,
            header,
            transaction,
            #[cfg(not(feature = "scroll"))]
            block_hashes,
            withdrawals,
            pre_state_root,
            states,
            codes,
        }
    }
}
