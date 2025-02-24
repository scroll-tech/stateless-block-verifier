//! Stateless Block Verifier primitives library.

use auto_impl::auto_impl;
use std::fmt;

/// The spec of an Ethereum network
pub mod chainspec;
/// Extension Traits
pub mod ext;
/// Ethereum fork types
pub mod hardforks;
/// Predeployed contracts
#[cfg(feature = "scroll")]
pub mod predeployed;
/// Types definition
pub mod types;

pub use alloy_consensus;
pub use revm;

pub use alloy_primitives::{
    self, Address, B256, BlockHash, BlockNumber, Bytes, ChainId, PrimitiveSignature, TxHash, U256,
    address, b256, keccak256,
};
pub use reth_primitives::RecoveredBlock;
pub use revm::primitives::{AccountInfo, Bytecode};

/// Network definition
#[cfg(not(feature = "scroll"))]
pub type Network = alloy_network::Ethereum;
/// Network definition
#[cfg(feature = "scroll")]
pub type Network = scroll_alloy_network::Scroll;

/// Eips
pub mod eips {
    pub use alloy_eips::*;
}

/// BlockWitness trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitness: fmt::Debug {
    /// Chain id
    fn chain_id(&self) -> ChainId;
    /// Header
    fn header(&self) -> impl types::consensus::BlockHeader;
    /// Build alloy header
    #[must_use]
    fn build_alloy_header(&self) -> types::consensus::Header;
    /// Pre-state root
    #[must_use]
    fn pre_state_root(&self) -> B256;
    /// Number of transactions
    fn num_transactions(&self) -> usize;
    /// Transactions
    #[must_use]
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<
        Item = Result<types::reth::TransactionSigned, alloy_primitives::SignatureError>,
    >;
    /// Block hashes
    #[must_use]
    #[cfg(not(feature = "scroll"))]
    fn block_hashes_iter(&self) -> impl ExactSizeIterator<Item = B256>;
    /// Withdrawals
    #[must_use]
    fn withdrawals_iter(&self) -> Option<impl ExactSizeIterator<Item = impl Withdrawal>>;
    /// States
    #[must_use]
    fn states_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>>;
    /// Codes
    #[must_use]
    fn codes_iter(&self) -> impl ExactSizeIterator<Item = impl AsRef<[u8]>>;

    // provided methods

    /// Pre-state root
    #[must_use]
    fn post_state_root(&self) -> B256 {
        use types::consensus::BlockHeader;
        self.header().state_root()
    }
    /// Withdrawal root
    #[must_use]
    fn withdrawals_root(&self) -> Option<B256> {
        use types::consensus::BlockHeader;
        self.header().withdrawals_root()
    }
    /// Number of states
    fn num_states(&self) -> usize {
        self.states_iter().len()
    }
    /// Number of codes
    fn num_codes(&self) -> usize {
        self.codes_iter().len()
    }

    /// Build a reth block
    fn build_reth_block(
        &self,
    ) -> Result<RecoveredBlock<types::reth::Block>, alloy_primitives::SignatureError> {
        use reth_primitives_traits::transaction::signed::SignedTransaction;

        let header = self.build_alloy_header();
        let transactions = self
            .build_typed_transactions()
            .collect::<Result<Vec<_>, _>>()?;
        let senders = transactions
            .iter()
            .map(|tx| tx.recover_signer())
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to recover signer");

        let body = types::reth::BlockBody {
            transactions,
            ommers: vec![],
            withdrawals: self.withdrawals_iter().map(|iter| {
                alloy_eips::eip4895::Withdrawals(
                    iter.map(|w| alloy_eips::eip4895::Withdrawal {
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
            types::reth::Block { header, body },
            senders,
        ))
    }
}

/// Withdrawal trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Withdrawal: fmt::Debug {
    /// Monotonically increasing identifier issued by consensus layer.
    fn index(&self) -> u64;
    /// Index of validator associated with withdrawal.
    fn validator_index(&self) -> u64;
    /// Target address for withdrawn ether.
    fn address(&self) -> Address;
    /// Value of the withdrawal in gwei.
    fn amount(&self) -> u64;
}

/// Chunk related extension methods for Block
/// FIXME: gate this behind scroll feature
pub trait BlockChunkExt {
    /// Hash the header of the block
    fn hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher);
    /// Hash the l1 messages of the block
    fn hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher);
}

impl BlockChunkExt for RecoveredBlock<types::reth::Block> {
    #[inline]
    fn hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher) {
        hasher.update(&self.number.to_be_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.update(
            &U256::from_limbs([self.base_fee_per_gas.unwrap_or_default(), 0, 0, 0])
                .to_be_bytes::<{ U256::BYTES }>(),
        );
        hasher.update(&self.gas_limit.to_be_bytes());
        hasher.update(&(self.body().transactions.len() as u16).to_be_bytes()); // FIXME: l1 tx could be skipped, the actual tx count needs to be calculated
    }

    #[inline]
    fn hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher) {
        use reth_primitives_traits::SignedTransaction;
        use types::consensus::Typed2718;
        for tx in self.body().transactions.iter().filter(|tx| tx.ty() == 0x7e) {
            hasher.update(tx.tx_hash().as_slice())
        }
    }
}
