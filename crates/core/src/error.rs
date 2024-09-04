use revm::primitives::alloy_primitives::SignatureError;
use revm::primitives::{EVMError, B256};
use sbv_primitives::U256;
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
    #[error("invalid signature for tx_hash={tx_hash}: {source}")]
    InvalidSignature {
        /// The tx hash.
        tx_hash: B256,
        /// The source error that occurred while recovering signer.
        source: SignatureError,
    },
    /// Invalid gas price
    #[error("invalid gas price for tx_hash={tx_hash}: ty={ty}, max_fee_per_gas={max_fee_per_gas}, max_priority_fee_per_gas={max_priority_fee_per_gas:?}, base_fee_per_gas={base_fee_per_gas:?}")]
    InvalidGasPrice {
        /// The tx hash.
        tx_hash: B256,
        /// The transaction type.
        ty: u8,
        /// The EIP-1559 the maximum fee per gas the caller is willing to pay.
        ///
        /// For legacy transactions this is `gas_price`.
        ///
        /// This is also commonly referred to as the "Gas Fee Cap".
        max_fee_per_gas: u128,
        /// The EIP-1559 Priority fee the caller is paying to the block author.
        ///
        /// This is `None` for non-EIP1559 transactions
        max_priority_fee_per_gas: Option<u128>,
        /// The base fee per gas.
        base_fee_per_gas: Option<U256>,
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
