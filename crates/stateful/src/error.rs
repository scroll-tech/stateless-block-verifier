use alloy::transports::{RpcError, TransportErrorKind};
use sbv::primitives::zk_trie::{
    hash::{keccak::KeccakError, poseidon::PoseidonError, HashSchemeKind},
    trie::ZkTrieError,
};

/// Stateful block verifier error
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Provider error
    #[error(transparent)]
    Provider(#[from] RpcError<TransportErrorKind>),
    /// Sled error
    #[error(transparent)]
    Sled(#[from] sled::Error),
    /// Json error
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Zktrie error
    #[error(transparent)]
    PoseidonZktrie(#[from] ZkTrieError<PoseidonError, sled::Error>),
    /// Zktrie error
    #[error(transparent)]
    KeccakZktrie(#[from] ZkTrieError<KeccakError, sled::Error>),
    /// Evm database error
    #[error(transparent)]
    EvmDatabase(#[from] sbv::core::DatabaseError),
    /// Evm verification error
    #[error("{hash_scheme_kind:?} evm verification error: {source}")]
    EvmVerification {
        ///
        hash_scheme_kind: HashSchemeKind,
        ///
        source: sbv::core::VerificationError,
    },
    /// Invalid block number
    #[error("expected sequential block")]
    ExpectedSequentialBlock,
    /// Post state root mismatch
    #[error("post state root mismatch")]
    PostStateRootMismatch,
    /// Pipeline shutdown
    #[error("pipeline shutdown")]
    PipelineShutdown,
}
