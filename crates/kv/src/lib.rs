//! Abstract KV-Store interface.

use auto_impl::auto_impl;
use std::borrow::{Borrow, Cow};
use std::hash::Hash;

mod imps;
pub use hashbrown;
pub use imps::nohash;
/// HashMap
pub type HashMap<K, V, S = rustc_hash::FxBuildHasher> = hashbrown::HashMap<K, V, S>;
/// HashSet
pub type HashSet<K, V, S = rustc_hash::FxBuildHasher> = hashbrown::HashSet<K, V, S>;

/// Value trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait Value: ToOwned<Owned = Self> {
    /// Serialize value to bytes
    ///
    /// # Panics
    ///
    /// This function may panic if the value cannot be serialized.
    #[cfg(feature = "sled")]
    fn serialize(&self) -> Vec<u8>;
    /// Deserialize value from bytes
    ///
    /// # Panics
    ///
    /// This function may panic if the bytes are not a valid encoding of the value.
    #[cfg(feature = "sled")]
    fn deserialize(buf: &[u8]) -> Self;
}

/// Key-Value store insert trait
#[auto_impl(&mut, Box)]
pub trait KeyValueStoreInsert<K: Ord + Hash + Eq, V: Value> {
    /// Insert key-value pair
    fn insert(&mut self, k: K, v: V);
    /// Insert key-value pair if key does not exist
    fn or_insert_with<F: FnOnce() -> V>(&mut self, k: K, default: F);
}

/// Key-Value store trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait KeyValueStoreGet<K: Ord + Hash + Eq, V: Value> {
    /// Get value by key
    fn get<Q>(&self, k: &Q) -> Option<Cow<V>>
    where
        K: Borrow<Q>,
        Q: Ord + Hash + Eq + ?Sized;
}

/// Key-Value store trait
#[auto_impl(&, &mut, Box, Rc, Arc)]
pub trait KeyValueStore<K: Ord + Hash + Eq, V: Value>:
    KeyValueStoreInsert<K, V> + KeyValueStoreGet<K, V>
{
}
