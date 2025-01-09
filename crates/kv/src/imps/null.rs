//! A null provider that does nothing.
use crate::{KeyValueStoreGet, KeyValueStoreInsert};
use std::{borrow::Borrow, hash::Hash};

/// A null provider that does nothing.
#[derive(Debug, Copy, Clone)]
pub struct NullProvider;

impl<K: Ord + Hash + Eq, V> KeyValueStoreGet<K, V> for NullProvider {
    fn get<Q>(&self, _k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized,
    {
        None
    }
}

impl<K: Ord + Hash + Eq, V> KeyValueStoreInsert<K, V> for NullProvider {
    fn insert(&mut self, _k: K, _v: V) {
        // do nothing
    }

    fn or_insert_with<F: FnOnce() -> V>(&mut self, _k: K, _default: F) {
        // do nothing
    }
}
