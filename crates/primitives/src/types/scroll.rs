use crate::B256;
use serde::{Deserialize, Serialize};

/// RPC response of the `scroll_diskRoot` method.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct DiskRoot {
    /// MPT state root
    #[serde(rename = "diskRoot")]
    pub disk_root: B256,
    /// B-MPT state root
    #[serde(rename = "headerRoot")]
    pub header_root: B256,
}
