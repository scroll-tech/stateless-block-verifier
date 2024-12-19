use crate::Value;
use alloy_primitives::Bytes;

impl Value for Bytes {
    #[cfg(feature = "sled")]
    fn serialize(&self) -> Vec<u8> {
        self.to_vec()
    }

    #[cfg(feature = "sled")]
    fn deserialize(buf: &[u8]) -> Self {
        Bytes::copy_from_slice(buf)
    }
}
