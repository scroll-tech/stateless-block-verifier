use crate::utils::ext::BlockChunkExt;
use eth_types::H256;
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
    pub fn from_block_traces<T: BlockChunkExt>(traces: &[T]) -> Self {
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

        ChunkInfo {
            chain_id,
            prev_state_root,
            post_state_root,
            withdraw_root,
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
        let chunk_info = ChunkInfo::from_block_traces(&traces);

        let tx_bytes_hasher = Rc::new(RefCell::new(Keccak::v256()));

        for trace in traces.iter() {
            EvmExecutorBuilder::new()
                .hardfork_config(fork_config)
                .with_execute_hooks(|hooks| {
                    let hasher = tx_bytes_hasher.clone();
                    hooks.add_tx_rlp_handler(move |_, rlp| {
                        hasher.borrow_mut().update(rlp);
                    });
                })
                .build(trace)
                .handle_block(trace);
        }

        let mut tx_bytes_hash = H256::zero();
        let haser = Rc::into_inner(tx_bytes_hasher).unwrap();
        haser.into_inner().finalize(&mut tx_bytes_hash.0);
        let public_input_hash = chunk_info.public_input_hash(&tx_bytes_hash);

        let aggregator_chunk_info = aggregator::ChunkInfo::from_block_traces(&traces);
        let aggregator_public_input_hash = aggregator_chunk_info.public_input_hash();

        assert_eq!(public_input_hash, aggregator_public_input_hash);
    }
}
