use crate::{
    B256, BlockHeader, Bytes, ChainId, Transaction, Withdrawals, alloy_primitives::map::B256HashMap,
};

/// Represents the execution witness of a block. Contains an optional map of state preimages.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ExecutionWitness {
    /// Map of all hashed trie nodes to their preimages that were required during the execution of
    /// the block, including during state root recomputation.
    ///
    /// `keccak(rlp(node)) => rlp(node)`
    pub state: B256HashMap<Bytes>,
    /// Map of all contract codes (created / accessed) to their preimages that were required during
    /// the execution of the block, including during state root recomputation.
    ///
    /// `keccak(bytecodes) => bytecodes`
    pub codes: B256HashMap<Bytes>,
}

/// Witness for a block.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BlockWitness {
    /// Chain id
    pub chain_id: ChainId,
    /// Block header representation.
    pub header: BlockHeader,
    /// State trie root before the block.
    pub pre_state_root: B256,
    /// Transactions in the block.
    pub transaction: Vec<Transaction>,
    /// Withdrawals in the block.
    pub withdrawals: Option<Withdrawals>,
    /// Last 256 Ancestor block hashes.
    #[cfg(not(feature = "scroll"))]
    pub block_hashes: Vec<B256>,
    /// Rlp encoded state trie nodes.
    pub states: Vec<Bytes>,
    /// Code bytecodes
    pub codes: Vec<Bytes>,
}

impl BlockWitness {
    /// Calculates compression ratios for all transactions in the block witness.
    #[cfg(feature = "scroll-compress-ratio")]
    pub fn compression_ratios(&self) -> Vec<crate::U256> {
        self.transaction
            .iter()
            .map(|tx| crate::evm::compute_compression_ratio(&tx.input))
            .collect()
    }
}

#[cfg(feature = "serde")]
impl BlockWitness {
    /// Deserialize a new `BlockWitness` from a JSON string,
    /// trying to convert from snake_case to camelCase if necessary.
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        if let Ok(raw) = serde_json::from_str::<Self>(s) {
            return Ok(raw);
        }

        Self::from_json_value(serde_json::from_str(s)?)
    }

    /// Creates a new `BlockWitness` from a JSON byte slice,
    /// trying to convert from snake_case to camelCase if necessary.
    pub fn from_json_slice(v: &[u8]) -> Result<Self, serde_json::Error> {
        if let Ok(raw) = serde_json::from_slice::<Self>(v) {
            return Ok(raw);
        }

        Self::from_json_value(serde_json::from_slice(v)?)
    }

    /// Creates a new `BlockWitness` from a JSON value,
    /// trying to convert from snake_case to camelCase if necessary.
    pub fn from_json_value(mut raw: serde_json::Value) -> Result<Self, serde_json::Error> {
        use convert_case::{Case, Casing};

        fn to_camel_recursive(s: &mut serde_json::Value) {
            if let Some(array) = s.as_array_mut() {
                for item in array.iter_mut() {
                    to_camel_recursive(item);
                }
                return;
            }
            let Some(map) = s.as_object_mut() else { return };
            let old = std::mem::take(map);
            for (key, mut v) in old.into_iter() {
                let new_key = key.to_case(Case::Camel);

                if key == "y_parity" || new_key == "yParity" {
                    if let Some(value) = v.as_bool() {
                        v = serde_json::Value::String(
                            if value { "0x1" } else { "0x0" }.to_string(),
                        );
                    }
                }

                to_camel_recursive(&mut v);
                map.insert(new_key, v);
            }
        }

        to_camel_recursive(&mut raw);

        serde_json::from_value(raw)
    }
}

#[cfg(test)]
#[cfg(feature = "serde")]
mod tests {
    use super::*;
    use rstest::rstest;

    #[cfg(not(feature = "scroll"))]
    #[rstest]
    fn test_bincode_serde(
        #[files("../../testdata/holesky_witness/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = BlockWitness::from_json_str(witness_json).unwrap();

        let bincode_serialized =
            bincode::serde::encode_to_vec(&witness, bincode::config::standard()).unwrap();
        let (bincode_deserialized, bytes_read): (BlockWitness, usize) =
            bincode::serde::decode_from_slice(&bincode_serialized, bincode::config::standard())
                .unwrap();
        assert_eq!(witness, bincode_deserialized);
        assert_eq!(bytes_read, bincode_serialized.len());
    }

    #[cfg(feature = "scroll")]
    #[rstest]
    fn test_bincode_serde(
        #[files("../../testdata/scroll_witness/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = BlockWitness::from_json_str(witness_json).unwrap();

        let bincode_serialized =
            bincode::serde::encode_to_vec(&witness, bincode::config::standard()).unwrap();
        let (bincode_deserialized, bytes_read): (BlockWitness, usize) =
            bincode::serde::decode_from_slice(&bincode_serialized, bincode::config::standard())
                .unwrap();
        assert_eq!(witness, bincode_deserialized);
        assert_eq!(bytes_read, bincode_serialized.len());
    }
}
