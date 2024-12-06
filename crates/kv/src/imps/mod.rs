use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert, Value};
use std::borrow::{Borrow, Cow};
use std::hash::Hash;

#[cfg(feature = "alloy-trie")]
mod alloy_trie;
#[cfg(feature = "sled")]
mod sled;
mod std_collections;

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value, S: KeyValueStore<K, V>> KeyValueStoreGet<K, V>
    for &S
{
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + AsRef<[u8]>,
    {
        S::get(*self, k)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value, S: KeyValueStore<K, V>> KeyValueStoreGet<K, V>
    for &mut S
{
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + AsRef<[u8]>,
    {
        S::get(*self, k)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value, S: KeyValueStore<K, V>> KeyValueStoreInsert<K, V>
    for &mut S
{
    fn insert(&mut self, k: K, v: V) {
        S::insert(*self, k, v)
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value, S: KeyValueStore<K, V>> KeyValueStore<K, V>
    for &mut S
{
}
