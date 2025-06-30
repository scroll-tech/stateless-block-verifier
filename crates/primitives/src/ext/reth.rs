use crate::{
    SignatureError, Withdrawal,
    types::{
        BlockWitness,
        consensus::{BlockWitnessConsensusExt, SignerRecoverable},
        eips::eip4895,
        reth::primitives::{Block, BlockBody, RecoveredBlock, TransactionSigned},
    },
};
use auto_impl::auto_impl;

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
                eip4895::Withdrawals(
                    iter.map(|w| eip4895::Withdrawal {
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

impl BlockWitnessRethExt for BlockWitness {
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, SignatureError>> {
        self.transaction.iter().map(|tx| tx.try_into())
    }
}

#[cfg(feature = "rkyv")]
impl BlockWitnessRethExt for crate::types::ArchivedBlockWitness {
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, SignatureError>> {
        self.transaction.iter().map(|tx| tx.try_into())
    }
}
