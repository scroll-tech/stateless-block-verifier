use crate::{KeyValueStore, KeyValueStoreGet, KeyValueStoreInsert, Value};
use std::borrow::{Borrow, Cow};
use std::collections::VecDeque;
use std::hash::Hash;

/// Small map implementation
#[derive(Debug)]
pub struct SmallMap<K, V> {
    inner: VecDeque<(K, V)>,
}

impl<K, V> Default for SmallMap<K, V> {
    fn default() -> Self {
        Self {
            inner: VecDeque::with_capacity(32),
        }
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreInsert<K, V> for SmallMap<K, V> {
    fn insert(&mut self, k: K, v: V) {
        for (key, value) in self.inner.iter_mut() {
            if *key == k {
                *value = v;
                return;
            }
        }
        self.inner.push_back((k, v));
    }

    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F) {
        if self.inner.iter().all(|(key, _)| key.as_ref() != k.as_ref()) {
            self.inner.push_back((k, default()));
        }
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStoreGet<K, V> for SmallMap<K, V> {
    fn get<Q: ?Sized>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + AsRef<[u8]>,
    {
        self.inner
            .iter()
            .find(|(key, _)| key.as_ref() == k.as_ref())
            .map(|(_, value)| Cow::Borrowed(value))
    }
}

impl<K: Ord + Hash + Eq + AsRef<[u8]>, V: Value> KeyValueStore<K, V> for SmallMap<K, V> {}
