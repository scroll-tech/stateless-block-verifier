use revm::primitives::{alloy_primitives::SignatureError, EVMError, B256};
use std::convert::Infallible;

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Error while recovering signer from an ECDSA signature.
    #[error("invalid signature for #{idx} tx: {source}")]
    InvalidSignature {
        /// The idx of the transaction in the block.
        idx: usize,
        /// The source error that occurred while recovering signer.
        source: SignatureError,
    },
    /// Invalid gas price
    #[error("invalid gas price for tx_hash={tx_hash}")]
    InvalidGasPrice {
        /// The tx hash.
        tx_hash: B256,
    },
    /// Error encountered from [`revm`].
    #[error("error encountered from revm for tx_hash={tx_hash}: {source}")]
    EvmExecution {
        /// The tx hash.
        tx_hash: B256,
        /// The source error originating in [`revm`].
        source: EVMError<Infallible>,
    },
    /// Root mismatch error
    #[error("root_after in trace doesn't match with root_after in revm: root_trace={root_trace}, root_revm={root_revm}")]
    RootMismatch {
        /// Root after in trace
        root_trace: B256,
        /// Root after in revm
        root_revm: B256,
    },
}
