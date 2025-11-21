/// re-export types from alloy_consensus
pub mod consensus {
    pub use alloy_consensus::{
        Block, BlockHeader, Header, SignableTransaction, Transaction, TxEip1559, TxEip2930,
        TxEip4844, TxEip4844Variant, TxEip4844WithSidecar, TxEip7702, TxLegacy, Typed2718,
        transaction::{SignerRecoverable, TxHashRef},
    };

    #[cfg(not(feature = "scroll"))]
    pub use alloy_consensus::{TxType, TypedTransaction};
    #[cfg(not(feature = "scroll"))]
    /// The Ethereum [EIP-2718] Transaction Envelope.
    pub type TxEnvelope = alloy_consensus::EthereumTxEnvelope<TxEip4844>;
    #[cfg(feature = "scroll")]
    pub use scroll_alloy_consensus::{
        ScrollReceiptEnvelope as ReceiptEnvelope, ScrollTransaction,
        ScrollTxEnvelope as TxEnvelope, ScrollTxType as TxType,
        ScrollTypedTransaction as TypedTransaction, TxL1Message,
    };
}
pub use consensus::{Header, TypedTransaction as AlloyTypedTransaction};

/// re-export types from alloy_eips
pub use alloy_eips as eips;

/// re-export types from alloy-evm
#[cfg(feature = "evm-types")]
pub mod evm {
    pub use alloy_evm::precompiles;

    #[cfg(feature = "scroll-evm-types")]
    pub use scroll_alloy_evm::{ScrollBlockExecutor, ScrollPrecompilesFactory};

    #[cfg(feature = "scroll-compress-info")]
    pub use scroll_alloy_evm::{compute_compressed_size, compute_compression_ratio};

    #[cfg(any(feature = "scroll-evm-types", feature = "scroll-compress-info"))]
    pub use scroll_alloy_evm::{ScrollTxCompressionInfo, ScrollTxCompressionInfos};
}

/// re-export types from alloy_network
#[cfg(feature = "network-types")]
pub mod network {
    /// Network definition
    #[cfg(not(feature = "scroll"))]
    pub type Network = alloy_network::Ethereum;
    /// Network definition
    #[cfg(feature = "scroll-network-types")]
    pub type Network = scroll_alloy_network::Scroll;
}
#[cfg(feature = "network-types")]
pub use network::*;

/// re-export types from revm
#[cfg(feature = "revm-types")]
pub mod revm {
    pub use revm::{bytecode::Bytecode, database, precompile, state::AccountInfo};

    #[cfg(not(feature = "scroll"))]
    pub use revm::primitives::hardfork::SpecId;

    #[cfg(feature = "scroll-revm-types")]
    pub use revm_scroll::{ScrollSpecId as SpecId, precompile::ScrollPrecompileProvider};
}

/// re-export types from reth_primitives
pub mod reth {
    /// Re-export types from `reth-primitives-types`
    pub mod primitives {
        pub use reth_primitives::{RecoveredBlock, SealedBlock};

        #[cfg(not(feature = "scroll"))]
        pub use reth_primitives::{Block, BlockBody, EthPrimitives, Receipt, TransactionSigned};
        #[cfg(feature = "scroll")]
        pub use reth_scroll_primitives::{
            ScrollBlock as Block, ScrollBlockBody as BlockBody, ScrollPrimitives as EthPrimitives,
            ScrollReceipt as Receipt, ScrollTransactionSigned as TransactionSigned,
        };

        pub use reth_primitives_traits::transaction::signed::SignedTransaction;
    }

    /// Re-export types from `reth-evm-ethereum`
    #[cfg(feature = "reth-evm-types")]
    pub mod evm {
        pub use reth_evm::*;

        #[cfg(not(feature = "scroll"))]
        pub use reth_evm_ethereum::{EthEvm, EthEvmConfig, RethReceiptBuilder};

        #[cfg(feature = "scroll-reth-evm-types")]
        pub use reth_scroll_evm::{
            ScrollEvmConfig as EthEvmConfig, ScrollRethReceiptBuilder as RethReceiptBuilder,
        };
    }

    #[cfg(feature = "reth-execution-types")]
    pub use reth_execution_types as execution_types;
}

/// re-export types from alloy_rpc_types_eth
pub mod rpc {
    pub use alloy_rpc_types_eth::{Header, TransactionTrait};

    pub use alloy_rpc_types_debug::ExecutionWitness;
    #[cfg(not(feature = "scroll"))]
    pub use alloy_rpc_types_eth::{Transaction, TransactionReceipt, TransactionRequest};
    #[cfg(feature = "scroll")]
    pub use scroll_alloy_rpc_types::{
        ScrollTransactionReceipt as TransactionReceipt,
        ScrollTransactionRequest as TransactionRequest, Transaction,
    };

    /// Transaction object used in RPC.
    #[allow(unused_qualifications)]
    pub type RpcTransaction<T = super::consensus::TxEnvelope> = alloy_rpc_types_eth::Transaction<T>;

    /// Block representation for RPC.
    pub type Block = alloy_rpc_types_eth::Block<Transaction>;
}
