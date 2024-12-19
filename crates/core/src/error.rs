use revm::primitives::{alloy_primitives::SignatureError, EVMError, B256};
use std::convert::Infallible;

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Error while recovering signer from an ECDSA signature.
    #[error("invalid signature: {0}")]
    InvalidSignature(#[from] SignatureError),
    /// Error encountered from [`revm`].
    #[error("error encountered from revm for tx_hash={tx_hash}: {source}")]
    EvmExecution {
        /// The tx hash.
        tx_hash: B256,
        /// The source error originating in [`revm`].
        source: EVMError<Infallible>,
    },
    /// Root mismatch error
    #[error("root_after in trace doesn't match with root_after in revm: expected {expected}, actual {actual}")]
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
