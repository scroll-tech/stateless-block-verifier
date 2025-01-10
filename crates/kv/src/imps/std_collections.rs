use crate::{HashMap, KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert};
use core::hash::{BuildHasher, Hash};
use std::{borrow::Borrow, collections::BTreeMap};

impl<K: Ord + Hash + Eq, V, S: BuildHasher> KeyValueStoreInsert<K, V> for HashMap<K, V, S> {
    fn insert(&mut self, k: K, v: V) {
        HashMap::insert(self, k, v);
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        HashMap::entry(self, k).or_insert_with(default);
    }
}

impl<K: Ord + Hash + Eq, V, S: BuildHasher> KeyValueStoreGet<K, V> for HashMap<K, V, S> {
    fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        HashMap::get(self, k)
    }
}

impl<K: Ord + Hash + Eq, V, S: BuildHasher> KeyValueStore<K, V> for HashMap<K, V, S> {}

impl<K: Ord + Hash + Eq, V> KeyValueStoreInsert<K, V> for BTreeMap<K, V> {
    fn insert(&mut self, k: K, v: V) {
        BTreeMap::insert(self, k, v);
    }
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        BTreeMap::entry(self, k).or_insert_with(default);
    }
}

impl<K: Ord + Hash + Eq, V> KeyValueStoreGet<K, V> for BTreeMap<K, V> {
    fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        BTreeMap::get(self, k)
    }
}

impl<K: Ord + Hash + Eq, V> KeyValueStore<K, V> for BTreeMap<K, V> {}
