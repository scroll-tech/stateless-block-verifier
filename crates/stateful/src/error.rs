use alloy::transports::{RpcError, TransportErrorKind};
use sbv::primitives::zk_trie::{hash::poseidon::PoseidonError, trie::ZkTrieError};

/// Stateful block verifier error
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Provider error
    #[error(transparent)]
    Provider(#[from] RpcError<TransportErrorKind>),
    /// Sled error
    #[error(transparent)]
    Sled(#[from] sled::Error),
    /// Zktrie error
    #[error(transparent)]
    Zktrie(#[from] ZkTrieError<PoseidonError, sled::Error>),
    /// Evm database error
    #[error(transparent)]
    EvmDatabase(#[from] sbv::core::DatabaseError),
    /// Evm verification error
    #[error(transparent)]
    EvmVerification(#[from] sbv::core::VerificationError),
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
