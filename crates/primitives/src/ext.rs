use crate::{B256, BlockWitness, Bytes, keccak256};
#[cfg(feature = "scroll")]
use itertools::Itertools;
use sbv_helpers::cycle_track;
use sbv_kv::KeyValueStore;

/// BlockWitnessCodeExt trait
#[cfg(feature = "scroll")]
pub trait BlockWitnessChunkExt {
    /// Get the chain id.
    fn chain_id(&self) -> crate::ChainId;
    /// Get the previous state root.
    fn prev_state_root(&self) -> B256;
    /// Check if all witnesses have the same chain id.
    fn has_same_chain_id(&self) -> bool;
    /// Check if all witnesses have a sequence block number.
    fn has_seq_block_number(&self) -> bool;
}

#[cfg(feature = "scroll")]
impl<T: BlockWitness> BlockWitnessChunkExt for [T] {
    #[inline(always)]
    fn chain_id(&self) -> crate::ChainId {
        debug_assert!(self.has_same_chain_id(), "chain id mismatch");
        self.first().expect("empty witnesses").chain_id()
    }
    #[inline(always)]
    fn prev_state_root(&self) -> B256 {
        self.first().expect("empty witnesses").pre_state_root()
    }
    #[inline(always)]
    fn has_same_chain_id(&self) -> bool {
        self.iter()
            .tuple_windows()
            .all(|(a, b)| a.chain_id() == b.chain_id())
    }
    #[inline(always)]
    fn has_seq_block_number(&self) -> bool {
        self.iter()
            .tuple_windows()
            .all(|(a, b)| a.number() + 1 == b.number())
    }
}

/// BlockWitnessExt trait
pub trait BlockWitnessExt {
    /// Import codes into code db
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, code_db: CodeDb);
    /// Import block hashes into block hash provider
    #[cfg(not(feature = "scroll"))]
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        block_hashes: BlockHashProvider,
    );
}

impl<T: BlockWitness> BlockWitnessExt for T {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.codes_iter() {
            let code = code.as_ref();
            let code_hash = cycle_track!(keccak256(code), "keccak256");
            code_db.or_insert_with(code_hash, || Bytes::copy_from_slice(code))
        }
    }

    #[cfg(not(feature = "scroll"))]
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        mut block_hashes: BlockHashProvider,
    ) {
        let block_number = self.number();
        for (i, hash) in self.block_hashes_iter().enumerate() {
            let block_number = block_number
                .checked_sub(i as u64 + 1)
                .expect("block number underflow");
            block_hashes.insert(block_number, hash)
        }
    }
}

impl<T: BlockWitness> BlockWitnessExt for [T] {
    fn import_codes<CodeDb: KeyValueStore<B256, Bytes>>(&self, mut code_db: CodeDb) {
        for code in self.iter().flat_map(|w| w.codes_iter()) {
            let code = code.as_ref();
            let code_hash = cycle_track!(keccak256(code), "keccak256");
            code_db.or_insert_with(code_hash, || Bytes::copy_from_slice(code))
        }
    }

    #[cfg(not(feature = "scroll"))]
    fn import_block_hashes<BlockHashProvider: KeyValueStore<u64, B256>>(
        &self,
        mut block_hashes: BlockHashProvider,
    ) {
        for witness in self.iter() {
            let block_number = witness.number();
            for (i, hash) in witness.block_hashes_iter().enumerate() {
                let block_number = block_number
                    .checked_sub(i as u64 + 1)
                    .expect("block number underflow");
                block_hashes.insert(block_number, hash)
            }
        }
    }
}

/// Helper trait for hashing transaction bytes.
#[cfg(feature = "scroll")]
pub trait TxBytesHashExt {
    /// Hash the transaction bytes.
    fn tx_bytes_hash(self) -> (usize, B256);

    /// Hash the transaction bytes.
    fn tx_bytes_hash_in(self, rlp_buffer: &mut Vec<u8>) -> (usize, B256);
}

#[cfg(all(feature = "scroll", feature = "eips"))]
impl<'a, I: IntoIterator<Item = &'a Tx>, Tx: crate::types::eips::eip2718::Encodable2718 + 'a>
    TxBytesHashExt for I
{
    fn tx_bytes_hash(self) -> (usize, B256) {
        let mut rlp_buffer = Vec::new();
        self.tx_bytes_hash_in(&mut rlp_buffer)
    }

    fn tx_bytes_hash_in(self, rlp_buffer: &mut Vec<u8>) -> (usize, B256) {
        use tiny_keccak::{Hasher, Keccak};
        let mut tx_bytes_hasher = Keccak::v256();
        let mut len = 0;
        for tx in self.into_iter() {
            tx.encode_2718(rlp_buffer);
            len += rlp_buffer.len();
            tx_bytes_hasher.update(rlp_buffer);
            rlp_buffer.clear();
        }
        let mut tx_bytes_hash = B256::ZERO;
        tx_bytes_hasher.finalize(&mut tx_bytes_hash.0);
        (len, tx_bytes_hash)
    }
}

/// Chunk related extension methods for Block
#[cfg(feature = "scroll-reth-types")]
pub trait BlockChunkExt {
    /// Hash the header of the block
    fn legacy_hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher);
    /// Hash the l1 messages of the block
    fn legacy_hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher);
    /// Hash the l1 messages of the block
    fn hash_msg_queue(&self, initial_queue_hash: &B256) -> B256;
    /// Number of L1 msg txs in the block
    fn num_l1_msgs(&self) -> usize;
}

#[cfg(feature = "scroll-reth-types")]
impl BlockChunkExt for crate::types::reth::RecoveredBlock<crate::types::reth::Block> {
    #[inline]
    fn legacy_hash_da_header(&self, hasher: &mut impl tiny_keccak::Hasher) {
        use crate::U256;

        hasher.update(&self.number.to_be_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.update(
            &U256::from_limbs([self.base_fee_per_gas.unwrap_or_default(), 0, 0, 0])
                .to_be_bytes::<{ U256::BYTES }>(),
        );
        hasher.update(&self.gas_limit.to_be_bytes());
        // FIXME: l1 tx could be skipped, the actual tx count needs to be calculated
        hasher.update(&(self.body().transactions.len() as u16).to_be_bytes());
    }

    #[inline]
    fn legacy_hash_l1_msg(&self, hasher: &mut impl tiny_keccak::Hasher) {
        use reth_primitives_traits::SignedTransaction;
        for tx in self
            .body()
            .transactions
            .iter()
            .filter(|tx| tx.is_l1_message())
        {
            hasher.update(tx.tx_hash().as_slice())
        }
    }

    #[inline]
    fn hash_msg_queue(&self, initial_queue_hash: &B256) -> B256 {
        use reth_primitives_traits::SignedTransaction;
        use tiny_keccak::Hasher;

        let mut rolling_hash = *initial_queue_hash;
        for tx in self
            .body()
            .transactions
            .iter()
            .filter(|tx| tx.is_l1_message())
        {
            let mut hasher = tiny_keccak::Keccak::v256();
            hasher.update(rolling_hash.as_slice());
            hasher.update(tx.tx_hash().as_slice());

            hasher.finalize(rolling_hash.as_mut_slice());

            // clear last 32 bits, i.e. 4 bytes.
            // https://github.com/scroll-tech/da-codec/blob/26dc8d575244560611548fada6a3a2745c60fe83/encoding/da.go#L817-L825
            // see also https://github.com/scroll-tech/da-codec/pull/42
            rolling_hash.0[28] = 0;
            rolling_hash.0[29] = 0;
            rolling_hash.0[30] = 0;
            rolling_hash.0[31] = 0;
        }

        rolling_hash
    }

    #[inline]
    fn num_l1_msgs(&self) -> usize {
        self.body()
            .transactions
            .iter()
            .filter(|tx| tx.is_l1_message())
            .count()
    }
}
