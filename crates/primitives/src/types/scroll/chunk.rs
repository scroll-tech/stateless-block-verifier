use crate::{
    B256, BlockChunkExt, RecoveredBlock, chainspec::ChainSpec, ext::TxBytesHashExt,
    types::reth::Block,
};
use sbv_helpers::cycle_track;
use tiny_keccak::{Hasher, Keccak};

/// Builder for ChunkInfo
#[derive(Clone, Debug)]
pub struct ChunkInfoBuilder<'a> {
    chain_spec: &'a ChainSpec,
    blocks: &'a [RecoveredBlock<Block>],
    prev_msg_queue_hash: Option<B256>,
}

/// ChunkInfo is metadata of chunk.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub enum ChunkInfo {
    /// ChunkInfo before EuclidV2 hardfork
    Legacy(LegacyChunkInfo),
    /// ChunkInfo after EuclidV2 hardfork
    EuclidV2(EuclidV2ChunkInfo),
}

/// ChunkInfo before EuclidV2 hardfork
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub struct LegacyChunkInfo {
    /// The EIP-155 chain ID for all txs in the chunk.
    #[rkyv(attr(doc = "The EIP-155 chain ID for all txs in the chunk."))]
    pub chain_id: u64,
    /// The state root before applying the chunk.
    #[rkyv(attr(doc = "The state root before applying the chunk."))]
    pub prev_state_root: B256,
    /// The state root after applying the chunk.
    #[rkyv(attr(doc = "The state root after applying the chunk."))]
    pub post_state_root: B256,
    /// The withdrawals root after applying the chunk.
    #[rkyv(attr(doc = "The withdrawals root after applying the chunk."))]
    pub withdraw_root: B256,
    /// Digest of L1 message txs force included in the chunk.
    #[rkyv(attr(doc = "Digest of L1 message txs force included in the chunk."))]
    pub data_hash: B256,
    /// Digest of L2 tx data flattened over all L2 txs in the chunk.
    #[rkyv(attr(doc = "Digest of L2 tx data flattened over all L2 txs in the chunk."))]
    pub tx_data_digest: B256,
}

/// ChunkInfo after EuclidV2 hardfork
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
#[rkyv(derive(Debug, PartialEq, Eq))]
pub struct EuclidV2ChunkInfo {
    /// The EIP-155 chain ID for all txs in the chunk.
    #[rkyv(attr(doc = "The EIP-155 chain ID for all txs in the chunk."))]
    pub chain_id: u64,
    /// The state root before applying the chunk.
    #[rkyv(attr(doc = "The state root before applying the chunk."))]
    pub prev_state_root: B256,
    /// The state root after applying the chunk.
    #[rkyv(attr(doc = "The state root after applying the chunk."))]
    pub post_state_root: B256,
    /// The withdrawals root after applying the chunk.
    #[rkyv(attr(doc = "The withdrawals root after applying the chunk."))]
    pub withdraw_root: B256,
    /// rlp encoded length of L2 tx data flattened over all L2 txs in the chunk.
    #[rkyv(attr(doc = "Digest of L2 tx data flattened over all L2 txs in the chunk."))]
    pub tx_data_length: usize,
    /// Digest of L2 tx data flattened over all L2 txs in the chunk.
    #[rkyv(attr(doc = "Digest of L2 tx data flattened over all L2 txs in the chunk."))]
    pub tx_data_digest: B256,
    /// Rolling hash of message queue before applying the chunk.
    #[rkyv(attr(doc = "Rolling hash of message queue before applying the chunk."))]
    pub prev_msg_queue_hash: B256,
    /// Rolling hash of message queue after applying the chunk.
    #[rkyv(attr(doc = "Rolling hash of message queue after applying the chunk."))]
    pub post_msg_queue_hash: B256,
}

impl<'a> ChunkInfoBuilder<'a> {
    /// Create a new ChunkInfoBuilder
    pub fn new(chain_spec: &'a ChainSpec, blocks: &'a [RecoveredBlock<Block>]) -> Self {
        assert!(!blocks.is_empty(), "blocks must not be empty");

        ChunkInfoBuilder {
            chain_spec,
            blocks,
            prev_msg_queue_hash: None,
        }
    }

    /// Check if EuclidV2 is enabled on this chunk
    #[inline]
    pub fn is_euclid_v2(&self) -> bool {
        todo!("waiting for reth hardfork implementation")
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
        self.blocks.first().expect("at least one block").state_root
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

impl ChunkInfo {
    /// Get the chain id
    pub fn chain_id(&self) -> u64 {
        match self {
            ChunkInfo::Legacy(info) => info.chain_id,
            ChunkInfo::EuclidV2(info) => info.chain_id,
        }
    }

    /// Get the prev state root
    pub fn prev_state_root(&self) -> B256 {
        match self {
            ChunkInfo::Legacy(info) => info.prev_state_root,
            ChunkInfo::EuclidV2(info) => info.prev_state_root,
        }
    }

    /// Get the post state root
    pub fn post_state_root(&self) -> B256 {
        match self {
            ChunkInfo::Legacy(info) => info.post_state_root,
            ChunkInfo::EuclidV2(info) => info.post_state_root,
        }
    }

    /// Get the withdraw root
    pub fn withdraw_root(&self) -> B256 {
        match self {
            ChunkInfo::Legacy(info) => info.withdraw_root,
            ChunkInfo::EuclidV2(info) => info.withdraw_root,
        }
    }

    /// Get the tx data digest
    pub fn tx_data_digest(&self) -> B256 {
        match self {
            ChunkInfo::Legacy(info) => info.tx_data_digest,
            ChunkInfo::EuclidV2(info) => info.tx_data_digest,
        }
    }

    /// As legacy chunk info
    pub fn as_legacy(&self) -> Option<&LegacyChunkInfo> {
        match self {
            ChunkInfo::Legacy(info) => Some(info),
            _ => None,
        }
    }

    /// As EuclidV2 chunk info
    pub fn as_euclid_v2(&self) -> Option<&EuclidV2ChunkInfo> {
        match self {
            ChunkInfo::EuclidV2(info) => Some(info),
            _ => None,
        }
    }

    /// Into legacy chunk info
    pub fn into_legacy(self) -> Option<LegacyChunkInfo> {
        match self {
            ChunkInfo::Legacy(info) => Some(info),
            _ => None,
        }
    }

    /// Into EuclidV2 chunk info
    pub fn into_euclid_v2(self) -> Option<EuclidV2ChunkInfo> {
        match self {
            ChunkInfo::EuclidV2(info) => Some(info),
            _ => None,
        }
    }

    /// Public input hash for a given chunk is defined as
    ///
    /// - Before EuclidV2:
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     chunk data hash ||
    ///     chunk txdata hash
    /// )
    /// ```
    ///
    /// - After EuclidV2:
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     tx data hash ||
    ///     prev msg queue hash ||
    ///     post msg queue hash
    /// )
    /// ```
    pub fn pi_hash(&self) -> B256 {
        match self {
            ChunkInfo::Legacy(info) => info.pi_hash(),
            ChunkInfo::EuclidV2(info) => info.pi_hash(),
        }
    }
}

impl ArchivedChunkInfo {
    /// Public input hash for a given chunk is defined as
    ///
    /// - Before EuclidV2:
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     chunk data hash ||
    ///     chunk txdata hash
    /// )
    /// ```
    ///
    /// - After EuclidV2:
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     tx data hash ||
    ///     prev msg queue hash ||
    ///     post msg queue hash
    /// )
    /// ```
    pub fn pi_hash(&self) -> B256 {
        match self {
            ArchivedChunkInfo::Legacy(info) => info.pi_hash(),
            ArchivedChunkInfo::EuclidV2(info) => info.pi_hash(),
        }
    }
}

impl LegacyChunkInfo {
    /// Public input hash for a given chunk is defined as
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     chunk data hash ||
    ///     chunk txdata hash
    /// )
    /// ```
    pub fn pi_hash(&self) -> B256 {
        let mut hasher = Keccak::v256();

        hasher.update(&self.chain_id.to_be_bytes());
        hasher.update(self.prev_state_root.as_ref());
        hasher.update(self.post_state_root.as_ref());
        hasher.update(self.withdraw_root.as_ref());
        hasher.update(self.data_hash.as_ref());
        hasher.update(self.tx_data_digest.as_ref());

        let mut public_input_hash = B256::ZERO;
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }
}

impl EuclidV2ChunkInfo {
    /// Public input hash for a given chunk is defined as
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     tx data hash ||
    ///     prev msg queue hash ||
    ///     post msg queue hash
    /// )
    /// ```
    pub fn pi_hash(&self) -> B256 {
        let mut hasher = Keccak::v256();

        hasher.update(&self.chain_id.to_be_bytes());
        hasher.update(self.prev_state_root.as_ref());
        hasher.update(self.post_state_root.as_ref());
        hasher.update(self.withdraw_root.as_ref());
        hasher.update(self.tx_data_digest.as_ref());
        hasher.update(self.prev_msg_queue_hash.as_ref());
        hasher.update(self.post_msg_queue_hash.as_ref());

        let mut public_input_hash = B256::ZERO;
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }
}

impl ArchivedLegacyChunkInfo {
    /// Public input hash for a given chunk is defined as
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     chunk data hash ||
    ///     chunk txdata hash
    /// )
    /// ```
    pub fn pi_hash(&self) -> B256 {
        let mut hasher = Keccak::v256();

        hasher.update(&self.chain_id.to_native().to_be_bytes());
        hasher.update(self.prev_state_root.0.as_ref());
        hasher.update(self.post_state_root.0.as_ref());
        hasher.update(self.withdraw_root.0.as_ref());
        hasher.update(self.data_hash.0.as_ref());
        hasher.update(self.tx_data_digest.0.as_ref());

        let mut public_input_hash = B256::ZERO;
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }
}

impl ArchivedEuclidV2ChunkInfo {
    /// Public input hash for a given chunk is defined as
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     tx data hash ||
    ///     prev msg queue hash ||
    ///     post msg queue hash
    /// )
    /// ```
    pub fn pi_hash(&self) -> B256 {
        let mut hasher = Keccak::v256();

        hasher.update(&self.chain_id.to_native().to_be_bytes());
        hasher.update(self.prev_state_root.0.as_ref());
        hasher.update(self.post_state_root.0.as_ref());
        hasher.update(self.withdraw_root.0.as_ref());
        hasher.update(self.tx_data_digest.0.as_ref());
        hasher.update(self.prev_msg_queue_hash.0.as_ref());
        hasher.update(self.post_msg_queue_hash.0.as_ref());

        let mut public_input_hash = B256::ZERO;
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rkyv::rancor;

    #[test]
    fn test_rkyv_chunk_info() {
        const LEGACY: ChunkInfo = ChunkInfo::Legacy(LegacyChunkInfo {
            chain_id: 1,
            prev_state_root: B256::new([1; 32]),
            post_state_root: B256::new([2; 32]),
            withdraw_root: B256::new([3; 32]),
            data_hash: B256::new([4; 32]),
            tx_data_digest: B256::new([5; 32]),
        });

        const EUCLID_V2: ChunkInfo = ChunkInfo::EuclidV2(EuclidV2ChunkInfo {
            chain_id: 1,
            prev_state_root: B256::new([1; 32]),
            post_state_root: B256::new([2; 32]),
            withdraw_root: B256::new([3; 32]),
            tx_data_length: 100,
            tx_data_digest: B256::new([5; 32]),
            prev_msg_queue_hash: B256::new([6; 32]),
            post_msg_queue_hash: B256::new([7; 32]),
        });

        for chunk_info in &[LEGACY, EUCLID_V2] {
            let serialized = rkyv::to_bytes::<rancor::Error>(chunk_info).unwrap();
            let _ = rkyv::access::<ArchivedChunkInfo, rancor::Error>(&serialized[..]).unwrap();
        }
    }
}
