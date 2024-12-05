//! Stateless Block Verifier primitives library.

/// Predeployed contracts
#[cfg(feature = "scroll")]
pub mod predeployed;
/// Types definition
pub mod types;

pub use alloy_consensus;
pub use alloy_consensus::Transaction;
pub use alloy_primitives;
pub use alloy_primitives::{Address, B256, U256};

/// Blanket trait for block trace extensions.
pub trait Block: std::fmt::Debug {
    /// transaction type
    type Tx: Transaction;

    /// Hash of the block
    fn block_hash(&self) -> B256;
    /// State root hash
    fn state_root(&self) -> B256;
    /// Difficulty
    fn difficulty(&self) -> U256;
    /// Block number
    fn number(&self) -> u64;
    /// Gas Limit
    fn gas_limit(&self) -> u64;
    /// Gas Used
    fn gas_used(&self) -> u64;
    /// Timestamp
    fn timestamp(&self) -> u64;
    /// prevRandao
    ///
    /// Before the merge this proves, combined with the nonce, that a sufficient amount of
    /// computation has been carried out on this block: the Proof-of-Work (PoF).
    ///
    /// After the merge this is `prevRandao`: Randomness value for the generated payload.
    ///
    /// This is an Option because it is not always set by non-ethereum networks.
    ///
    /// See also <https://eips.ethereum.org/EIPS/eip-4399>
    /// And <https://github.com/ethereum/execution-apis/issues/328>
    fn prevrandao(&self) -> Option<B256>;
    /// Base fee per unit of gas (if past London)
    fn base_fee_per_gas(&self) -> Option<u64>;
    /// Withdrawals root hash added by EIP-4895 and is ignored in legacy headers.
    fn withdraw_root(&self) -> B256;
    /// Block Transactions
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx>;
    /// Number of transactions
    fn num_txs(&self) -> usize;
}

#[cfg(feature = "scroll")]
pub trait BlockScrollExt: Block {
    /// start l1 queue index
    fn start_l1_queue_index(&self) -> u64;

    /// Number of l1 transactions
    #[inline]
    fn num_l1_txs(&self) -> u64 {
        // 0x7e is l1 tx
        match self
            .transactions()
            .filter(|tx| tx.is_l1_tx())
            // tx.nonce for l1 tx is the l1 queue index, which is a globally index,
            // not per user as suggested by the name...
            .map(|tx| tx.nonce())
            .max()
        {
            None => 0, // not l1 tx in this block
            Some(end_l1_queue_index) => end_l1_queue_index - self.start_l1_queue_index() + 1,
        }
    }

    /// Number of l2 transactions
    #[inline]
    fn num_l2_txs(&self) -> u64 {
        // 0x7e is l1 tx
        self.transactions().filter(|tx| !tx.is_l1_tx()).count() as u64
    }

    /// Hash the header of the block
    #[inline]
    fn hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher) {
        let num_txs = (self.num_l1_txs() + self.num_l2_txs()) as u16;
        hasher.update(&self.number().to_be_bytes());
        hasher.update(&self.timestamp().to::<u64>().to_be_bytes());
        hasher.update(
            &self
                .base_fee_per_gas()
                .map(U256::from)
                .unwrap_or_default()
                .to_be_bytes::<{ U256::BYTES }>(),
        );
        hasher.update(&self.gas_limit().to::<u64>().to_be_bytes());
        hasher.update(&num_txs.to_be_bytes());
    }

    /// Hash the l1 messages of the block
    #[inline]
    fn hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher) {
        for tx_hash in self
            .transactions()
            .filter(|tx| tx.is_l1_tx())
            .map(|tx| tx.tx_hash())
        {
            hasher.update(tx_hash.as_slice())
        }
    }
}

impl<T: Block> Block for &T {
    type Tx = T::Tx;

    fn block_hash(&self) -> B256 {
        (*self).block_hash()
    }
    fn state_root(&self) -> B256 {
        (*self).state_root()
    }
    fn difficulty(&self) -> U256 {
        (*self).difficulty()
    }
    fn number(&self) -> u64 {
        (*self).number()
    }
    fn gas_limit(&self) -> u64 {
        (*self).gas_limit()
    }
    fn gas_used(&self) -> u64 {
        (*self).gas_used()
    }
    fn timestamp(&self) -> u64 {
        (*self).timestamp()
    }
    fn prevrandao(&self) -> Option<B256> {
        (*self).prevrandao()
    }
    fn base_fee_per_gas(&self) -> Option<u64> {
        (*self).base_fee_per_gas()
    }
    fn withdraw_root(&self) -> B256 {
        (*self).withdraw_root()
    }
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        (*self).transactions()
    }
    fn num_txs(&self) -> usize {
        (*self).num_txs()
    }
}
