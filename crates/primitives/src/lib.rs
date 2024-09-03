//! Stateless Block Verifier primitives library.

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate sbv_utils;

use alloy::primitives::SignatureError;
use mpt_zktrie::ZktrieState;
use revm_primitives::{Address, B256, U256};
use std::fmt::Debug;

/// Types definition
pub mod types;

/// Blanket trait for block trace extensions.
pub trait Block: Debug {
    /// transaction type
    type Tx: TxTrace;

    /// Get block number
    fn number(&self) -> u64;

    /// Get block hash
    fn block_hash(&self) -> B256;

    /// Get chain id
    fn chain_id(&self) -> u64;

    /// Get coinbase
    fn coinbase(&self) -> Address;

    /// Get timestamp
    fn timestamp(&self) -> U256;

    /// Get gas limit
    fn gas_limit(&self) -> U256;

    /// Get base fee per gas
    fn base_fee_per_gas(&self) -> Option<U256>;

    /// Get difficulty
    fn difficulty(&self) -> U256;

    /// Get prevrandao
    fn prevrandao(&self) -> Option<B256>;

    /// creates [`revm::primitives::BlockEnv`]
    #[inline]
    fn as_block_env(&self) -> revm_primitives::BlockEnv {
        revm_primitives::BlockEnv {
            number: U256::from_limbs([self.number(), 0, 0, 0]),
            coinbase: self.coinbase(),
            timestamp: self.timestamp(),
            gas_limit: self.gas_limit(),
            basefee: self.base_fee_per_gas().unwrap_or_default(),
            difficulty: self.difficulty(),
            prevrandao: self.prevrandao(),
            blob_excess_gas_and_price: None,
        }
    }

    /// transactions
    fn transactions(&self) -> impl Iterator<Item = &Self::Tx>;

    /// root before
    fn root_before(&self) -> B256;
    /// root after
    fn root_after(&self) -> B256;
    /// withdraw root
    fn withdraw_root(&self) -> B256;
    /// codes
    fn codes(&self) -> impl ExactSizeIterator<Item = &[u8]>;
    /// start l1 queue index
    fn start_l1_queue_index(&self) -> u64;

    /// flatten proofs
    fn flatten_proofs(&self) -> impl Iterator<Item = (&B256, &[u8])>;

    /// Update zktrie state from trace
    #[inline]
    fn build_zktrie_state(&self, zktrie_state: &mut ZktrieState) {
        let zk_db = zktrie_state.expose_db();

        for (k, bytes) in self.flatten_proofs() {
            zk_db.add_node_bytes(bytes, Some(k.as_slice())).unwrap();
        }
    }

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

    /// execution_results
    fn execution_results(&self, _tx_id: usize) -> Option<&eth_types::l2_types::ExecutionResult> {
        None
    }
}

/// Utility trait for transaction trace
pub trait TxTrace: TryInto<types::TypedTransaction, Error = SignatureError> {
    /// Return the hash of the transaction
    fn tx_hash(&self) -> &B256;

    /// Get `nonce`.
    fn nonce(&self) -> u64;

    /// Returns the transaction type
    fn ty(&self) -> u8;

    /// Check if the transaction is an L1 transaction
    fn is_l1_tx(&self) -> bool {
        self.ty() == 0x7e
    }
}

#[cfg(test)]
mod tests {
    use std::array;
    use std::mem::transmute;

    #[test]
    fn test_memory_layout() {
        use eth_types::{ArchivedH160, H160};
        // H160 and ArchivedH160 should have the same memory layout
        assert_eq!(size_of::<H160>(), 20);
        assert_eq!(size_of::<ArchivedH160>(), 20);
        assert_eq!(size_of::<&[u8; 20]>(), size_of::<usize>());
        assert_eq!(size_of::<&H160>(), size_of::<usize>());
        assert_eq!(size_of::<&ArchivedH160>(), size_of::<usize>());

        let h160 = eth_types::H160::from(array::from_fn(|i| i as u8));
        let serialized = rkyv::to_bytes::<_, 20>(&h160).unwrap();
        let archived: &ArchivedH160 = unsafe { rkyv::archived_root::<H160>(&serialized[..]) };
        assert_eq!(archived.0, h160.0);
        let ptr_to_archived: usize = archived as *const _ as usize;
        let ptr_to_archived_inner: usize = (&archived.0) as *const _ as usize;
        assert_eq!(ptr_to_archived, ptr_to_archived_inner);
        let transmuted: &H160 = unsafe { transmute(archived) };
        assert_eq!(transmuted, &h160);
        let transmuted: &H160 = unsafe { transmute(&archived.0) };
        assert_eq!(transmuted, &h160);
    }
}
