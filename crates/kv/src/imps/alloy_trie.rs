use alloy_trie::nodes::TrieNode;

impl crate::Value for TrieNode {
    #[cfg(feature = "sled")]
    fn serialize(&self) -> Vec<u8> {
        todo!()
    }

    #[cfg(feature = "sled")]
    fn deserialize(buf: &[u8]) -> Self {
        todo!()
    }
}
