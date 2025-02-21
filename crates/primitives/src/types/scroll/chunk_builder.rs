use crate::{
    B256, BlockChunkExt, RecoveredBlock,
    chainspec::ChainSpec,
    ext::TxBytesHashExt,
    hardforks::ScrollHardforks,
    types::{BlockContextV2, ChunkInfo, EuclidV2ChunkInfo, LegacyChunkInfo, reth::Block},
};
use alloy_primitives::U256;
use itertools::Itertools;
use sbv_helpers::cycle_track;
use tiny_keccak::{Hasher, Keccak};

/// Builder for ChunkInfo
#[derive(Clone, Debug)]
pub struct ChunkInfoBuilder<'a> {
    chain_spec: &'a ChainSpec,
    blocks: &'a [RecoveredBlock<Block>],
    prev_state_root: B256,
    prev_msg_queue_hash: Option<B256>,
}

impl<'a> ChunkInfoBuilder<'a> {
    /// Create a new ChunkInfoBuilder
    pub fn new(
        chain_spec: &'a ChainSpec,
        prev_state_root: B256,
        blocks: &'a [RecoveredBlock<Block>],
    ) -> Self {
        assert!(!blocks.is_empty(), "blocks must not be empty");

        assert!(
            blocks
                .iter()
                .map(|b| chain_spec.is_euclid_v2_active_at_timestamp(b.timestamp))
                .tuple_windows()
                .all(|(a, b)| a == b),
            "all blocks must have the same hardfork enabled"
        );

        ChunkInfoBuilder {
            chain_spec,
            blocks,
            prev_state_root,
            prev_msg_queue_hash: None,
        }
    }

    /// Check if EuclidV2 is enabled on this chunk
    #[inline]
    pub fn is_euclid_v2(&self) -> bool {
        self.chain_spec
            .is_euclid_v2_active_at_timestamp(self.blocks[0].timestamp)
    }

    /// Set the prev msg queue hash
    #[inline]
    pub fn prev_msg_queue_hash(&mut self, prev_msg_queue_hash: B256) -> &mut Self {
        assert!(
            self.is_euclid_v2(),
            "prev_msg_queue_hash is only for EuclidV2"
        );

        self.prev_msg_queue_hash = Some(prev_msg_queue_hash);
        self
    }

    /// Get the previous state root
    #[inline]
    pub fn get_prev_state_root(&self) -> B256 {
        self.prev_state_root
    }

    /// Get the post state root
    #[inline]
    pub fn get_post_state_root(&self) -> B256 {
        self.blocks.last().expect("at least one block").state_root
    }

    /// Build the chunk info
    pub fn build(self, withdraw_root: B256) -> ChunkInfo {
        let chain_id = self.chain_spec.chain.id();
        let prev_state_root = self.get_prev_state_root();
        let post_state_root = self.get_post_state_root();

        let (tx_data_length, tx_data_digest) = self
            .blocks
            .iter()
            .flat_map(|b| b.body().transactions.iter())
            .tx_bytes_hash();

        if self.is_euclid_v2() {
            let initial_block_number = self.blocks.first().expect("at least one block").number;
            let prev_msg_queue_hash = self
                .prev_msg_queue_hash
                .expect("msg queue hash is required");
            let post_msg_queue_hash = cycle_track!(
                {
                    let mut rolling_hash = prev_msg_queue_hash;
                    for block in self.blocks.iter() {
                        rolling_hash = block.hash_msg_queue(&rolling_hash);
                    }
                    rolling_hash
                },
                "Keccak::v256"
            );
            ChunkInfo::EuclidV2(EuclidV2ChunkInfo {
                chain_id,
                prev_state_root,
                post_state_root,
                withdraw_root,
                tx_data_length,
                tx_data_digest,
                prev_msg_queue_hash,
                post_msg_queue_hash,
                initial_block_number,
                block_ctxs: self.blocks.iter().map(BlockContextV2::from_block).collect(),
            })
        } else {
            let data_hash = cycle_track!(
                {
                    let mut data_hasher = Keccak::v256();
                    for block in self.blocks.iter() {
                        block.legacy_hash_da_header(&mut data_hasher);
                    }
                    for block in self.blocks.iter() {
                        block.legacy_hash_l1_msg(&mut data_hasher);
                    }
                    let mut data_hash = B256::ZERO;
                    data_hasher.finalize(&mut data_hash.0);
                    data_hash
                },
                "Keccak::v256"
            );
            ChunkInfo::Legacy(LegacyChunkInfo {
                chain_id,
                prev_state_root,
                post_state_root,
                withdraw_root,
                data_hash,
                tx_data_digest,
            })
        }
    }
}

impl BlockContextV2 {
    fn from_block(block: &RecoveredBlock<Block>) -> Self {
        BlockContextV2 {
            timestamp: block.timestamp,
            base_fee: U256::from_limbs([
                block.base_fee_per_gas.expect("base fee must enabled"),
                0,
                0,
                0,
            ]),
            gas_limit: block.gas_limit,
            num_txs: block.body().transactions.len() as u16,
            num_l1_msgs: block
                .body()
                .transactions
                .iter()
                .filter(|tx| tx.is_l1_message())
                .count() as u16,
        }
    }
}
