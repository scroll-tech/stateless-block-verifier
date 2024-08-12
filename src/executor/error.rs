use ethers_core::types::{Address, SignatureError, H256};
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
}
