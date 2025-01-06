//! Stateless Block Verifier primitives library.

use auto_impl::auto_impl;
use sbv_kv::KeyValueStore;
use std::fmt;

/// Predeployed contracts
#[cfg(feature = "scroll")]
pub mod predeployed;
/// Types definition
pub mod types;

pub use alloy_consensus;
pub use alloy_eips;

use alloy_consensus::Typed2718;
pub use alloy_consensus::{BlockHeader, Header};
pub use alloy_primitives;
pub use alloy_primitives::{
    address, b256, keccak256, Address, BlockHash, BlockNumber, Bytes, ChainId, B256, U256,
};
pub use reth_primitives::{Block, BlockBody, BlockWithSenders, Receipt, TransactionSigned};
use sbv_helpers::cycle_track;

/// The spec of an Ethereum network
pub mod chainspec {
    use std::sync::Arc;

    pub use reth_chainspec::*;
    #[cfg(feature = "scroll")]
    pub use reth_scroll_chainspec as scroll;

    /// An Ethereum chain specification.
    ///
    /// A chain specification describes:
    ///
    /// - Meta-information about the chain (the chain ID)
    /// - The genesis block of the chain ([`Genesis`])
    /// - What hardforks are activated, and under which conditions
    #[cfg(not(feature = "scroll"))]
    pub type ChainSpec = reth_chainspec::ChainSpec;
    /// Scroll chain spec type.
    #[cfg(feature = "scroll")]
    pub type ChainSpec = scroll::ScrollChainSpec;

    /// Get chain spec
    #[cfg(not(feature = "scroll"))]
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
        if chain == Chain::dev() {
            return Some(DEV.clone());
        }
        None
    }

    /// Get chain spec
    #[cfg(feature = "scroll")]
    pub fn get_chain_spec(chain: Chain) -> Option<Arc<ChainSpec>> {
        if chain == Chain::from_named(NamedChain::Scroll) {
            return Some(scroll::SCROLL_MAINNET.clone());
        }
        if chain == Chain::from_named(NamedChain::ScrollSepolia) {
            return Some(scroll::SCROLL_SEPOLIA.clone());
        }
        if chain == Chain::dev() {
            return Some(scroll::SCROLL_DEV.clone());
        }
        None
    }
}

/// Eips
pub mod eips {
    pub use alloy_eips::eip2718::Encodable2718;
}

/// States types
pub mod states {
    #[cfg(not(feature = "scroll"))]
    pub use revm::db::BundleAccount;

    #[cfg(feature = "scroll")]
    pub use reth_scroll_revm::states::ScrollBundleAccount as BundleAccount;
}

/// BlockWitness trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait BlockWitness: fmt::Debug {
    /// Chain id
    fn chain_id(&self) -> ChainId;
    /// Header
    fn header(&self) -> impl BlockHeader;
    /// Build alloy header
    #[must_use]
    fn build_alloy_header(&self) -> Header;
    /// Pre-state root
    #[must_use]
    fn pre_state_root(&self) -> B256;
    /// Number of transactions
    fn num_transactions(&self) -> usize;
    /// Transactions
    #[must_use]
    fn build_typed_transactions(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<TransactionSigned, alloy_primitives::SignatureError>>;
    /// Block hashes
    #[must_use]
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
        self.header().state_root()
    }
    /// Withdrawal root
    #[must_use]
    fn withdrawals_root(&self) -> Option<B256> {
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
    fn build_reth_block(&self) -> Result<BlockWithSenders, alloy_primitives::SignatureError> {
        let header = self.build_alloy_header();
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

/// BlockWitnessCodeExt trait
pub trait BlockWitnessCodeExt {
    /// Import codes into code db
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, code_db: CodeDb);
}

impl<T: BlockWitness> BlockWitnessCodeExt for T {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.codes_iter() {
            let code = code.as_ref();
            let code_hash = cycle_track!(keccak256(code), "keccak256");
            code_db.or_insert_with(code_hash, || Bytes::copy_from_slice(code))
        }
    }
}

impl<T: BlockWitness> BlockWitnessCodeExt for [T] {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.iter().flat_map(|w| w.codes_iter()) {
            let code = code.as_ref();
            let code_hash = cycle_track!(keccak256(code), "keccak256");
            code_db.or_insert_with(code_hash, || Bytes::copy_from_slice(code))
        }
    }
}

/// BlockWitnessBlockHashExt trait
pub trait BlockWitnessBlockHashExt {
    /// Import block hashes into block hash provider
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        block_hashes: BlockHashProvider,
    );
}

impl<T: BlockWitness> BlockWitnessBlockHashExt for T {
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        mut block_hashes: BlockHashProvider,
    ) {
        let block_number = self.header().number();
        for (i, hash) in self.block_hashes_iter().enumerate() {
            let block_number = block_number
                .checked_sub(i as u64 + 1)
                .expect("block number underflow");
            block_hashes.insert(block_number, hash)
        }
    }
}

impl<T: BlockWitness> BlockWitnessBlockHashExt for [T] {
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        mut block_hashes: BlockHashProvider,
    ) {
        for witness in self.iter() {
            let block_number = witness.header().number();
            for (i, hash) in witness.block_hashes_iter().enumerate() {
                let block_number = block_number
                    .checked_sub(i as u64 + 1)
                    .expect("block number underflow");
                block_hashes.insert(block_number, hash)
            }
        }
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

impl BlockChunkExt for Block {
    #[inline]
    fn hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher) {
        hasher.update(&self.number.to_be_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.update(
            &U256::from_limbs([self.base_fee_per_gas.unwrap_or_default(), 0, 0, 0])
                .to_be_bytes::<{ U256::BYTES }>(),
        );
        hasher.update(&self.gas_limit.to_be_bytes());
        hasher.update(&(self.body.transactions.len() as u16).to_be_bytes()); // FIXME: l1 tx could be skipped, the actual tx count needs to be calculated
    }
    #[inline]
    fn hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher) {
        for tx in self.body.transactions.iter().filter(|tx| tx.ty() == 0x7e) {
            hasher.update(tx.hash().as_slice())
        }
    }
}
