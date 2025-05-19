use crate::database::DatabaseError;
use reth_evm::execute::BlockExecutionError;
use sbv_primitives::{B256, alloy_primitives::SignatureError};

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Empty chunk
    #[error("empty chunk")]
    EmptyChunk,
    /// Expect same chain id
    #[error("expect same chain id")]
    ExpectSameChainId,
    /// Expect sequential block number
    #[error("expect sequential block number")]
    ExpectSequentialBlockNumber,
    /// Error while recovering signer from an ECDSA signature.
    #[error("invalid signature: {0}")]
    InvalidSignature(#[from] SignatureError),
    /// Error encountered from database.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// Error encountered from [`revm`].
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

    /// Return the hash of the blinded node if the error is a blinded node error.
    pub fn as_blinded_node_err(&self) -> Option<B256> {
        use DatabaseError::PartialStateTrie;
        use VerificationError::Database;
        use sbv_trie::PartialStateTrieError::BlindedNode;

        match self {
            DatabaseCreation(PartialStateTrie(BlindedNode { hash, .. }))
            | DatabaseUpdate(PartialStateTrie(BlindedNode { hash, .. }))
            | GetWithdrawRoot(PartialStateTrie(BlindedNode { hash, .. }))
            | BlockExecution(Database(PartialStateTrie(BlindedNode { hash, .. }))) => Some(*hash),
            _ => None,
        }
    }
}
