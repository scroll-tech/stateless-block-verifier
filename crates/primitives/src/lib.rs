//! Stateless Block Verifier primitives library.

use auto_impl::auto_impl;
use std::fmt;

/// Predeployed contracts
#[cfg(feature = "scroll")]
pub mod predeployed;
/// Types definition
pub mod types;

pub use alloy_consensus;
pub use alloy_eips;

pub use alloy_consensus::Header;
pub use alloy_primitives;
pub use alloy_primitives::{
    address, b256, keccak256, Address, BlockHash, BlockNumber, Bytes, ChainId, B256, U256,
};
pub use reth_primitives::{Block, BlockBody, BlockWithSenders, TransactionSigned};
use sbv_kv::KeyValueStore;

/// The spec of an Ethereum network
pub mod chainspec {
    pub use reth_chainspec::*;
    use std::sync::Arc;

    /// Get chain spec
    pub fn get_chain_spec(chain: Chain) -> Option<Arc<ChainSpec>> {
        if chain == Chain::from_named(NamedChain::Mainnet) {
            return Some(MAINNET.clone());
        }
        if chain == Chain::from_named(NamedChain::Sepolia) {
            return Some(SEPOLIA.clone());
        }
        if chain == Chain::from_named(NamedChain::Holesky) {
            return Some(HOLESKY.clone());
        }
        None
    }
}

/// BlockWitness trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitness: fmt::Debug {
    /// Chain id
    fn chain_id(&self) -> ChainId;
    /// Header
    #[must_use]
    fn header(&self) -> Header;
    /// Pre-state root
    fn pre_state_root(&self) -> B256;
    /// Number of transactions
    fn num_transactions(&self) -> usize;
    /// Transactions
    fn build_typed_transactions(
        &self,
    ) -> impl Iterator<Item = Result<TransactionSigned, alloy_primitives::SignatureError>>;
    /// Withdrawals
    fn withdrawals_iter(&self) -> Option<impl Iterator<Item = impl Withdrawal>>;
    /// States
    fn states_iter(&self) -> impl Iterator<Item = impl AsRef<[u8]>>;
    /// Codes
    fn codes_iter(&self) -> impl Iterator<Item = impl AsRef<[u8]>>;

    /// Import codes into code db
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.codes_iter() {
            let code = code.as_ref();
            let code_hash = keccak256(code);
            code_db.insert(code_hash, Bytes::copy_from_slice(code))
        }
    }

    /// Build a reth block
    fn build_reth_block(&self) -> Result<BlockWithSenders, alloy_primitives::SignatureError> {
        let header = self.header();
        let transactions = self
            .build_typed_transactions()
            .collect::<Result<Vec<_>, _>>()?;
        let senders =
            TransactionSigned::recover_signers(&transactions, transactions.len()).unwrap(); // FIXME: proper error handling

        let body = BlockBody {
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

        Ok(BlockWithSenders::new_unchecked(
            Block { header, body },
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

// FIXME
// #[cfg(feature = "scroll")]
// pub trait BlockScrollExt: Block {
//     /// start l1 queue index
//     fn start_l1_queue_index(&self) -> u64;
//
//     /// Number of l1 transactions
//     #[inline]
//     fn num_l1_txs(&self) -> u64 {
//         // 0x7e is l1 tx
//         match self
//             .transactions()
//             .filter(|tx| tx.is_l1_tx())
//             // tx.nonce for l1 tx is the l1 queue index, which is a globally index,
//             // not per user as suggested by the name...
//             .map(|tx| tx.nonce())
//             .max()
//         {
//             None => 0, // not l1 tx in this block
//             Some(end_l1_queue_index) => end_l1_queue_index - self.start_l1_queue_index() + 1,
//         }
//     }
//
//     /// Number of l2 transactions
//     #[inline]
//     fn num_l2_txs(&self) -> u64 {
//         // 0x7e is l1 tx
//         self.transactions().filter(|tx| !tx.is_l1_tx()).count() as u64
//     }
//
//     /// Hash the header of the block
//     #[inline]
//     fn hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher) {
//         let num_txs = (self.num_l1_txs() + self.num_l2_txs()) as u16;
//         hasher.update(&self.number().to_be_bytes());
//         hasher.update(&self.timestamp().to::<u64>().to_be_bytes());
//         hasher.update(
//             &self
//                 .base_fee_per_gas()
//                 .map(U256::from)
//                 .unwrap_or_default()
//                 .to_be_bytes::<{ U256::BYTES }>(),
//         );
//         hasher.update(&self.gas_limit().to::<u64>().to_be_bytes());
//         hasher.update(&num_txs.to_be_bytes());
//     }
//
//     /// Hash the l1 messages of the block
//     #[inline]
//     fn hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher) {
//         for tx_hash in self
//             .transactions()
//             .filter(|tx| tx.is_l1_tx())
//             .map(|tx| tx.tx_hash())
//         {
//             hasher.update(tx_hash.as_slice())
//         }
//     }
// }
