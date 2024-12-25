use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert, Value};
use std::borrow::{Borrow, Cow};
use std::collections::BTreeMap;
use std::hash::Hash;

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreInsert<K, V> for sled::Tree {
    fn insert(&mut self, k: K, v: V) {
        sled::Tree::insert(self, k, v.serialize()).expect("sled io error");
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        sled::Tree::insert(self, k, default().serialize()).expect("sled io error");
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreGet<K, V> for sled::Tree {
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq,
    {
        sled::Tree::get(self, k)
            .expect("sled io error")
            .map(|vec| Value::deserialize(vec.as_ref()))
            .map(Cow::Owned)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStore<K, V> for sled::Tree {}
