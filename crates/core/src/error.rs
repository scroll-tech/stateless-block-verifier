use crate::database::DatabaseError;
#[cfg(not(target_os = "zkvm"))]
use sbv_primitives::types::revm::database::BundleState;
use sbv_primitives::{
    B256, alloy_primitives::SignatureError, types::reth::evm::execute::BlockExecutionError,
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
    /// The parent hash of a block does not match the hash of the previous block.
    #[error("parent hash of a block does not match the hash of the previous block")]
    ParentHashMismatch,
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
    RootMismatch {
        /// Root after in trace
        expected: B256,
        /// Root after in revm
        actual: B256,
        /// The bundle state at the time of the mismatch.
        #[cfg(not(target_os = "zkvm"))]
        bundle_state: Box<BundleState>,
    },
}

impl VerificationError {
    /// Create a new [`VerificationError::RootMismatch`] variant.
    #[inline]
    pub fn root_mismatch(
        expected: B256,
        actual: B256,
        #[cfg(not(target_os = "zkvm"))] bundle_state: impl Into<Box<BundleState>>,
    ) -> Self {
        VerificationError::RootMismatch {
            expected,
            actual,
            #[cfg(not(target_os = "zkvm"))]
            bundle_state: bundle_state.into(),
        }
    }
}
