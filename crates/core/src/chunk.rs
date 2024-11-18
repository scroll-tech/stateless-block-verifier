use revm::primitives::B256;
use sbv_primitives::zk_trie::db::NodeDb;
use sbv_primitives::{zk_trie::db::kv::HashMapDb, Block};
use tiny_keccak::{Hasher, Keccak};

/// A chunk is a set of continuous blocks.
/// ChunkInfo is metadata of chunk, with following fields:
/// - state root before this chunk
/// - state root after this chunk
/// - the withdraw root after this chunk
/// - the data hash of this chunk
/// - the tx data hash of this chunk
/// - flattened L2 tx bytes hash
#[derive(Debug)]
pub struct ChunkInfo {
    chain_id: u64,
    prev_state_root: B256,
    post_state_root: B256,
    withdraw_root: B256,
    data_hash: B256,
}

impl ChunkInfo {
    /// Construct by block traces
    pub fn from_block_traces<T: Block>(traces: &[T]) -> (Self, NodeDb<HashMapDb>) {
        let chain_id = traces.first().unwrap().chain_id();
        let prev_state_root = traces
            .first()
            .expect("at least 1 block needed")
            .root_before();
        let post_state_root = traces.last().expect("at least 1 block needed").root_after();
        let withdraw_root = traces.last().unwrap().withdraw_root();

        cycle_tracker_start!("Keccak::v256");
        let mut data_hasher = Keccak::v256();
        for trace in traces.iter() {
            trace.hash_da_header(&mut data_hasher);
        }
        for trace in traces.iter() {
            trace.hash_l1_msg(&mut data_hasher);
        }
        let mut data_hash = B256::ZERO;
        data_hasher.finalize(&mut data_hash.0);
        cycle_tracker_end!("Keccak::v256");

        let mut zktrie_db = NodeDb::new(HashMapDb::default());
        cycle_tracker_start!("Block::build_zktrie_db");
        for trace in traces.iter() {
            measure_duration_millis!(
                build_zktrie_db_duration_milliseconds,
                trace.build_zktrie_db(&mut zktrie_db).unwrap()
            );
        }
        cycle_tracker_end!("Block::build_zktrie_db");

        let info = ChunkInfo {
            chain_id,
            prev_state_root,
            post_state_root,
            withdraw_root,
            data_hash,
        };

        (info, zktrie_db)
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
    pub fn public_input_hash(&self, tx_bytes_hash: &B256) -> B256 {
        let mut hasher = Keccak::v256();

        hasher.update(&self.chain_id.to_be_bytes());
        hasher.update(self.prev_state_root.as_ref());
        hasher.update(self.post_state_root.as_slice());
        hasher.update(self.withdraw_root.as_slice());
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

    /// Withdraw root after this chunk
    pub fn withdraw_root(&self) -> B256 {
        self.withdraw_root
    }

    /// Data hash of this chunk
    pub fn data_hash(&self) -> B256 {
        self.data_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockExecutionResult, EvmExecutorBuilder, HardforkConfig};
    use revm::primitives::b256;
    use sbv_primitives::types::BlockTrace;

    const TRACES_STR: [&str; 4] = [
        include_str!("../../../testdata/mainnet_blocks/8370400.json"),
        include_str!("../../../testdata/mainnet_blocks/8370401.json"),
        include_str!("../../../testdata/mainnet_blocks/8370402.json"),
        include_str!("../../../testdata/mainnet_blocks/8370403.json"),
    ];

    #[test]
    fn test_public_input_hash() {
        let traces: [BlockTrace; 4] = TRACES_STR.map(|s| {
            #[derive(serde::Deserialize)]
            pub struct BlockTraceJsonRpcResult {
                pub result: BlockTrace,
            }
            serde_json::from_str::<BlockTraceJsonRpcResult>(s)
                .unwrap()
                .result
        });

        let fork_config = HardforkConfig::default_from_chain_id(traces[0].chain_id());
        let (chunk_info, mut zktrie_db) = ChunkInfo::from_block_traces(&traces);
        let mut code_db = HashMapDb::default();

        let mut tx_bytes_hasher = Keccak::v256();

        let mut executor = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
            .hardfork_config(fork_config)
            .chain_id(traces[0].chain_id())
            .build(traces[0].root_before())
            .unwrap();
        for trace in traces.iter() {
            executor.insert_codes(trace).unwrap();
        }

        for trace in traces.iter() {
            let BlockExecutionResult { tx_rlps, .. } = executor.handle_block(trace).unwrap();
            for tx_rlp in tx_rlps {
                tx_bytes_hasher.update(&tx_rlp);
            }
        }

        let post_state_root = executor.commit_changes().unwrap();
        assert_eq!(post_state_root, chunk_info.post_state_root);
        drop(executor); // drop executor to release Rc<Keccek>

        let mut tx_bytes_hash = B256::ZERO;
        tx_bytes_hasher.finalize(&mut tx_bytes_hash.0);
        let public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash);
        assert_eq!(
            public_input_hash,
            b256!("764bffabc9fd4227d447a46d8bb04e5448ed64d89d6e5f4215fcf3593e00f109")
        );
    }
}
