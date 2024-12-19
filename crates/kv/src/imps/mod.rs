use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert, Value};
use std::borrow::{Borrow, Cow};
use std::hash::Hash;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

mod alloy_primitives;
mod alloy_trie;
pub mod nohash;
#[cfg(feature = "sled")]
mod sled;
pub mod small;
mod std_collections;

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value, T: KeyValueStoreGet<K, V>> KeyValueStoreGet<K, V>
    for ManuallyDrop<T>
{
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + AsRef<[u8]>,
    {
        self.deref().get(k)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value, T: KeyValueStoreInsert<K, V>>
    KeyValueStoreInsert<K, V> for ManuallyDrop<T>
{
    fn insert(&mut self, k: K, v: V) {
        self.deref_mut().insert(k, v)
    }

    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        self.deref_mut().or_insert_with(k, default)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value, T: KeyValueStore<K, V>> KeyValueStore<K, V>
    for ManuallyDrop<T>
{
}
