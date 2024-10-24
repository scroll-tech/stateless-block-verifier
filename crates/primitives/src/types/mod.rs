use crate::{Block, NodeProof};
use alloy::primitives::{Address, Bytes, B256, U256};
use rkyv::vec::ArchivedVec;
use rkyv::{rancor, Archive};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, Map};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use zktrie_ng::db::kv::KVDatabase;
use zktrie_ng::db::NodeDb;
use zktrie_ng::hash::poseidon::Poseidon;
use zktrie_ng::trie::{ArchivedNode, Node, MAGIC_NODE_BYTES};

mod tx;
pub use tx::{
    AlloyTransaction, ArchivedTransactionTrace, TransactionTrace, TxL1Msg, TypedTransaction,
};

/// Block header
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Serialize, Deserialize, Default, Debug, Clone,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct BlockHeader {
    /// block number
    #[rkyv(attr(doc = "block number"))]
    pub number: U256,
    /// block hash
    #[rkyv(attr(doc = "block hash"))]
    pub hash: B256,
    /// timestamp
    #[rkyv(attr(doc = "timestamp"))]
    pub timestamp: U256,
    /// gas limit
    #[serde(rename = "gasLimit")]
    #[rkyv(attr(doc = "gas limit"))]
    pub gas_limit: U256,
    /// gas used
    #[serde(rename = "gasUsed")]
    #[rkyv(attr(doc = "gas used"))]
    pub gas_used: U256,
    /// base fee per gas
    #[serde(rename = "baseFeePerGas")]
    #[rkyv(attr(doc = "base fee per gas"))]
    pub base_fee_per_gas: Option<U256>,
    /// difficulty
    #[rkyv(attr(doc = "difficulty"))]
    pub difficulty: U256,
    /// mix hash
    #[rkyv(attr(doc = "mix hash"))]
    #[serde(rename = "mixHash")]
    pub mix_hash: Option<B256>,
}

/// Coinbase
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Serialize, Deserialize, Default, Debug, Clone,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct Coinbase {
    /// address of coinbase
    #[rkyv(attr(doc = "address of coinbase"))]
    pub address: Address,
}

/// Bytecode trace
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Serialize, Deserialize, Default, Debug, Clone,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct BytecodeTrace {
    /// bytecode
    #[rkyv(attr(doc = "bytecode"))]
    pub code: Bytes,
}

/// storage trace
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Serialize,
    Default,
    Debug,
    Clone,
    Hash,
    Eq,
    PartialEq,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct StorageTrace<N = Bytes>
where
    N: Archive,
    Vec<N>: Archive<Archived = ArchivedVec<<N as Archive>::Archived>>,
    <Vec<N> as Archive>::Archived: Debug + Hash + PartialEq + Eq,
{
    /// root before
    #[rkyv(attr(doc = "root before"))]
    #[serde(rename = "rootBefore")]
    pub root_before: B256,
    /// root after
    #[rkyv(attr(doc = "root after"))]
    #[serde(rename = "rootAfter")]
    pub root_after: B256,
    /// proofs
    #[serde(rename = "flattenProofs")]
    #[rkyv(attr(doc = "proofs"))]
    pub flatten_proofs: Vec<N>,
}

/// rkyv serialized node bytes
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Default, Debug, Clone)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
#[repr(C, align(4))]
pub struct ArchivedNodeBytes(Vec<u8>);

/// legacy storage trace
#[serde_as]
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Serialize,
    Deserialize,
    Default,
    Debug,
    Clone,
    Hash,
    Eq,
    PartialEq,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
#[allow(clippy::type_complexity)]
pub struct LegacyStorageTrace {
    /// root before
    #[rkyv(attr(doc = "root before"))]
    #[serde(rename = "rootBefore")]
    pub root_before: B256,
    /// root after
    #[rkyv(attr(doc = "root after"))]
    #[serde(rename = "rootAfter")]
    pub root_after: B256,
    /// account proofs
    #[rkyv(attr(doc = "account proofs"))]
    #[serde(default)]
    #[serde_as(as = "Map<_, _>")]
    pub proofs: Vec<(Address, Vec<Bytes>)>,
    /// storage proofs for each account
    #[rkyv(attr(doc = "storage proofs for each account"))]
    #[serde(rename = "storageProofs", default)]
    #[serde_as(as = "Map<_, Map<_, _>>")]
    pub storage_proofs: Vec<(Address, Vec<(B256, Vec<Bytes>)>)>,
    /// additional deletion proofs
    #[rkyv(attr(doc = "additional deletion proofs"))]
    #[serde(rename = "deletionProofs", default)]
    pub deletion_proofs: Vec<Bytes>,
}

/// Block trace format
///
/// ref: <https://github.com/scroll-tech/go-ethereum/blob/develop/core/types/l2trace.go>
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Serialize, Deserialize, Default, Debug, Clone,
)]
#[rkyv(derive(Debug, Hash, PartialEq, Eq))]
pub struct BlockTrace<S = StorageTrace>
where
    S: Archive,
    <S as Archive>::Archived: Debug + Hash + PartialEq + Eq,
{
    /// chain id
    #[rkyv(attr(doc = "chain id"))]
    #[serde(rename = "chainID", default)]
    pub chain_id: u64,
    /// coinbase
    #[rkyv(attr(doc = "coinbase"))]
    pub coinbase: Coinbase,
    /// block
    #[rkyv(attr(doc = "block"))]
    pub header: BlockHeader,
    /// txs
    #[rkyv(attr(doc = "txs"))]
    pub transactions: Vec<TransactionTrace>,
    /// bytecodes
    #[rkyv(attr(doc = "bytecodes"))]
    pub codes: Vec<BytecodeTrace>,
    /// storage trace BEFORE execution
    #[rkyv(attr(doc = "storage trace BEFORE execution"))]
    #[serde(rename = "storageTrace")]
    pub storage_trace: S,
    /// l1 tx queue
    #[rkyv(attr(doc = "l1 tx queue"))]
    #[serde(rename = "startL1QueueIndex", default)]
    pub start_l1_queue_index: u64,
    /// withdraw root
    #[rkyv(attr(doc = "withdraw root"))]
    pub withdraw_trie_root: B256,
}

impl Hash for ArchivedNodeBytes {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0.len());
        Hash::hash_slice(self.0.as_ref(), state)
    }
}

impl PartialEq for ArchivedNodeBytes {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_slice().eq(other.0.as_slice())
    }
}

impl Eq for ArchivedNodeBytes {}

impl Serialize for ArchivedNodeBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StorageTrace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum FlattenProofs {
            Map(HashMap<B256, Bytes>),
            Vec(Vec<Bytes>),
        }
        #[derive(Deserialize)]
        struct StorageTraceDe {
            #[serde(rename = "rootBefore")]
            pub root_before: B256,
            #[serde(rename = "rootAfter")]
            pub root_after: B256,
            #[serde(rename = "flattenProofs")]
            pub flatten_proofs: FlattenProofs,
        }

        let de = StorageTraceDe::deserialize(deserializer)?;
        let mut flatten_proofs = match de.flatten_proofs {
            FlattenProofs::Map(map) => map.into_values().collect(),
            FlattenProofs::Vec(vec) => vec,
        };
        flatten_proofs.sort();

        Ok(StorageTrace {
            root_before: de.root_before,
            root_after: de.root_after,
            flatten_proofs,
        })
    }
}

impl From<StorageTrace> for StorageTrace<ArchivedNodeBytes> {
    fn from(trace: StorageTrace) -> Self {
        StorageTrace {
            root_before: trace.root_before,
            root_after: trace.root_after,
            flatten_proofs: trace
                .flatten_proofs
                .into_iter()
                .filter(|proof| proof.as_ref() != MAGIC_NODE_BYTES)
                .map(|proof| {
                    ArchivedNodeBytes(
                        Node::<Poseidon>::try_from(proof.as_ref())
                            .expect("invalid node")
                            .archived()
                            .to_vec(),
                    )
                })
                .collect(),
        }
    }
}

impl From<LegacyStorageTrace> for StorageTrace {
    fn from(trace: LegacyStorageTrace) -> Self {
        let mut flatten_proofs = BTreeSet::new();
        for (_, proofs) in trace.proofs {
            flatten_proofs.extend(proofs);
        }
        for (_, proofs) in trace.storage_proofs {
            for (_, proofs) in proofs {
                flatten_proofs.extend(proofs);
            }
        }
        flatten_proofs.extend(trace.deletion_proofs);

        StorageTrace {
            root_before: trace.root_before,
            root_after: trace.root_after,
            flatten_proofs: flatten_proofs.into_iter().collect(),
        }
    }
}

impl From<BlockTrace> for BlockTrace<StorageTrace<ArchivedNodeBytes>> {
    fn from(trace: BlockTrace) -> Self {
        BlockTrace {
            chain_id: trace.chain_id,
            coinbase: trace.coinbase,
            header: trace.header,
            transactions: trace.transactions,
            codes: trace.codes,
            storage_trace: trace.storage_trace.into(),
            start_l1_queue_index: trace.start_l1_queue_index,
            withdraw_trie_root: trace.withdraw_trie_root,
        }
    }
}

impl From<BlockTrace<LegacyStorageTrace>> for BlockTrace {
    fn from(trace: BlockTrace<LegacyStorageTrace>) -> Self {
        BlockTrace {
            chain_id: trace.chain_id,
            coinbase: trace.coinbase,
            header: trace.header,
            transactions: trace.transactions,
            codes: trace.codes,
            storage_trace: trace.storage_trace.into(),
            start_l1_queue_index: trace.start_l1_queue_index,
            withdraw_trie_root: trace.withdraw_trie_root,
        }
    }
}

impl<S> Block for BlockTrace<S>
where
    S: StorageTraceExt + Archive + Debug,
    <S as Archive>::Archived: Debug + Hash + PartialEq + Eq,
    <S as StorageTraceExt>::Node: NodeProof,
{
    type Node = S::Node;
    type Tx = TransactionTrace;

    fn number(&self) -> u64 {
        self.header.number.to()
    }
    fn block_hash(&self) -> B256 {
        self.header.hash
    }
    fn chain_id(&self) -> u64 {
        self.chain_id
    }
    fn coinbase(&self) -> Address {
        self.coinbase.address
    }
    fn timestamp(&self) -> U256 {
        self.header.timestamp
    }

    fn gas_limit(&self) -> U256 {
        self.header.gas_limit
    }

    fn gas_used(&self) -> U256 {
        self.header.gas_used
    }

    fn base_fee_per_gas(&self) -> Option<U256> {
        self.header.base_fee_per_gas
    }

    fn difficulty(&self) -> U256 {
        self.header.difficulty
    }

    fn prevrandao(&self) -> Option<B256> {
        self.header.mix_hash
    }

    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.iter()
    }

    fn root_before(&self) -> B256 {
        self.storage_trace.root_before()
    }

    fn root_after(&self) -> B256 {
        self.storage_trace.root_after()
    }

    fn withdraw_root(&self) -> B256 {
        self.withdraw_trie_root
    }

    fn codes(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.codes.iter().map(|code| code.code.as_ref())
    }

    fn start_l1_queue_index(&self) -> u64 {
        self.start_l1_queue_index
    }

    fn node_proofs(&self) -> impl Iterator<Item = &Self::Node> {
        self.storage_trace.node_proofs()
    }
}

impl<S> Block for ArchivedBlockTrace<S>
where
    S: Archive + Debug,
    <S as Archive>::Archived: StorageTraceExt + Debug + Hash + PartialEq + Eq,
    <<S as Archive>::Archived as StorageTraceExt>::Node: NodeProof,
{
    type Node = <<S as Archive>::Archived as StorageTraceExt>::Node;
    type Tx = ArchivedTransactionTrace;

    fn number(&self) -> u64 {
        let number: U256 = self.header.number.into();
        number.to()
    }

    fn block_hash(&self) -> B256 {
        self.header.hash.into()
    }

    fn chain_id(&self) -> u64 {
        self.chain_id.into()
    }

    fn coinbase(&self) -> Address {
        self.coinbase.address.into()
    }

    fn timestamp(&self) -> U256 {
        self.header.timestamp.into()
    }

    fn gas_limit(&self) -> U256 {
        self.header.gas_limit.into()
    }

    fn gas_used(&self) -> U256 {
        self.header.gas_used.into()
    }

    fn base_fee_per_gas(&self) -> Option<U256> {
        self.header.base_fee_per_gas.as_ref().map(|p| p.into())
    }

    fn difficulty(&self) -> U256 {
        self.header.difficulty.into()
    }

    fn prevrandao(&self) -> Option<B256> {
        self.header.mix_hash.as_ref().map(|p| p.into())
    }

    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.iter()
    }

    fn root_before(&self) -> B256 {
        self.storage_trace.root_before()
    }

    fn root_after(&self) -> B256 {
        self.storage_trace.root_after()
    }

    fn withdraw_root(&self) -> B256 {
        self.withdraw_trie_root.into()
    }

    fn codes(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.codes.iter().map(|code| code.code.as_ref())
    }

    fn start_l1_queue_index(&self) -> u64 {
        self.start_l1_queue_index.into()
    }

    fn node_proofs(&self) -> impl Iterator<Item = &Self::Node> {
        self.storage_trace.node_proofs()
    }
}

/// Extension trait for storage trace
pub trait StorageTraceExt {
    /// Node type
    type Node: Debug;

    /// Get root before
    fn root_before(&self) -> B256;

    /// Get root after
    fn root_after(&self) -> B256;

    /// Get node proofs
    fn node_proofs(&self) -> impl Iterator<Item = &Self::Node>;
}

impl<N> StorageTraceExt for StorageTrace<N>
where
    N: Archive + Debug,
    Vec<N>: Archive<Archived = ArchivedVec<<N as Archive>::Archived>>,
    <Vec<N> as Archive>::Archived: Debug + Hash + PartialEq + Eq,
{
    type Node = N;

    fn root_before(&self) -> B256 {
        self.root_before
    }

    fn root_after(&self) -> B256 {
        self.root_after
    }

    fn node_proofs(&self) -> impl Iterator<Item = &Self::Node> {
        self.flatten_proofs.iter()
    }
}

impl<N> StorageTraceExt for ArchivedStorageTrace<N>
where
    N: Archive + Debug,
    <N as Archive>::Archived: Debug,
    Vec<N>: Archive<Archived = ArchivedVec<<N as Archive>::Archived>>,
    <Vec<N> as Archive>::Archived: Debug + Hash + PartialEq + Eq,
{
    type Node = <N as Archive>::Archived;

    fn root_before(&self) -> B256 {
        self.root_before.into()
    }

    fn root_after(&self) -> B256 {
        self.root_after.into()
    }

    fn node_proofs(&self) -> impl Iterator<Item = &Self::Node> {
        self.flatten_proofs.as_ref().iter()
    }
}

impl StorageTraceExt for LegacyStorageTrace {
    type Node = Bytes;

    fn root_before(&self) -> B256 {
        self.root_before
    }

    fn root_after(&self) -> B256 {
        self.root_after
    }

    fn node_proofs(&self) -> impl Iterator<Item = &Self::Node> {
        self.proofs.iter().flat_map(|(_, proofs)| proofs.iter())
    }
}

fn import_serialized_node<N: AsRef<[u8]>, Db: KVDatabase>(
    node: N,
    db: &mut NodeDb<Db>,
) -> Result<(), Db::Error> {
    let bytes = node.as_ref();
    if bytes == MAGIC_NODE_BYTES {
        return Ok(());
    }
    let node =
        cycle_track!(Node::<Poseidon>::try_from(bytes), "Node::try_from").expect("invalid node");
    cycle_track!(
        node.get_or_calculate_node_hash(),
        "Node::get_or_calculate_node_hash"
    )
    .expect("infallible");
    dev_trace!("put zktrie node: {:?}", node);
    cycle_track!(db.put_node(node), "NodeDb::put_node")
}

fn import_archived_node<N: AsRef<[u8]>, Db: KVDatabase>(
    node: N,
    db: &mut NodeDb<Db>,
) -> Result<(), Db::Error> {
    let bytes = node.as_ref();
    let node = cycle_track!(
        rkyv::access::<ArchivedNode, rancor::Error>(bytes),
        "rkyv::access"
    )
    .expect("invalid node");
    let node_hash = cycle_track!(
        node.calculate_node_hash::<Poseidon>(),
        "Node::calculate_node_hash"
    )
    .expect("infallible");
    dev_trace!("put zktrie node: {:?}", node);
    cycle_track!(
        unsafe { db.put_archived_node_unchecked(node_hash, bytes.to_owned()) },
        "NodeDb::put_archived_node_unchecked"
    )
}

impl NodeProof for Bytes {
    fn import_node<Db: KVDatabase>(&self, db: &mut NodeDb<Db>) -> Result<(), Db::Error> {
        import_serialized_node(self, db)
    }
}

impl NodeProof for ArchivedVec<u8> {
    fn import_node<Db: KVDatabase>(&self, db: &mut NodeDb<Db>) -> Result<(), Db::Error> {
        import_serialized_node(self, db)
    }
}

impl NodeProof for ArchivedNodeBytes {
    fn import_node<Db: KVDatabase>(&self, db: &mut NodeDb<Db>) -> Result<(), Db::Error> {
        import_archived_node(&self.0, db)
    }
}

impl NodeProof for ArchivedArchivedNodeBytes {
    fn import_node<Db: KVDatabase>(&self, db: &mut NodeDb<Db>) -> Result<(), Db::Error> {
        import_archived_node(&self.0, db)
    }
}

impl Block for alloy::rpc::types::Block<AlloyTransaction, alloy::rpc::types::Header> {
    type Node = Bytes;

    type Tx = AlloyTransaction;

    fn number(&self) -> u64 {
        self.header.number
    }

    fn block_hash(&self) -> B256 {
        self.header.hash
    }

    fn chain_id(&self) -> u64 {
        0
    }

    fn coinbase(&self) -> Address {
        self.header.miner
    }

    fn timestamp(&self) -> U256 {
        U256::from_limbs([self.header.timestamp, 0, 0, 0])
    }

    fn gas_limit(&self) -> U256 {
        U256::from(self.header.gas_limit)
    }

    fn gas_used(&self) -> U256 {
        U256::from(self.header.gas_used)
    }

    fn base_fee_per_gas(&self) -> Option<U256> {
        self.header.base_fee_per_gas.map(U256::from)
    }

    fn difficulty(&self) -> U256 {
        self.header.difficulty
    }

    fn prevrandao(&self) -> Option<B256> {
        self.header.mix_hash
    }

    fn transactions(&self) -> impl Iterator<Item = &Self::Tx> {
        self.transactions.txns()
    }

    fn root_before(&self) -> B256 {
        unimplemented!()
    }

    fn root_after(&self) -> B256 {
        unimplemented!()
    }

    fn withdraw_root(&self) -> B256 {
        self.header.withdrawals_root.unwrap_or_default()
    }

    fn codes(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        [].into_iter()
    }

    fn start_l1_queue_index(&self) -> u64 {
        unimplemented!()
    }

    fn node_proofs(&self) -> impl Iterator<Item = &Self::Node> {
        [].into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TxTrace;
    use alloy::primitives::*;

    const TRACE: &str = include_str!("../../../../testdata/mainnet_blocks/8370400.json");

    #[test]
    fn test_deserialize() {
        let trace = serde_json::from_str::<serde_json::Value>(TRACE).unwrap()["result"].clone();

        let coinbase: Coinbase = serde_json::from_value(trace["coinbase"].clone()).unwrap();
        assert_eq!(
            coinbase.address,
            address!("5300000000000000000000000000000000000005")
        );

        let header: BlockHeader = serde_json::from_value(trace["header"].clone()).unwrap();
        assert_eq!(header.number, U256::from(8370400));
        assert_eq!(
            header.hash,
            b256!("3aec6d882b0548a6f073d5aed65cfa527809ea71528d5e387a9a37436f0c6f9c")
        );
        assert_eq!(header.timestamp, U256::from(0x66bcc0a0));
        assert_eq!(header.gas_limit, U256::from(0x989680));
        assert_eq!(header.base_fee_per_gas, Some(U256::from(0x3c9d282)));
        assert_eq!(header.difficulty, U256::from(0x2));
        assert_eq!(
            header.mix_hash,
            Some(b256!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            ))
        );

        let transactions: Vec<TransactionTrace> =
            serde_json::from_value(trace["transactions"].clone()).unwrap();
        assert_eq!(transactions[0].ty, 2);
        assert_eq!(transactions[1].nonce, 52786);
        assert_eq!(
            transactions[2].tx_hash,
            b256!("66fc61ff7dd747503aed97a04ca47f00534d914a3b242ea4b2c75b66d720d4b8")
        );
        assert_eq!(transactions[3].gas, 53340);
        assert_eq!(transactions[4].gas_price, U256::from(0x48c1a5b));
        assert_eq!(transactions[5].gas_tip_cap, Some(U256::from(0x416e)));
        assert_eq!(transactions[6].gas_fee_cap, Some(U256::from(0x4f4f99a)));
        assert_eq!(
            transactions[7].from,
            address!("32a29339a23afff0febd75bef656b9bf32967085")
        );
        assert_eq!(
            transactions[8].to,
            Some(address!("a2a9fd768d482caf519d749d3123a133db278a66")),
        );
        assert_eq!(transactions[9].chain_id, U64::from(0x82750));
        assert_eq!(transactions[10].value, uint!(0x297ee5eafa233c_U256));
        assert_eq!(transactions[11].data, bytes!("21c69a19000000000000000000000000f610a9dfb7c89644979b4a0f27063e9e7d7cda320000000000000000000000000000000000000000000000000086e6edc73882e2000000000000000000000000956df8424b556f0076e8abf5481605f5a791cc7f000000000000000000000000956df8424b556f0076e8abf5481605f5a791cc7f00000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000244627dd56a000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000001ea01a200020000000000000000000000000000000000000000000000000086e6edc73882e2006018df4b145f074d63efa24dbd61dd00da2cdb3697f610a9dfb7c89644979b4a0f27063e9e7d7cda3200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000066bcce7c000000000000000000000000000000000000000000000000009b579427d194d20000000000000000000000000000000000000000000000000086e6edc73882e202007e07060300e6000000000000000000000000fffd8963efd1fc6a506488495d951d5263988d2500000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000014010037060300920300e603010e03012e03014e03007e6e6cc7163fa93a693ee8491bb2b01656ba5ea1f3070a00000000000000000000000000000000000000000000000000000000000003019805000002007e02018405004053000000000000000000000000000000000000040201c705004002009202006a050040004802008000000106010e00000000e40040016e01840184070040002001b801be0000010060000001be01c70000050040000001db01e10000030060000001e101ea00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"));
        assert!(!transactions[12].is_create);
        assert_eq!(transactions[13].access_list.len(), 0);
        assert_eq!(transactions[14].v.to::<u64>(), 0x1);
        assert_eq!(
            transactions[15].r,
            uint!(0xb6c02ca3114e6eb88b49e122a24903579112bdf93a615a145949c60c0d5f0b4b_U256)
        );
        assert_eq!(
            transactions[15].s,
            uint!(0x6989f79234aa582bbe1efbd6b5d5dcb1ba0247bcc3b641bc2ef1573b330254cf_U256)
        );

        let _codes: Vec<BytecodeTrace> = serde_json::from_value(trace["codes"].clone()).unwrap();

        let storage_trace: StorageTrace =
            serde_json::from_value(trace["storageTrace"].clone()).unwrap();
        assert_eq!(
            storage_trace.root_before,
            b256!("1cee7c2b120a46f498630029b2463d67e579c27d49ebedcb82f31960d5c03a7c")
        );
        assert_eq!(
            storage_trace.root_after,
            b256!("2f6af5a76ddd2fcd78b2f72e39282a782bfa30625b7d8d8f7506603b7511ba38")
        );

        #[derive(Deserialize)]
        struct Test {
            result: BlockTrace,
        }
        let _block: BlockTrace = serde_json::from_str::<Test>(TRACE).unwrap().result;
    }

    #[test]
    fn test_rkyv() {
        let trace = serde_json::from_str::<serde_json::Value>(TRACE).unwrap()["result"].clone();
        let block: BlockTrace = serde_json::from_value(trace).unwrap();
        let archived_bytes = rkyv::to_bytes::<rancor::Error>(&block).unwrap();
        let archived_block =
            rkyv::access::<ArchivedBlockTrace, rancor::Error>(archived_bytes.as_ref()).unwrap();

        assert_eq!(block.chain_id, archived_block.chain_id);
        assert_eq!(
            block.coinbase.address,
            Address::from(archived_block.coinbase.address)
        );

        assert_eq!(block.header.number, archived_block.header.number.into());
        assert_eq!(block.header.hash, B256::from(archived_block.header.hash));
        assert_eq!(
            block.header.timestamp,
            archived_block.header.timestamp.into()
        );
        assert_eq!(
            block.header.gas_limit,
            archived_block.header.gas_limit.into()
        );
        assert_eq!(
            block.header.base_fee_per_gas,
            archived_block
                .header
                .base_fee_per_gas
                .as_ref()
                .map(|p| p.into())
        );
        assert_eq!(
            block.header.difficulty,
            archived_block.header.difficulty.into()
        );
        assert_eq!(
            block.header.mix_hash,
            archived_block.header.mix_hash.as_ref().map(|p| p.into())
        );

        let txs = block
            .transactions
            .iter()
            .map(|tx| tx.try_build_typed_tx().unwrap());
        let archived_txs = archived_block
            .transactions
            .iter()
            .map(|tx| tx.try_build_typed_tx().unwrap());
        for (tx, archived_tx) in txs.zip(archived_txs) {
            assert_eq!(tx, archived_tx);
        }

        for (code, archived_code) in block.codes.iter().zip(archived_block.codes.iter()) {
            assert_eq!(code.code.as_ref(), archived_code.code.as_ref());
        }

        assert_eq!(
            block.storage_trace.root_before,
            B256::from(archived_block.storage_trace.root_before)
        );
        assert_eq!(
            block.storage_trace.root_after,
            B256::from(archived_block.storage_trace.root_after)
        );
        for (proof, archived_proof) in block
            .storage_trace
            .flatten_proofs
            .iter()
            .zip(archived_block.storage_trace.flatten_proofs.iter())
        {
            assert_eq!(proof.as_ref(), archived_proof.as_ref());
        }

        assert_eq!(
            block.start_l1_queue_index,
            archived_block.start_l1_queue_index
        );
    }
}
