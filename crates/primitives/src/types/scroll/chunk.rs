//! Chunk related types
use crate::{B256, U256};
use tiny_keccak::{Hasher, Keccak};

/// ChunkInfo is metadata of chunk.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ChunkInfo {
    /// ChunkInfo before EuclidV2 hardfork
    Legacy(LegacyChunkInfo),
    /// ChunkInfo after EuclidV2 hardfork
    EuclidV2(EuclidV2ChunkInfo),
}

/// ChunkInfo before EuclidV2 hardfork
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LegacyChunkInfo {
    /// The EIP-155 chain ID for all txs in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The EIP-155 chain ID for all txs in the chunk."))
    )]
    pub chain_id: u64,
    /// The state root before applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The state root before applying the chunk."))
    )]
    pub prev_state_root: B256,
    /// The state root after applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The state root after applying the chunk."))
    )]
    pub post_state_root: B256,
    /// The withdrawals root after applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The withdrawals root after applying the chunk."))
    )]
    pub withdraw_root: B256,
    /// Digest of L1 message txs force included in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Digest of L1 message txs force included in the chunk."))
    )]
    pub data_hash: B256,
    /// Digest of L2 tx data flattened over all L2 txs in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Digest of L2 tx data flattened over all L2 txs in the chunk."))
    )]
    pub tx_data_digest: B256,
}

/// ChunkInfo after EuclidV2 hardfork
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EuclidV2ChunkInfo {
    /// The EIP-155 chain ID for all txs in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The EIP-155 chain ID for all txs in the chunk."))
    )]
    pub chain_id: u64,
    /// The state root before applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The state root before applying the chunk."))
    )]
    pub prev_state_root: B256,
    /// The state root after applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The state root after applying the chunk."))
    )]
    pub post_state_root: B256,
    /// The withdrawals root after applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The withdrawals root after applying the chunk."))
    )]
    pub withdraw_root: B256,
    /// length of L2 tx data (rlp encoded) flattened over all L2 txs in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Digest of L2 tx data flattened over all L2 txs in the chunk."))
    )]
    pub tx_data_length: usize,
    /// Digest of L2 tx data flattened over all L2 txs in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Digest of L2 tx data flattened over all L2 txs in the chunk."))
    )]
    pub tx_data_digest: B256,
    /// Rolling hash of message queue before applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Rolling hash of message queue before applying the chunk."))
    )]
    pub prev_msg_queue_hash: B256,
    /// Rolling hash of message queue after applying the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "Rolling hash of message queue after applying the chunk."))
    )]
    pub post_msg_queue_hash: B256,
    /// The block number of the first block in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The block number of the first block in the chunk."))
    )]
    pub initial_block_number: u64,
    /// The block contexts of the blocks in the chunk.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The block contexts of the blocks in the chunk."))
    )]
    pub block_ctxs: Vec<BlockContextV2>,
}

/// Represents the version 2 of block context.
///
/// The difference between v2 and v1 is that the block number field has been removed since v2.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockContextV2 {
    /// The timestamp of the block.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "The timestamp of the block.")))]
    pub timestamp: u64,
    /// The base fee of the block.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "The base fee of the block.")))]
    pub base_fee: U256,
    /// The gas limit of the block.
    #[cfg_attr(feature = "rkyv", rkyv(attr(doc = "The gas limit of the block.")))]
    pub gas_limit: u64,
    /// The number of transactions in the block, including both L1 msg txs and L2 txs.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(
            doc = "The number of transactions in the block, including both L1 msg txs and L2 txs."
        ))
    )]
    pub num_txs: u16,
    /// The number of L1 msg txs in the block.
    #[cfg_attr(
        feature = "rkyv",
        rkyv(attr(doc = "The number of L1 msg txs in the block."))
    )]
    pub num_l1_msgs: u16,
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

#[cfg(feature = "rkyv")]
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
    ///     tx data digest ||
    ///     prev msg queue hash ||
    ///     post msg queue hash ||
    ///     initial block number ||
    ///     block_ctx for block_ctx in block_ctxs
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
        hasher.update(&self.initial_block_number.to_be_bytes());
        for block_ctx in &self.block_ctxs {
            block_ctx.hash_into(&mut hasher);
        }

        let mut public_input_hash = B256::ZERO;
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }
}

#[cfg(feature = "rkyv")]
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

#[cfg(feature = "rkyv")]
impl ArchivedEuclidV2ChunkInfo {
    /// Public input hash for a given chunk is defined as
    /// ```text
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     tx data digest ||
    ///     prev msg queue hash ||
    ///     post msg queue hash ||
    ///     initial block number ||
    ///     block_ctx for block_ctx in block_ctxs
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
        hasher.update(&self.initial_block_number.to_native().to_be_bytes());
        for block_ctx in self.block_ctxs.iter() {
            block_ctx.hash_into(&mut hasher);
        }

        let mut public_input_hash = B256::ZERO;
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }
}

impl BlockContextV2 {
    /// Number of bytes used to serialise [`BlockContextV2`] into bytes.
    pub const BYTES_SIZE: usize = 52;

    /// Hash the block context into the given hasher.
    pub fn hash_into<H: Hasher>(&self, hasher: &mut H) {
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.update(&self.base_fee.to_be_bytes::<32>());
        hasher.update(&self.gas_limit.to_be_bytes());
        hasher.update(&self.num_txs.to_be_bytes());
        hasher.update(&self.num_l1_msgs.to_be_bytes());
    }

    /// Serialise the block context into bytes.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(Self::BYTES_SIZE);
        vec.extend_from_slice(&self.timestamp.to_be_bytes());
        vec.extend_from_slice(&self.base_fee.to_be_bytes::<32>());
        vec.extend_from_slice(&self.gas_limit.to_be_bytes());
        vec.extend_from_slice(&self.num_txs.to_be_bytes());
        vec.extend_from_slice(&self.num_l1_msgs.to_be_bytes());
        vec
    }
}

impl<T: AsRef<[u8]>> From<T> for BlockContextV2 {
    fn from(bytes: T) -> Self {
        let bytes = bytes.as_ref();

        assert_eq!(bytes.len(), Self::BYTES_SIZE);

        let timestamp = u64::from_be_bytes(bytes[0..8].try_into().expect("should not fail"));
        let base_fee = U256::from_be_slice(&bytes[8..40]);
        let gas_limit = u64::from_be_bytes(bytes[40..48].try_into().expect("should not fail"));
        let num_txs = u16::from_be_bytes(bytes[48..50].try_into().expect("should not fail"));
        let num_l1_msgs = u16::from_be_bytes(bytes[50..52].try_into().expect("should not fail"));

        Self {
            timestamp,
            base_fee,
            gas_limit,
            num_txs,
            num_l1_msgs,
        }
    }
}

#[cfg(feature = "rkyv")]
impl ArchivedBlockContextV2 {
    /// Number of bytes used to serialise [`BlockContextV2`] into bytes.
    pub const BYTES_SIZE: usize = BlockContextV2::BYTES_SIZE;

    /// Hash the block context into the given hasher.
    pub fn hash_into<H: Hasher>(&self, hasher: &mut H) {
        let base_fee: U256 = self.base_fee.into();
        hasher.update(&self.timestamp.to_native().to_be_bytes());
        hasher.update(&base_fee.to_be_bytes::<32>());
        hasher.update(&self.gas_limit.to_native().to_be_bytes());
        hasher.update(&self.num_txs.to_native().to_be_bytes());
        hasher.update(&self.num_l1_msgs.to_native().to_be_bytes());
    }

    /// Serialise the block context into bytes.
    pub fn to_vec(&self) -> Vec<u8> {
        let base_fee: U256 = self.base_fee.into();
        let mut vec = Vec::with_capacity(BlockContextV2::BYTES_SIZE);
        vec.extend_from_slice(&self.timestamp.to_native().to_be_bytes());
        vec.extend_from_slice(&base_fee.to_be_bytes::<32>());
        vec.extend_from_slice(&self.gas_limit.to_native().to_be_bytes());
        vec.extend_from_slice(&self.num_txs.to_native().to_be_bytes());
        vec.extend_from_slice(&self.num_l1_msgs.to_native().to_be_bytes());
        vec
    }
}

#[cfg(test)]
#[cfg(feature = "rkyv")]
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
            initial_block_number: 0,
            block_ctxs: vec![],
        });

        for chunk_info in &[LEGACY, EUCLID_V2] {
            let serialized = rkyv::to_bytes::<rancor::Error>(chunk_info).unwrap();
            let _ = rkyv::access::<ArchivedChunkInfo, rancor::Error>(&serialized[..]).unwrap();
        }
    }
}
