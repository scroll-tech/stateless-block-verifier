// This code is modified from the original implementation of Zeth.
//
// Reference: https://github.com/risc0/zeth
//
// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use alloy_rlp::{Decodable, EMPTY_STRING_CODE, Encodable, Header};
use alloy_trie::{EMPTY_ROOT_HASH, KECCAK_EMPTY};
use reth_trie::Nibbles;
use sbv_kv::HashMap;
use sbv_primitives::{Address, B256, keccak256};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Mutex;
use std::{
    cmp,
    fmt::{Debug, Write},
    iter, mem,
};

/// Ethereum state trie and account storage tries.
#[derive(Debug, thiserror::Error)]
pub enum FromProofError {
    #[error("Node {} is not found by hash", .0)]
    NodeNotFoundByHash(usize),
    #[error("Node {} refrences invalid successor", .0)]
    NodeHasInvalidSuccessor(usize),
    #[error("Node {} cannot have children and is invalid", .0)]
    NodeCannotHaveChildren(usize),
    #[error("Found mismatched storage root after reconstruction \n account {}, found {}, expected {}", .0, .1, .2)]
    MismatchedStorageRoot(Address, B256, B256),
    #[error("Found mismatched staet root after reconstruction \n found {}, expected {}", .0, .1)]
    MismatchedStateRoot(B256, B256),
    // todo: Should decode return a decoder error?
    #[error("Error decoding proofs from bytes, {}", .0)]
    DecodingError(#[from] Error),
}

pub trait RlpBytes {
    /// Returns the RLP-encoding.
    fn to_rlp(&self) -> Vec<u8>;
}

impl<T> RlpBytes for T
where
    T: Encodable,
{
    #[inline]
    fn to_rlp(&self) -> Vec<u8> {
        let rlp_length = self.length();
        let mut out = Vec::with_capacity(rlp_length);
        self.encode(&mut out);
        debug_assert_eq!(out.len(), rlp_length);
        out
    }
}

/// Represents the root node of a sparse Merkle Patricia Trie.
///
/// The "sparse" nature of this trie allows for truncation of certain unneeded parts,
/// representing them by their node hash. This design choice is particularly useful for
/// optimizing storage. However, operations targeting a truncated part will fail and
/// return an error. Another distinction of this implementation is that branches cannot
/// store values, aligning with the construction of MPTs in Ethereum.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MptNode<'a> {
    /// The type and data of the node.
    data: MptNodeData<'a>,
    /// Cache for a previously computed reference of this node. This is skipped during
    /// serialization.
    #[serde(skip)]
    cached_reference: Mutex<Option<MptNodeReference<'a>>>,
}

impl Ord for MptNode<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.data.cmp(&other.data)
    }
}

impl PartialOrd for MptNode<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for MptNode<'_> {}

impl PartialEq for MptNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl Clone for MptNode<'_> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            cached_reference: Mutex::new(self.cached_reference.lock().unwrap().clone()),
        }
    }
}

/// Represents custom error types for the sparse Merkle Patricia Trie (MPT).
///
/// These errors cover various scenarios that can occur during trie operations, such as
/// encountering unresolved nodes, finding values in branches where they shouldn't be, and
/// issues related to RLP (Recursive Length Prefix) encoding and decoding.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Triggered when an operation reaches an unresolved node. The associated `B256`
    /// value provides details about the unresolved node.
    #[error("reached an unresolved node: {0:#}")]
    NodeNotResolved(B256),
    /// Occurs when a value is unexpectedly found in a branch node.
    #[error("branch node with value")]
    ValueInBranch,
    /// Represents errors related to the RLP encoding and decoding using the `alloy_rlp`
    /// library.
    #[error("RLP error")]
    Rlp(#[from] alloy_rlp::Error),
}

/// Represents the various types of data that can be stored within a node in the sparse
/// Merkle Patricia Trie (MPT).
///
/// Each node in the trie can be of one of several types, each with its own specific data
/// structure. This enum provides a clear and type-safe way to represent the data
/// associated with each node type.
#[derive(Clone, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum MptNodeData<'a> {
    /// Represents an empty trie node.
    #[default]
    Null,
    /// A node that can have up to 16 children. Each child is an optional boxed [MptNode].
    Branch([Option<Box<MptNode<'a>>>; 16]),
    /// A leaf node that contains a key and a value, both represented as byte vectors.
    Leaf(Cow<'a, [u8]>, Cow<'a, [u8]>),
    /// A node that has exactly one child and is used to represent a shared prefix of
    /// several keys.
    Extension(Cow<'a, [u8]>, Box<MptNode<'a>>),
    /// Represents a sub-trie by its hash, allowing for efficient storage of large
    /// sub-tries without storing their entire content.
    Digest(B256),
}

/// Represents the ways in which one node can reference another node inside the sparse
/// Merkle Patricia Trie (MPT).
///
/// Nodes in the MPT can reference other nodes either directly through their byte
/// representation or indirectly through a hash of their encoding. This enum provides a
/// clear and type-safe way to represent these references.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum MptNodeReference<'a> {
    /// Represents a direct reference to another node using its byte encoding. Typically
    /// used for short encodings that are less than 32 bytes in length.
    Bytes(Cow<'a, [u8]>),
    /// Represents an indirect reference to another node using the Keccak hash of its long
    /// encoding. Used for encodings that are not less than 32 bytes in length.
    Digest(B256),
}

/// Provides a conversion from [MptNodeData] to [MptNode].
///
/// This implementation allows for conversion from [MptNodeData] to [MptNode],
/// initializing the `data` field with the provided value and setting the
/// `cached_reference` field to `None`.
impl<'a> From<MptNodeData<'a>> for MptNode<'a> {
    fn from(value: MptNodeData<'a>) -> Self {
        Self {
            data: value,
            cached_reference: Mutex::new(None),
        }
    }
}

/// Provides encoding functionalities for the `MptNode` type.
///
/// This implementation allows for the serialization of an [MptNode] into its RLP-encoded
/// form. The encoding is done based on the type of node data ([MptNodeData]) it holds.
impl Encodable for MptNode<'_> {
    /// Encodes the node into the provided `out` buffer.
    ///
    /// The encoding is done using the Recursive Length Prefix (RLP) encoding scheme. The
    /// method handles different node data types and encodes them accordingly.
    #[inline]
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match &self.data {
            MptNodeData::Null => {
                out.put_u8(EMPTY_STRING_CODE);
            }
            MptNodeData::Branch(nodes) => {
                Header {
                    list: true,
                    payload_length: self.payload_length(),
                }
                .encode(out);
                nodes.iter().for_each(|child| match child {
                    Some(node) => node.reference_encode(out),
                    None => out.put_u8(EMPTY_STRING_CODE),
                });
                // in the MPT reference, branches have values so always add empty value
                out.put_u8(EMPTY_STRING_CODE);
            }
            MptNodeData::Leaf(prefix, value) => {
                Header {
                    list: true,
                    payload_length: self.payload_length(),
                }
                .encode(out);
                prefix.as_ref().encode(out);
                value.as_ref().encode(out);
            }
            MptNodeData::Extension(prefix, node) => {
                Header {
                    list: true,
                    payload_length: self.payload_length(),
                }
                .encode(out);
                prefix.as_ref().encode(out);
                node.reference_encode(out);
            }
            MptNodeData::Digest(digest) => {
                digest.encode(out);
            }
        }
    }

    /// Returns the length of the encoded node in bytes.
    ///
    /// This method calculates the length of the RLP-encoded node. It's useful for
    /// determining the size requirements for storage or transmission.
    #[inline]
    fn length(&self) -> usize {
        let payload_length = self.payload_length();
        payload_length + alloy_rlp::length_of_length(payload_length)
    }
}

/// Represents a node in the sparse Merkle Patricia Trie (MPT).
///
/// The [MptNode] type encapsulates the data and functionalities associated with a node in
/// the MPT. It provides methods for manipulating the trie, such as inserting, deleting,
/// and retrieving values, as well as utility methods for encoding, decoding, and
/// debugging.
impl<'a> MptNode<'a> {
    pub fn decode(buf: &mut &'a [u8]) -> alloy_rlp::Result<Self> {
        let mut items = match Header::decode_raw(buf)? {
            alloy_rlp::PayloadView::List(list) => list,
            alloy_rlp::PayloadView::String(val) => {
                return if val.is_empty() {
                    Ok(MptNodeData::Null.into())
                } else if val.len() == 32 {
                    Ok(MptNodeData::Digest(B256::from_slice(val)).into())
                } else {
                    println!("Invalid digest length: {val:?} {}", val.len());
                    Err(alloy_rlp::Error::Custom("invalid digest"))
                };
            }
        };

        // A valid number of trie node items is either 17 (branch node)
        // or 2 (extension or leaf node).
        match items.len() {
            17 => {
                let mut node_list = Vec::with_capacity(16);
                for item in items.iter().take(16) {
                    if *item == [EMPTY_STRING_CODE] {
                        node_list.push(None);
                    } else {
                        node_list.push(Some(Box::new(MptNode::decode(&mut &**item)?)));
                    }
                }
                if items[16] != [EMPTY_STRING_CODE] {
                    return Err(alloy_rlp::Error::Custom(
                        "branch node values are not supported",
                    ));
                }
                Ok(MptNodeData::Branch(node_list.try_into().unwrap()).into())
            }
            2 => {
                let path = Header::decode_bytes(&mut &*items[0], false)?;
                let prefix = path[0];
                if (prefix & (2 << 4)) == 0 {
                    let node = MptNode::decode(&mut items[1])?;
                    Ok(MptNodeData::extension(path, node).into())
                } else {
                    let value = Header::decode_bytes(&mut &*items[1], false)?;
                    Ok(MptNodeData::leaf(path, value).into())
                }
            }
            _ => Err(alloy_rlp::Error::Custom(
                "invalid number of items in the list",
            )),
        }
    }

    /// Retrieves the underlying data of the node.
    ///
    /// This method provides a reference to the node's data, allowing for inspection and
    /// manipulation.
    #[inline]
    pub fn as_data(&self) -> &MptNodeData<'a> {
        &self.data
    }

    /// Retrieves the [MptNodeReference] reference of the node when it's referenced inside
    /// another node.
    ///
    /// This method provides a way to obtain a compact representation of the node for
    /// storage or transmission purposes.
    #[inline]
    pub fn reference(&self) -> MptNodeReference<'a> {
        self.cached_reference
            .lock()
            .unwrap()
            .get_or_insert_with(|| self.calc_reference())
            .clone()
    }

    /// Computes and returns the 256-bit hash of the node.
    ///
    /// This method provides a unique identifier for the node based on its content.
    #[inline]
    pub fn hash(&self) -> B256 {
        match self.data {
            MptNodeData::Null => EMPTY_ROOT_HASH,
            _ => match self.reference() {
                MptNodeReference::Digest(digest) => digest,
                MptNodeReference::Bytes(bytes) => keccak256(bytes),
            },
        }
    }

    /// Encodes the [MptNodeReference] of this node into the `out` buffer.
    fn reference_encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        match self.reference() {
            // if the reference is an RLP-encoded byte slice, copy it directly
            MptNodeReference::Bytes(bytes) => out.put_slice(&bytes),
            // if the reference is a digest, RLP-encode it with its fixed known length
            MptNodeReference::Digest(digest) => {
                out.put_u8(EMPTY_STRING_CODE + 32);
                out.put_slice(digest.as_slice());
            }
        }
    }

    /// Returns the length of the encoded [MptNodeReference] of this node.
    fn reference_length(&self) -> usize {
        match self.reference() {
            MptNodeReference::Bytes(bytes) => bytes.len(),
            MptNodeReference::Digest(_) => 1 + 32,
        }
    }

    fn calc_reference(&self) -> MptNodeReference<'a> {
        match &self.data {
            MptNodeData::Null => MptNodeReference::bytes(&[EMPTY_STRING_CODE]),
            MptNodeData::Digest(digest) => MptNodeReference::Digest(*digest),
            _ => {
                let encoded = alloy_rlp::encode(self);
                if encoded.len() < 32 {
                    MptNodeReference::bytes(Cow::Owned(encoded))
                } else {
                    MptNodeReference::Digest(keccak256(encoded))
                }
            }
        }
    }

    /// Determines if the trie is empty.
    ///
    /// This method checks if the node represents an empty trie, i.e., it doesn't contain
    /// any key-value pairs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(&self.data, MptNodeData::Null)
    }

    /// Determines if the node represents a digest.
    ///
    /// A digest is a compact representation of a sub-trie, represented by its hash.
    #[inline]
    pub fn is_digest(&self) -> bool {
        matches!(&self.data, MptNodeData::Digest(_))
    }

    /// Retrieves the value associated with a given key in the trie.
    ///
    /// If the key is not present in the trie, this method returns `None`. Otherwise, it
    /// returns a reference to the associated value. If [None] is returned, the key is
    /// provably not in the trie.
    #[inline]
    pub fn get(&self, key: &[u8]) -> Result<Option<&[u8]>, Error> {
        self.get_internal(&to_nibs(key))
    }

    /// Retrieves the RLP-decoded value corresponding to the key.
    ///
    /// If the key is not present in the trie, this method returns `None`. Otherwise, it
    /// returns the RLP-decoded value.
    #[inline]
    pub fn get_rlp<T: Decodable>(&self, key: &[u8]) -> Result<Option<T>, Error> {
        match self.get(key)? {
            Some(mut bytes) => Ok(Some(T::decode(&mut bytes)?)),
            None => Ok(None),
        }
    }

    fn get_internal(&self, key_nibs: &[u8]) -> Result<Option<&[u8]>, Error> {
        match &self.data {
            MptNodeData::Null => Ok(None),
            MptNodeData::Branch(nodes) => {
                if let Some((i, tail)) = key_nibs.split_first() {
                    match nodes[*i as usize] {
                        Some(ref node) => node.get_internal(tail),
                        None => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            MptNodeData::Leaf(prefix, value) => {
                if prefix_nibs(prefix) == key_nibs {
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }
            MptNodeData::Extension(prefix, node) => {
                if let Some(tail) = key_nibs.strip_prefix(prefix_nibs(prefix).as_slice()) {
                    node.get_internal(tail)
                } else {
                    Ok(None)
                }
            }
            MptNodeData::Digest(digest) => Err(Error::NodeNotResolved(*digest)),
        }
    }

    /// Removes a key from the trie.
    ///
    /// This method attempts to remove a key-value pair from the trie. If the key is
    /// present, it returns `true`. Otherwise, it returns `false`.
    #[inline]
    pub fn delete(&mut self, key: &[u8]) -> Result<bool, Error> {
        self.delete_internal(&to_nibs(key))
    }

    fn delete_internal(&mut self, key_nibs: &[u8]) -> Result<bool, Error> {
        match &mut self.data {
            MptNodeData::Null => return Ok(false),
            MptNodeData::Branch(children) => {
                if let Some((i, tail)) = key_nibs.split_first() {
                    let child = &mut children[*i as usize];
                    match child {
                        Some(node) => {
                            if !node.delete_internal(tail)? {
                                return Ok(false);
                            }
                            // if the node is now empty, remove it
                            if node.is_empty() {
                                *child = None;
                            }
                        }
                        None => return Ok(false),
                    }
                } else {
                    return Err(Error::ValueInBranch);
                }

                let mut remaining = children.iter_mut().enumerate().filter(|(_, n)| n.is_some());
                // there will always be at least one remaining node
                let (index, node) = remaining.next().unwrap();
                // if there is only exactly one node left, we need to convert the branch
                if remaining.next().is_none() {
                    let mut orphan = node.take().unwrap();
                    match &mut orphan.data {
                        // if the orphan is a leaf, prepend the corresponding nib to it
                        MptNodeData::Leaf(prefix, orphan_value) => {
                            let new_nibs: Vec<_> =
                                iter::once(index as u8).chain(prefix_nibs(prefix)).collect();
                            self.data = MptNodeData::leaf(
                                to_encoded_path(&new_nibs, true),
                                mem::take(orphan_value),
                            );
                        }
                        // if the orphan is an extension, prepend the corresponding nib to it
                        MptNodeData::Extension(prefix, orphan_child) => {
                            let new_nibs: Vec<_> =
                                iter::once(index as u8).chain(prefix_nibs(prefix)).collect();
                            self.data = MptNodeData::extension(
                                to_encoded_path(&new_nibs, false),
                                mem::take(orphan_child),
                            );
                        }
                        // if the orphan is a branch or digest, convert to an extension
                        MptNodeData::Branch(_) | MptNodeData::Digest(_) => {
                            self.data = MptNodeData::extension(
                                to_encoded_path(&[index as u8], false),
                                orphan,
                            );
                        }
                        MptNodeData::Null => unreachable!(),
                    }
                }
            }
            MptNodeData::Leaf(prefix, _) => {
                if prefix_nibs(prefix) != key_nibs {
                    return Ok(false);
                }
                self.data = MptNodeData::Null;
            }
            MptNodeData::Extension(prefix, child) => {
                let mut self_nibs = prefix_nibs(prefix);
                if let Some(tail) = key_nibs.strip_prefix(self_nibs.as_slice()) {
                    if !child.delete_internal(tail)? {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }

                // an extension can only point to a branch or a digest; since it's sub trie was
                // modified, we need to make sure that this property still holds
                match &mut child.data {
                    // if the child is empty, remove the extension
                    MptNodeData::Null => {
                        self.data = MptNodeData::Null;
                    }
                    // for a leaf, replace the extension with the extended leaf
                    MptNodeData::Leaf(prefix, value) => {
                        self_nibs.extend(prefix_nibs(prefix));
                        self.data =
                            MptNodeData::leaf(to_encoded_path(&self_nibs, true), mem::take(value));
                    }
                    // for an extension, replace the extension with the extended extension
                    MptNodeData::Extension(prefix, node) => {
                        self_nibs.extend(prefix_nibs(prefix));
                        self.data = MptNodeData::extension(
                            to_encoded_path(&self_nibs, false),
                            mem::take(node),
                        );
                    }
                    // for a branch or digest, the extension is still correct
                    MptNodeData::Branch(_) | MptNodeData::Digest(_) => {}
                }
            }
            MptNodeData::Digest(digest) => return Err(Error::NodeNotResolved(*digest)),
        };

        self.invalidate_ref_cache();
        Ok(true)
    }

    /// Inserts an RLP-encoded value into the trie.
    ///
    /// This method inserts a value that's been encoded using RLP into the trie.
    #[inline]
    pub fn insert_rlp(&mut self, key: &[u8], value: impl Encodable) -> Result<bool, Error> {
        self.insert_internal(&to_nibs(key), value.to_rlp())
    }

    fn insert_internal(&mut self, key_nibs: &[u8], value: Vec<u8>) -> Result<bool, Error> {
        match &mut self.data {
            MptNodeData::Null => {
                self.data = MptNodeData::leaf(to_encoded_path(key_nibs, true), value);
            }
            MptNodeData::Branch(children) => {
                if let Some((i, tail)) = key_nibs.split_first() {
                    let child = &mut children[*i as usize];
                    match child {
                        Some(node) => {
                            if !node.insert_internal(tail, value)? {
                                return Ok(false);
                            }
                        }
                        // if the corresponding child is empty, insert a new leaf
                        None => {
                            *child = Some(Box::new(
                                MptNodeData::leaf(to_encoded_path(tail, true), value).into(),
                            ));
                        }
                    }
                } else {
                    return Err(Error::ValueInBranch);
                }
            }
            MptNodeData::Leaf(prefix, old_value) => {
                let self_nibs = prefix_nibs(prefix);
                let common_len = lcp(&self_nibs, key_nibs);
                if common_len == self_nibs.len() && common_len == key_nibs.len() {
                    // if self_nibs == key_nibs, update the value if it is different
                    if old_value == &value {
                        return Ok(false);
                    }
                    *old_value = value.into();
                } else if common_len == self_nibs.len() || common_len == key_nibs.len() {
                    return Err(Error::ValueInBranch);
                } else {
                    let split_point = common_len + 1;
                    // otherwise, create a branch with two children
                    let mut children: [Option<Box<MptNode>>; 16] = Default::default();

                    children[self_nibs[common_len] as usize] = Some(Box::new(
                        MptNodeData::leaf(
                            to_encoded_path(&self_nibs[split_point..], true),
                            mem::take(old_value),
                        )
                        .into(),
                    ));
                    children[key_nibs[common_len] as usize] = Some(Box::new(
                        MptNodeData::leaf(to_encoded_path(&key_nibs[split_point..], true), value)
                            .into(),
                    ));

                    let branch = MptNodeData::Branch(children);
                    if common_len > 0 {
                        // create parent extension for new branch
                        self.data = MptNodeData::extension(
                            to_encoded_path(&self_nibs[..common_len], false),
                            Box::new(branch.into()),
                        );
                    } else {
                        self.data = branch;
                    }
                }
            }
            MptNodeData::Extension(prefix, existing_child) => {
                let self_nibs = prefix_nibs(prefix);
                let common_len = lcp(&self_nibs, key_nibs);
                if common_len == self_nibs.len() {
                    // traverse down for update
                    if !existing_child.insert_internal(&key_nibs[common_len..], value)? {
                        return Ok(false);
                    }
                } else if common_len == key_nibs.len() {
                    return Err(Error::ValueInBranch);
                } else {
                    let split_point = common_len + 1;
                    // otherwise, create a branch with two children
                    let mut children: [Option<Box<MptNode>>; 16] = Default::default();

                    children[self_nibs[common_len] as usize] = if split_point < self_nibs.len() {
                        Some(Box::new(
                            MptNodeData::extension(
                                to_encoded_path(&self_nibs[split_point..], false),
                                mem::take(existing_child),
                            )
                            .into(),
                        ))
                    } else {
                        Some(mem::take(existing_child))
                    };
                    children[key_nibs[common_len] as usize] = Some(Box::new(
                        MptNodeData::leaf(to_encoded_path(&key_nibs[split_point..], true), value)
                            .into(),
                    ));

                    let branch = MptNodeData::Branch(children);
                    if common_len > 0 {
                        // Create parent extension for new branch
                        self.data = MptNodeData::extension(
                            to_encoded_path(&self_nibs[..common_len], false),
                            Box::new(branch.into()),
                        );
                    } else {
                        self.data = branch;
                    }
                }
            }
            MptNodeData::Digest(digest) => return Err(Error::NodeNotResolved(*digest)),
        };

        self.invalidate_ref_cache();
        Ok(true)
    }

    fn invalidate_ref_cache(&mut self) {
        self.cached_reference.lock().unwrap().take();
    }

    /// Returns the length of the RLP payload of the node.
    fn payload_length(&self) -> usize {
        match &self.data {
            MptNodeData::Null => 0,
            MptNodeData::Branch(nodes) => {
                1 + nodes
                    .iter()
                    .map(|child| child.as_ref().map_or(1, |node| node.reference_length()))
                    .sum::<usize>()
            }
            MptNodeData::Leaf(prefix, value) => prefix.as_ref().length() + value.as_ref().length(),
            MptNodeData::Extension(prefix, node) => {
                prefix.as_ref().length() + node.reference_length()
            }
            MptNodeData::Digest(_) => 32,
        }
    }
}

impl<'a> MptNodeData<'a> {
    fn leaf(prefix: impl Into<Cow<'a, [u8]>>, value: impl Into<Cow<'a, [u8]>>) -> MptNodeData<'a> {
        MptNodeData::Leaf(prefix.into(), value.into())
    }

    fn extension(
        prefix: impl Into<Cow<'a, [u8]>>,
        node: impl Into<Box<MptNode<'a>>>,
    ) -> MptNodeData<'a> {
        MptNodeData::Extension(prefix.into(), node.into())
    }
}

impl<'a> MptNodeReference<'a> {
    fn bytes(bytes: impl Into<Cow<'a, [u8]>>) -> MptNodeReference<'a> {
        MptNodeReference::Bytes(bytes.into())
    }
}
/// Converts a byte slice into a vector of nibbles.
///
/// A nibble is 4 bits or half of an 8-bit byte. This function takes each byte from the
/// input slice, splits it into two nibbles, and appends them to the resulting vector.
pub fn to_nibs(slice: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(2 * slice.len());
    for byte in slice {
        result.push(byte >> 4);
        result.push(byte & 0xf);
    }
    result
}

/// Encodes a slice of nibbles into a vector of bytes, with an additional prefix to
/// indicate the type of node (leaf or extension).
///
/// The function starts by determining the type of node based on the `is_leaf` parameter.
/// If the node is a leaf, the prefix is set to `0x20`. If the length of the nibbles is
/// odd, the prefix is adjusted and the first nibble is incorporated into it.
///
/// The remaining nibbles are then combined into bytes, with each pair of nibbles forming
/// a single byte. The resulting vector starts with the prefix, followed by the encoded
/// bytes.
pub fn to_encoded_path(mut nibs: &[u8], is_leaf: bool) -> Vec<u8> {
    let mut prefix = (is_leaf as u8) * 0x20;
    if nibs.len() % 2 != 0 {
        prefix += 0x10 + nibs[0];
        nibs = &nibs[1..];
    }
    iter::once(prefix)
        .chain(nibs.chunks_exact(2).map(|byte| (byte[0] << 4) + byte[1]))
        .collect()
}

/// Returns the length of the common prefix.
fn lcp(a: &[u8], b: &[u8]) -> usize {
    for (i, (a, b)) in iter::zip(a, b).enumerate() {
        if a != b {
            return i;
        }
    }
    cmp::min(a.len(), b.len())
}

fn prefix_nibs(prefix: &[u8]) -> Vec<u8> {
    let (extension, tail) = prefix.split_first().unwrap();
    // the first bit of the first nibble denotes the parity
    let is_odd = extension & (1 << 4) != 0;

    let mut result = Vec::with_capacity(2 * tail.len() + is_odd as usize);
    // for odd lengths, the second nibble contains the first element
    if is_odd {
        result.push(extension & 0xf);
    }
    for nib in tail {
        result.push(nib >> 4);
        result.push(nib & 0xf);
    }
    result
}

/// Creates a new MPT trie where all the digests contained in `node_store` are resolved.
pub fn resolve_nodes<'a>(
    root: &MptNode<'a>,
    node_store: &HashMap<MptNodeReference, MptNode<'a>>,
) -> MptNode<'a> {
    resolve_nodes_detect_storage_roots(root, node_store, None, Nibbles::default())
}

/// Creates a new MPT trie where all the digests contained in `node_store` are resolved.
pub fn resolve_nodes_detect_storage_roots<'a>(
    root: &MptNode<'a>,
    node_store: &HashMap<MptNodeReference, MptNode<'a>>,
    mut storage_roots: Option<&mut Vec<(B256, B256)>>,
    path: Nibbles,
) -> MptNode<'a> {
    let trie = match root.as_data() {
        MptNodeData::Null => root.clone(),
        MptNodeData::Leaf(key, value) => {
            if let Some(storage_roots) = storage_roots.as_deref_mut() {
                let full_path = path.join(&Nibbles::from_nibbles(&prefix_nibs(key)));
                let hashed_address = B256::from_slice(&full_path.pack());
                let account =
                    <alloy_trie::TrieAccount as Decodable>::decode(&mut &value[..]).unwrap();
                if account.storage_root != alloy_trie::EMPTY_ROOT_HASH {
                    storage_roots.push((hashed_address, account.storage_root));
                }
            }

            root.clone()
        }
        MptNodeData::Branch(children) => {
            let children: Vec<_> = children
                .iter()
                .enumerate()
                .map(|(idx, child)| {
                    child.as_ref().map(|node| {
                        let mut child_path = path.clone();
                        child_path.push_unchecked(idx as u8);
                        Box::new(resolve_nodes_detect_storage_roots(
                            node,
                            node_store,
                            storage_roots.as_deref_mut(),
                            child_path,
                        ))
                    })
                })
                .collect();
            MptNodeData::Branch(children.try_into().unwrap()).into()
        }
        MptNodeData::Extension(prefix, target) => {
            let mut child_path = path.clone();
            child_path.extend(&Nibbles::from_nibbles(prefix_nibs(prefix)));

            MptNodeData::Extension(
                prefix.clone(),
                Box::new(resolve_nodes_detect_storage_roots(
                    target,
                    node_store,
                    storage_roots,
                    child_path,
                )),
            )
            .into()
        }
        MptNodeData::Digest(digest) => {
            if let Some(node) = node_store.get(&MptNodeReference::Digest(*digest)) {
                resolve_nodes_detect_storage_roots(node, node_store, storage_roots, path)
            } else {
                root.clone()
            }
        }
    };
    // the root hash must not change
    assert_eq!(root.hash(), trie.hash());

    trie
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbv_primitives::alloy_primitives::hex;

    #[test]
    pub fn test_trie_pointer_no_keccak() {
        let cases = [
            ("do", "verb"),
            ("dog", "puppy"),
            ("doge", "coin"),
            ("horse", "stallion"),
        ];
        for (k, v) in cases {
            let node: MptNode = MptNodeData::leaf(k.as_bytes(), v.as_bytes()).into();
            assert!(
                matches!(node.reference(),MptNodeReference::Bytes(bytes) if bytes == node.to_rlp().to_vec())
            );
        }
    }

    #[test]
    pub fn test_to_encoded_path() {
        // extension node with an even path length
        let nibbles = vec![0x0a, 0x0b, 0x0c, 0x0d];
        assert_eq!(to_encoded_path(&nibbles, false), vec![0x00, 0xab, 0xcd]);
        // extension node with an odd path length
        let nibbles = vec![0x0a, 0x0b, 0x0c];
        assert_eq!(to_encoded_path(&nibbles, false), vec![0x1a, 0xbc]);
        // leaf node with an even path length
        let nibbles = vec![0x0a, 0x0b, 0x0c, 0x0d];
        assert_eq!(to_encoded_path(&nibbles, true), vec![0x20, 0xab, 0xcd]);
        // leaf node with an odd path length
        let nibbles = vec![0x0a, 0x0b, 0x0c];
        assert_eq!(to_encoded_path(&nibbles, true), vec![0x3a, 0xbc]);
    }

    #[test]
    pub fn test_lcp() {
        let cases = [
            (vec![], vec![], 0),
            (vec![0xa], vec![0xa], 1),
            (vec![0xa, 0xb], vec![0xa, 0xc], 1),
            (vec![0xa, 0xb], vec![0xa, 0xb], 2),
            (vec![0xa, 0xb], vec![0xa, 0xb, 0xc], 2),
            (vec![0xa, 0xb, 0xc], vec![0xa, 0xb, 0xc], 3),
            (vec![0xa, 0xb, 0xc], vec![0xa, 0xb, 0xc, 0xd], 3),
            (vec![0xa, 0xb, 0xc, 0xd], vec![0xa, 0xb, 0xc, 0xd], 4),
        ];
        for (a, b, cpl) in cases {
            assert_eq!(lcp(&a, &b), cpl)
        }
    }

    #[test]
    pub fn test_empty() {
        let trie = MptNode::default();

        assert!(trie.is_empty());
        assert_eq!(trie.reference(), MptNodeReference::bytes(vec![0x80]));
        let expected = hex!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
        assert_eq!(expected, trie.hash().0);

        // test RLP encoding
        let mut out = Vec::new();
        trie.encode(&mut out);
        assert_eq!(out, vec![0x80]);
        assert_eq!(trie.length(), out.len());
        let decoded = MptNode::decode(&mut &*out).unwrap();
        assert_eq!(trie.hash(), decoded.hash());
    }

    #[test]
    pub fn test_empty_key() {
        let mut trie = MptNode::default();

        trie.insert_rlp(&[], b"empty".to_vec()).unwrap();
        assert_eq!(trie.get_rlp(&[]).unwrap(), Some(b"empty".to_vec()));
        assert!(trie.delete(&[]).unwrap());
    }

    #[test]
    pub fn test_tiny() {
        // trie consisting of an extension, a branch and two leafs
        let mut trie = MptNode::default();
        trie.insert_rlp(b"a", 0u8).unwrap();
        trie.insert_rlp(b"b", 1u8).unwrap();

        assert!(!trie.is_empty());
        let exp_rlp = hex!("d816d680c3208180c220018080808080808080808080808080");
        assert_eq!(trie.reference(), MptNodeReference::bytes(&exp_rlp));
        let exp_hash = hex!("6fbf23d6ec055dd143ff50d558559770005ff44ae1d41276f1bd83affab6dd3b");
        assert_eq!(trie.hash().0, exp_hash);

        // test RLP encoding
        let mut out = Vec::new();
        trie.invalidate_ref_cache();
        trie.encode(&mut out);
        assert_eq!(out, exp_rlp.to_vec());
        assert_eq!(trie.length(), out.len());
        let decoded = MptNode::decode(&mut &*out).unwrap();
        assert_eq!(trie.hash(), decoded.hash());
    }

    #[test]
    pub fn test_partial() {
        let mut trie = MptNode::default();
        trie.insert_rlp(b"aa", 0u8).unwrap();
        trie.insert_rlp(b"ab", 1u8).unwrap();
        trie.insert_rlp(b"ba", 2u8).unwrap();

        let exp_hash = trie.hash();

        // replace one node with its digest
        let MptNodeData::Extension(_, node) = &mut trie.data else {
            panic!("extension expected")
        };
        **node = MptNodeData::Digest(node.hash()).into();
        assert!(node.is_digest());
        let bytes = trie.to_rlp();
        let trie = MptNode::decode(&mut bytes.as_slice()).unwrap();
        assert_eq!(trie.hash(), exp_hash);

        // lookups should fail
        trie.get(b"aa").unwrap_err();
        trie.get(b"a0").unwrap_err();
    }

    #[test]
    pub fn test_branch_value() {
        let mut trie = MptNode::default();
        trie.insert_rlp(b"do", b"verb".to_vec()).unwrap();
        // leads to a branch with value which is not supported
        trie.insert_rlp(b"dog", b"puppy".to_vec()).unwrap_err();
    }

    #[test]
    pub fn test_keccak_trie() {
        const N: usize = 512;

        // insert
        let mut trie = MptNode::default();
        for i in 0..N {
            assert!(
                trie.insert_rlp(keccak256(i.to_be_bytes()).as_ref(), i)
                    .unwrap()
            );

            // check hash against trie build in reverse
            let mut reference = MptNode::default();
            for j in (0..=i).rev() {
                reference
                    .insert_rlp(keccak256(j.to_be_bytes()).as_ref(), j)
                    .unwrap();
            }
            assert_eq!(trie.hash(), reference.hash());
        }

        let expected = hex!("7310027edebdd1f7c950a7fb3413d551e85dff150d45aca4198c2f6315f9b4a7");
        assert_eq!(trie.hash().0, expected);

        // get
        for i in 0..N {
            assert_eq!(
                trie.get_rlp(keccak256(i.to_be_bytes()).as_ref()).unwrap(),
                Some(i)
            );
            assert!(
                trie.get(keccak256((i + N).to_be_bytes()).as_ref())
                    .unwrap()
                    .is_none()
            );
        }

        // delete
        for i in 0..N {
            assert!(trie.delete(keccak256(i.to_be_bytes()).as_ref()).unwrap());

            let mut reference = MptNode::default();
            for j in ((i + 1)..N).rev() {
                reference
                    .insert_rlp(keccak256(j.to_be_bytes()).as_ref(), j)
                    .unwrap();
            }
            assert_eq!(trie.hash(), reference.hash());
        }
        assert!(trie.is_empty());
    }

    #[test]
    pub fn test_index_trie() {
        const N: usize = 512;

        // insert
        let mut trie = MptNode::default();
        for i in 0..N {
            assert!(trie.insert_rlp(&i.to_rlp(), i).unwrap());

            // check hash against trie build in reverse
            let mut reference = MptNode::default();
            for j in (0..=i).rev() {
                reference.insert_rlp(&j.to_rlp(), j).unwrap();
            }
            assert_eq!(trie.hash(), reference.hash());

            // try RLP roundtrip
            let bytes = trie.to_rlp();
            let decoded = MptNode::decode(&mut bytes.as_slice()).unwrap();

            assert_eq!(trie.hash(), decoded.hash());
        }

        // get
        for i in 0..N {
            assert_eq!(trie.get_rlp(&i.to_rlp()).unwrap(), Some(i));
            assert!(trie.get(&(i + N).to_rlp()).unwrap().is_none());
        }

        // delete
        for i in 0..N {
            assert!(trie.delete(&i.to_rlp()).unwrap());

            let mut reference = MptNode::default();
            for j in ((i + 1)..N).rev() {
                reference.insert_rlp(&j.to_rlp(), j).unwrap();
            }
            assert_eq!(trie.hash(), reference.hash());
        }
        assert!(trie.is_empty());
    }
}
