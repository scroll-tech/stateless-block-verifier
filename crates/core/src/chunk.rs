use sbv_primitives::{B256, BlockChunkExt, RecoveredBlock, types::reth::Block};
use tiny_keccak::{Hasher, Keccak};

/// A chunk is a set of continuous blocks.
/// ChunkInfo is metadata of chunk, with following fields:
/// - state root before this chunk
/// - state root after this chunk
/// - the withdraw root after this chunk
/// - the data hash of this chunk
/// - the tx data hash of this chunk
/// - flattened L2 tx bytes hash
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChunkInfo {
    chain_id: u64,
    prev_state_root: B256,
    post_state_root: B256,
    data_hash: B256,
}

impl ChunkInfo {
    /// Construct by block traces
    #[must_use]
    pub fn from_blocks(
        chain_id: u64,
        prev_state_root: B256,
        blocks: &[RecoveredBlock<Block>],
    ) -> Self {
        let last_block = blocks.last().expect("at least one block");

        let data_hash = cycle_track!(
            {
                let mut data_hasher = Keccak::v256();
                for block in blocks.iter() {
                    block.hash_da_header(&mut data_hasher);
                }
                for block in blocks.iter() {
                    block.hash_l1_msg(&mut data_hasher);
                }
                let mut data_hash = B256::ZERO;
                data_hasher.finalize(&mut data_hash.0);
                data_hash
            },
            "Keccak::v256"
        );

        ChunkInfo {
            chain_id,
            prev_state_root,
            post_state_root: last_block.state_root,
            data_hash,
        }
    }

    /// Public input hash for a given chunk is defined as
    /// keccak(
    ///     chain id ||
    ///     prev state root ||
    ///     post state root ||
    ///     withdraw root ||
    ///     chunk data hash ||
    ///     chunk txdata hash
    /// )
    pub fn public_input_hash(&self, withdraw_root: &B256, tx_bytes_hash: &B256) -> B256 {
        let mut hasher = Keccak::v256();

        hasher.update(&self.chain_id.to_be_bytes());
        hasher.update(self.prev_state_root.as_ref());
        hasher.update(self.post_state_root.as_slice());
        hasher.update(withdraw_root.as_slice());
        hasher.update(self.data_hash.as_slice());
        hasher.update(tx_bytes_hash.as_slice());

        let mut public_input_hash = B256::ZERO;
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }

    /// Chain ID of this chunk
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// State root before this chunk
    pub fn prev_state_root(&self) -> B256 {
        self.prev_state_root
    }

    /// State root after this chunk
    pub fn post_state_root(&self) -> B256 {
        self.post_state_root
    }

    /// Data hash of this chunk
    pub fn data_hash(&self) -> B256 {
        self.data_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbv_primitives::{BlockWitness as _, RecoveredBlock, types::BlockWitness};

    const TRACES_STR: [&str; 4] = [
        include_str!("../../../testdata/holesky_witness/2971844.json"),
        include_str!("../../../testdata/holesky_witness/2971845.json"),
        include_str!("../../../testdata/holesky_witness/2971846.json"),
        include_str!("../../../testdata/holesky_witness/2971847.json"),
    ];

    #[test]
    fn test_public_input_hash() {
        let witnesses: [BlockWitness; 4] = TRACES_STR.map(|s| serde_json::from_str(s).unwrap());
        let blocks: [RecoveredBlock<Block>; 4] =
            witnesses.clone().map(|s| s.build_reth_block().unwrap());

        let _ = ChunkInfo::from_blocks(1, witnesses[0].pre_state_root, &blocks);
    }
}
