#[cfg(feature = "scroll")]
mod scroll;
#[cfg(feature = "scroll")]
pub use scroll::TxL1Msg;

mod access_list;
mod block_header;
mod signature;
mod transaction;
mod witness;

pub use access_list::{AccessList, AccessListItem, ArchivedAccessList, ArchivedAccessListItem};
pub use block_header::{ArchivedBlockHeader, BlockHeader};
pub use signature::{ArchivedSignature, Signature};
pub use transaction::{ArchivedTransaction, Transaction, TypedTransaction};
pub use witness::{ArchivedBlockWitness, BlockWitness};

#[cfg(test)]
mod test {
    use super::*;
    use alloy_rpc_types_debug::ExecutionWitness;
    use alloy_rpc_types_eth::Block;

    const BLOCK_2BA60C: &str =
        include_str!("../../../../testdata/holesky_witness/0x2ba60c/block.json");
    const BLOCK_2BA60D: &str =
        include_str!("../../../../testdata/holesky_witness/0x2ba60d/block.json");
    const WITNESS_2BA60C: &str =
        include_str!("../../../../testdata/holesky_witness/0x2ba60c/witness.json");
    const WITNESS_2BA60D: &str =
        include_str!("../../../../testdata/holesky_witness/0x2ba60d/witness.json");

    #[test]
    fn test_deserialize_block() {
        serde_json::from_str::<Block>(BLOCK_2BA60C).unwrap();
        serde_json::from_str::<Block>(BLOCK_2BA60D).unwrap();
        serde_json::from_str::<ExecutionWitness>(WITNESS_2BA60C).unwrap();
        serde_json::from_str::<ExecutionWitness>(WITNESS_2BA60D).unwrap();
    }

    #[test]
    fn test_build() {
        let block = serde_json::from_str::<Block>(BLOCK_2BA60D).unwrap();
        let prev_state_root = serde_json::from_str::<Block>(BLOCK_2BA60C)
            .unwrap()
            .header
            .state_root;
        let witness = serde_json::from_str::<ExecutionWitness>(WITNESS_2BA60D).unwrap();

        let block_witness = BlockWitness::new_from_block(block, prev_state_root, witness);
        test_block_witness(block_witness);
    }

    fn test_block_witness<B: crate::BlockWitness>(_block_witness: B) {}
}
