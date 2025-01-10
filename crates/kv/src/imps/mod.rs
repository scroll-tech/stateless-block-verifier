use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert};
use std::{
    borrow::Borrow,
    hash::Hash,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

pub mod nohash;
pub mod null;
mod std_collections;

impl<K: Ord + Hash + Eq, V, T: KeyValueStoreGet<K, V>> KeyValueStoreGet<K, V> for ManuallyDrop<T> {
    fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        self.deref().get(k)
    }
}

impl<K: Ord + Hash + Eq, V, T: KeyValueStoreInsert<K, V>> KeyValueStoreInsert<K, V>
    for ManuallyDrop<T>
{
    fn insert(&mut self, k: K, v: V) {
        self.deref_mut().insert(k, v)
    }

    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        self.deref_mut().or_insert_with(k, default)
    }
}

impl<K: Ord + Hash + Eq, V, T: KeyValueStore<K, V>> KeyValueStore<K, V> for ManuallyDrop<T> {}
