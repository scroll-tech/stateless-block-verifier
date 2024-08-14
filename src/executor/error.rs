use std::error::Error;

use eth_types::{types::SignatureError, Address, H256, U256};
use revm::primitives::EVMError;

use crate::ReadOnlyDB;

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(thiserror::Error, Debug)]
pub enum VerificationError {
    /// Error while recovering signer from an ECDSA signature.
    #[error("failed to recover signer from signature for tx_hash={tx_hash}: {source}")]
    SignerRecovery {
        /// The tx hash.
        tx_hash: H256,
        /// The source error that occurred while recovering signer.
        source: SignatureError,
    },
    /// The signer recovered from the tx's signature does not match the stated tx sender.
    #[error("recovered signer does not match tx sender for tx_hash={tx_hash}: sender={sender}, signer={signer}")]
    SenderSignerMismatch {
        /// The tx hash.
        tx_hash: H256,
        /// The tx sender address.
        sender: Address,
        /// The signer address recovered from tx signature.
        signer: Address,
    },
    /// Error encountered from [`revm`].
    #[error("error encountered from revm for tx_hash={tx_hash}: {source}")]
    EvmExecution {
        /// The tx hash.
        tx_hash: H256,
        /// The source error originating in [`revm`].
        source: EVMError<<ReadOnlyDB as revm::Database>::Error>,
    },

    /// Root mismatch error
    #[error("root_after in trace doesn't match with root_after in revm: root_trace={root_trace}, root_revm={root_revm}")]
    RootMismatch {
        /// Root after in trace
        root_trace: U256,
        /// Root after in revm
        root_revm: U256,
    },

    /// Error encountered from [`rkyv`]
    #[error("error encountered from rkyv: {0}")]
    RkyvError(String),
}
