//! Abstract KV-Store interface.

use auto_impl::auto_impl;
use std::{borrow::Borrow, hash::Hash};

mod imps;
pub use hashbrown;
pub use imps::{nohash, null};

/// HashMap
pub type HashMap<K, V, S = rustc_hash::FxBuildHasher> = hashbrown::HashMap<K, V, S>;
/// HashSet
pub type HashSet<K, V, S = rustc_hash::FxBuildHasher> = hashbrown::HashSet<K, V, S>;

/// Key-Value store insert trait
#[auto_impl(&mut, Box)]
pub trait KeyValueStoreInsert<K: Ord + Hash + Eq, V> {
    /// Insert key-value pair
    fn insert(&mut self, k: K, v: V);
    /// Insert key-value pair if key does not exist
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F);
}

/// Key-Value store trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait KeyValueStoreGet<K: Ord + Hash + Eq, V> {
    /// Get value by key
    fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized;
}

/// Key-Value store trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait KeyValueStore<K: Ord + Hash + Eq, V>:
    KeyValueStoreInsert<K, V> + KeyValueStoreGet<K, V>
{
}
