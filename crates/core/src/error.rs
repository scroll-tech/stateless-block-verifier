use crate::database::DatabaseError;
use sbv_primitives::{
    B256, alloy_primitives::SignatureError, reth::evm::execute::BlockExecutionError,
};

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
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
        "state root in trace doesn't match with state root executed: expected {expected}, actual {actual}"
    )]
    RootMismatch {
        /// Root after in trace
        expected: B256,
        /// Root after in revm
        actual: B256,
    },
}

impl VerificationError {
    /// Create a new [`VerificationError::RootMismatch`] variant.
    #[inline]
    pub fn root_mismatch(expected: B256, actual: B256) -> Self {
        VerificationError::RootMismatch { expected, actual }
    }
}
