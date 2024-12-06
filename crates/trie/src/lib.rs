use alloy_rlp::Decodable;
use alloy_trie::{
    nodes::{TrieNode, CHILD_INDEX_RANGE},
    Nibbles,
};
use reth_trie_sparse::RevealedSparseTrie;
use sbv_kv::KeyValueStore;
use sbv_primitives::{keccak256, B256};

/// Fill a KeyValueStore<B256, TrieNode> from a list of nodes
pub fn decode_nodes<B: AsRef<[u8]>, S: KeyValueStore<B256, TrieNode>, I: Iterator<Item = B>>(
    store: &mut S,
    iter: I,
) -> Result<(), alloy_rlp::Error> {
    for byte in iter {
        let mut buf = byte.as_ref();
        let node_hash = keccak256(buf);
        let node = TrieNode::decode(&mut buf)?;
        assert!(
            buf.is_empty(),
            "the rlp buffer should only contains the node"
        );
        store.insert(node_hash, node);
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub struct PartialTrie {
    inner: RevealedSparseTrie,
}

impl PartialTrie {
    pub fn open<S: KeyValueStore<B256, TrieNode>>(store: &S, root: TrieNode) {
        let mut trie = RevealedSparseTrie::from_root(root.clone()).unwrap();
        // traverse the partial trie
        traverse_import_partial_trie(&Nibbles::default(), &root, store, &mut trie);
    }
}

fn traverse_import_partial_trie<S: KeyValueStore<B256, TrieNode>>(
    path: &Nibbles,
    node: &TrieNode,
    nodes: &S,
    trie: &mut RevealedSparseTrie,
) {
    trie.reveal_node(path.clone(), node.clone()).unwrap();

    if let TrieNode::Branch(branch) = node {
        let mut stack_ptr = branch.as_ref().first_child_index();
        for idx in CHILD_INDEX_RANGE {
            if branch.state_mask.is_bit_set(idx) {
                let mut child_path = path.clone();
                child_path.push(idx);
                let child = &branch.stack[stack_ptr];
                stack_ptr += 1;

                let child_node = if child.len() == B256::len_bytes() + 1 {
                    let hash = B256::from_slice(&child[1..]);

                    match nodes.get(&hash) {
                        Some(node) => node.into_owned(),
                        // the node is not in the witness
                        None => continue,
                    }
                } else {
                    let mut buf = child.as_ref();
                    let child = TrieNode::decode(&mut buf).unwrap();
                    assert!(buf.is_empty());

                    child
                };

                traverse_import_partial_trie(&child_path, &child_node, nodes, trie);

                trie.reveal_node(child_path, child_node).unwrap();
            }
        }
    }
}
