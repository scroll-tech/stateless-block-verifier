use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert, Value};
use std::borrow::{Borrow, Cow};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreInsert<K, V> for HashMap<K, V> {
    fn insert(&mut self, k: K, v: V) {
        HashMap::insert(self, k, v);
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        HashMap::entry(self, k).or_insert_with(default);
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreGet<K, V> for HashMap<K, V> {
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + AsRef<[u8]>,
    {
        HashMap::get(self, k).map(Cow::Borrowed)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStore<K, V> for HashMap<K, V> {}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreInsert<K, V> for BTreeMap<K, V> {
    fn insert(&mut self, k: K, v: V) {
        BTreeMap::insert(self, k, v);
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        BTreeMap::entry(self, k).or_insert_with(default);
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreGet<K, V> for BTreeMap<K, V> {
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + AsRef<[u8]>,
    {
        BTreeMap::get(self, k).map(Cow::Borrowed)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStore<K, V> for BTreeMap<K, V> {}
