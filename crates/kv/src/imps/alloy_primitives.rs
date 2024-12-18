use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert, Value};
use alloy_primitives::{map::B256HashMap, Bytes, B256};
use std::borrow::{Borrow, Cow};
use std::hash::Hash;

impl Value for Bytes {
    #[cfg(feature = "sled")]
    fn serialize(&self) -> Vec<u8> {
        self.to_vec()
    }

    #[cfg(feature = "sled")]
    fn deserialize(buf: &[u8]) -> Self {
        Bytes::copy_from_slice(buf)
    }
}

impl<V: Value> KeyValueStoreInsert<B256, V> for B256HashMap<V> {
    fn insert(&mut self, k: B256, v: V) {
        B256HashMap::insert(self, k, v);
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: B256, default: F) {
        B256HashMap::entry(self, k).or_insert_with(default);
    }
}

impl<V: Value> KeyValueStoreGet<B256, V> for B256HashMap<V> {
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        B256: Borrow<Q>,
        Q: Ord + Hash + Eq + AsRef<[u8]>,
    {
        B256HashMap::get(self, k).map(Cow::Borrowed)
    }
}

impl<V: Value> KeyValueStore<B256, V> for B256HashMap<V> {}
