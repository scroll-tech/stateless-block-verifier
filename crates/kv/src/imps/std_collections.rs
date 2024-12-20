use crate::HashMap;
use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert, Value};
use core::hash::{BuildHasher, Hash};
use std::borrow::{Borrow, Cow};
use std::collections::BTreeMap;

impl<K: Ord + Hash + Eq, V: Value, S: BuildHasher> KeyValueStoreInsert<K, V> for HashMap<K, V, S> {
    fn insert(&mut self, k: K, v: V) {
        HashMap::insert(self, k, v);
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        HashMap::entry(self, k).or_insert_with(default);
    }
}

impl<K: Ord + Hash + Eq, V: Value, S: BuildHasher> KeyValueStoreGet<K, V> for HashMap<K, V, S> {
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq,
    {
        HashMap::get(self, k).map(Cow::Borrowed)
    }
}

impl<K: Ord + Hash + Eq, V: Value, S: BuildHasher> KeyValueStore<K, V> for HashMap<K, V, S> {}

impl<K: Ord + Hash + Eq, V: Value> KeyValueStoreInsert<K, V> for BTreeMap<K, V> {
    fn insert(&mut self, k: K, v: V) {
        BTreeMap::insert(self, k, v);
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        BTreeMap::entry(self, k).or_insert_with(default);
    }
}

impl<K: Ord + Hash + Eq, V: Value> KeyValueStoreGet<K, V> for BTreeMap<K, V> {
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq,
    {
        BTreeMap::get(self, k).map(Cow::Borrowed)
    }
}

impl<K: Ord + Hash + Eq, V: Value> KeyValueStore<K, V> for BTreeMap<K, V> {}
