mod access_list;
mod auth_list;
mod block_header;
mod signature;
mod transaction;
mod withdrawal;
mod witness;

pub use access_list::AccessList;
pub use block_header::BlockHeader;
pub use signature::Signature;
pub use transaction::Transaction;
pub use withdrawal::Withdrawal;
pub use witness::BlockWitness;

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[cfg(feature = "scroll")]
    fn serde_scroll_legacy_blocks_roundtrip(
        #[files("../../testdata/legacy/scroll_witness/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let serialized = serde_json::to_string(&witness).unwrap();
        let deserialized: BlockWitness = serde_json::from_str(&serialized).unwrap();
        assert_eq!(witness, deserialized);
    }
}
