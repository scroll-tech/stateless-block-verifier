//! NoHash is a [`HashMap`] optimized for key already being a hash.
use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};

/// [`HashMap`] optimized for key already being a hash.
pub type NoHashMap<K, V> = HashMap<K, V, NoHashBuildHasher>;

/// A build hasher that does not hash anything.
#[derive(Default, Debug, Copy, Clone)]
pub struct NoHashBuildHasher;

/// A hasher that does not hash anything, truncates input to u64.
///
/// Expect input to be a fairly random distributed slice greater than 8 bytes, like a code hash.
#[derive(Default, Debug, Copy, Clone)]
pub struct NoHashHasher(u64);

impl BuildHasher for NoHashBuildHasher {
    type Hasher = NoHashHasher;
    fn build_hasher(&self) -> Self::Hasher {
        NoHashHasher::default()
    }
}

impl Hasher for NoHashHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // expect the input to be a slice greater than 8 bytes
        self.0 = u64::from_le_bytes(bytes[..8].try_into().unwrap());
    }
}
