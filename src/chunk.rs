use crate::BlockTraceExt;
use eth_types::H256;
use mpt_zktrie::ZktrieState;
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
    prev_state_root: H256,
    post_state_root: H256,
    withdraw_root: H256,
    data_hash: H256,
}

impl ChunkInfo {
    /// Construct by block traces
    pub fn from_block_traces<T: BlockTraceExt>(traces: &[T]) -> (Self, ZktrieState) {
        let chain_id = traces.first().unwrap().chain_id();
        let prev_state_root = traces
            .first()
            .expect("at least 1 block needed")
            .root_before();
        let post_state_root = traces.last().expect("at least 1 block needed").root_after();
        let withdraw_root = traces.last().unwrap().withdraw_root();

        let mut data_hasher = Keccak::v256();
        for trace in traces.iter() {
            trace.hash_da_header(&mut data_hasher);
        }
        for trace in traces.iter() {
            trace.hash_l1_msg(&mut data_hasher);
        }
        let mut data_hash = H256::zero();
        data_hasher.finalize(&mut data_hash.0);

        let mut zktrie_state = ZktrieState::construct(prev_state_root);
        for trace in traces.iter() {
            trace.build_zktrie_state(&mut zktrie_state);
        }

        let info = ChunkInfo {
            chain_id,
            prev_state_root,
            post_state_root,
            withdraw_root,
            data_hash,
        };

        (info, zktrie_state)
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
    pub fn public_input_hash(&self, tx_bytes_hash: &H256) -> H256 {
        let mut hasher = Keccak::v256();

        hasher.update(&self.chain_id.to_be_bytes());
        hasher.update(self.prev_state_root.as_ref());
        hasher.update(self.post_state_root.as_bytes());
        hasher.update(self.withdraw_root.as_bytes());
        hasher.update(self.data_hash.as_bytes());
        hasher.update(tx_bytes_hash.as_bytes());

        let mut public_input_hash = H256::zero();
        hasher.finalize(&mut public_input_hash.0);
        public_input_hash
    }

    /// Chain ID of this chunk
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// State root before this chunk
    pub fn prev_state_root(&self) -> H256 {
        self.prev_state_root
    }

    /// State root after this chunk
    pub fn post_state_root(&self) -> H256 {
        self.post_state_root
    }

    /// Withdraw root after this chunk
    pub fn withdraw_root(&self) -> H256 {
        self.withdraw_root
    }

    /// Data hash of this chunk
    pub fn data_hash(&self) -> H256 {
        self.data_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvmExecutorBuilder, HardforkConfig};
    use eth_types::l2_types::BlockTrace;
    use std::cell::RefCell;
    use std::rc::Rc;

    const TRACES_STR: [&str; 4] = [
        include_str!("../testdata/mainnet_blocks/8370400.json"),
        include_str!("../testdata/mainnet_blocks/8370401.json"),
        include_str!("../testdata/mainnet_blocks/8370402.json"),
        include_str!("../testdata/mainnet_blocks/8370403.json"),
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

        let fork_config = HardforkConfig::default_from_chain_id(traces[0].chain_id);
        let (chunk_info, zktrie_state) = ChunkInfo::from_block_traces(&traces);

        let tx_bytes_hasher = Rc::new(RefCell::new(Keccak::v256()));

        let mut executor = EvmExecutorBuilder::new()
            .hardfork_config(fork_config)
            .with_execute_hooks(|hooks| {
                let hasher = tx_bytes_hasher.clone();
                hooks.add_tx_rlp_handler(move |_, rlp| {
                    hasher.borrow_mut().update(rlp);
                });
            })
            .zktrie_state(&zktrie_state)
            .build(&traces[0]);
        executor.handle_block(&traces[0]).unwrap();

        for trace in traces[1..].iter() {
            executor.update_db(trace, &zktrie_state);
            executor.handle_block(trace).unwrap();
        }

        let post_state_root = executor.commit_changes();
        assert_eq!(post_state_root, chunk_info.post_state_root);
        drop(executor); // drop executor to release Rc<Keccek>

        let mut tx_bytes_hash = H256::zero();
        let hasher = Rc::into_inner(tx_bytes_hasher).unwrap();
        hasher.into_inner().finalize(&mut tx_bytes_hash.0);
        let _public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash);
    }
}
