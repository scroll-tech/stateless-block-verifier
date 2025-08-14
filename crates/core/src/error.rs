use crate::database::DatabaseError;
use sbv_primitives::{
    B256,
    alloy_primitives::SignatureError,
    types::{reth::evm::execute::BlockExecutionError, revm::database::BundleState},
};

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// The witnesses are empty.
    #[error("witnesses are empty")]
    EmptyWitnesses,
    /// The witnesses are not on the same chain ID.
    #[error("witnesses are not on the same chain ID")]
    ChainIdMismatch,
    /// The witnesses are not sequential.
    #[error("witnesses are not sequential")]
    NonSequentialWitnesses,
    /// Error while recovering signer from an ECDSA signature.
    #[error("invalid signature: {0}")]
    InvalidSignature(#[from] SignatureError),
    /// Error encountered from database.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// Error encountered from [`revm`](sbv_primitives::types::revm).
    #[error(transparent)]
    Execution(#[from] BlockExecutionError),
    /// Root mismatch error
    #[error(
        "state root in witness doesn't match with state root executed: expected {expected}, actual {actual}"
    )]
    BlockRootMismatch {
        /// Root after in trace
        expected: B256,
        /// Root after in revm
        actual: B256,
        /// The bundle state at the time of the mismatch.
        bundle_state: Box<BundleState>,
    },
    /// Root mismatch error
    #[error(
        "state root in last witness doesn't match with state root executed: expected {expected}, actual {actual}"
    )]
    ChunkRootMismatch {
        /// Root after in trace
        expected: B256,
        /// Root after in revm
        actual: B256,
    },
}

impl VerificationError {
    /// Create a new [`VerificationError::BlockRootMismatch`] variant.
    #[inline]
    pub fn block_root_mismatch(
        expected: B256,
        actual: B256,
        bundle_state: impl Into<Box<BundleState>>,
    ) -> Self {
        VerificationError::BlockRootMismatch {
            expected,
            actual,
            bundle_state: bundle_state.into(),
        }
    }

    /// Create a new [`VerificationError::ChunkRootMismatch`] variant.
    #[inline]
    pub fn chunk_root_mismatch(expected: B256, actual: B256) -> Self {
        VerificationError::ChunkRootMismatch { expected, actual }
    }
}
