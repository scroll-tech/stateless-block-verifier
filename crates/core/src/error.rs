use eth_types::{types::SignatureError, Address, H256};
use revm::primitives::EVMError;
use std::convert::Infallible;

/// Error variants encountered during manipulation of a zkTrie.
#[derive(Debug, thiserror::Error)]
pub enum ZkTrieError {
    #[error("zktrie root not found")]
    ZkTrieRootNotFound,
}

/// Error variants encountered during verification of transactions in a L2 block.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Malformed trace.
    #[error("malformed trace, unexpected zktrie error: {source}")]
    MalformedTrace {
        /// The source error that occurred while parsing the trace.
        #[from]
        source: ZkTrieError,
    },
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
        source: EVMError<Infallible>,
    },
    /// Root mismatch error
    #[error("root_after in trace doesn't match with root_after in revm: root_trace={root_trace}, root_revm={root_revm}")]
    RootMismatch {
        /// Root after in trace
        root_trace: H256,
        /// Root after in revm
        root_revm: H256,
    },
}
