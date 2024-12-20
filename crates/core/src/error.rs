use reth_evm::execute::BlockExecutionError;
use sbv_primitives::{alloy_primitives::SignatureError, B256};

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Error while recovering signer from an ECDSA signature.
    #[error("invalid signature: {0}")]
    InvalidSignature(#[from] SignatureError),
    /// Error encountered from [`revm`].
    #[error(transparent)]
    Execution(#[from] BlockExecutionError),
    /// Root mismatch error
    #[error("state root in trace doesn't match with state root executed: expected {expected}, actual {actual}")]
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
