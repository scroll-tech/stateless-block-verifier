/// re-export types from alloy_consensus
pub mod consensus {
    pub use alloy_consensus::{
        Block, BlockHeader, Header, SignableTransaction, Transaction, TxEip1559, TxEip2930,
        TxEip4844, TxEip4844Variant, TxEip4844WithSidecar, TxEip7702, TxLegacy, Typed2718,
        transaction::SignerRecoverable,
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
    pub use scroll_alloy_evm::{
        ScrollBlockExecutor, ScrollPrecompilesFactory, ScrollTxCompressionRatios,
    };

    #[cfg(feature = "scroll-compress-ratio")]
    pub use scroll_alloy_evm::compute_compression_ratio;
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
        pub use reth_primitives::RecoveredBlock;

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
    pub type RpcTransaction<T = super::consensus::TxEnvelope> = alloy_rpc_types_eth::Transaction<T>;

    /// Block representation for RPC.
    pub type Block = alloy_rpc_types_eth::Block<Transaction>;
}

/// Witness type
pub mod witness {
    use crate::{
        B256, Bytes, ChainId, SignatureError, U256,
        types::{
            Header,
            consensus::{SignerRecoverable, TxEnvelope},
            eips::eip4895::Withdrawals,
            reth::primitives::{Block, BlockBody, RecoveredBlock},
        },
    };
    use reth_primitives_traits::serde_bincode_compat::BincodeReprFor;

    /// Witness for a block.
    #[serde_with::serde_as]
    #[derive(Debug, Clone, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct BlockWitness {
        /// Chain id
        pub chain_id: ChainId,
        /// Block header representation.
        #[serde_as(as = "BincodeReprFor<'_, Header>")]
        pub header: Header,
        /// State trie root before the block.
        pub prev_state_root: B256,
        /// Transactions in the block.
        #[serde_as(as = "Vec<BincodeReprFor<'_, TxEnvelope>>")]
        pub transactions: Vec<TxEnvelope>,
        /// Withdrawals in the block.
        pub withdrawals: Option<Withdrawals>,
        /// Last 256 Ancestor block hashes.
        #[cfg(not(feature = "scroll"))]
        pub block_hashes: Vec<B256>,
        /// Rlp encoded state trie nodes.
        pub states: Vec<Bytes>,
        /// Code bytecodes
        pub codes: Vec<Bytes>,
    }

    impl BlockWitness {
        /// Calculates compression ratios for all transactions in the block witness.
        ///
        /// # Panics
        ///
        /// Panics if called without the "scroll-compress-ratio" feature enabled, as this
        /// functionality is not intended to be used in guest environments.
        pub fn compression_ratios(&self) -> Vec<U256> {
            #[cfg(feature = "scroll-compress-ratio")]
            {
                use crate::types::consensus::Transaction;

                self.transactions
                    .iter()
                    .map(|tx| crate::types::evm::compute_compression_ratio(&tx.input()))
                    .collect()
            }
            #[cfg(not(feature = "scroll-compress-ratio"))]
            {
                unimplemented!("you should not build ChunkWitness in guest?");
            }
        }

        /// Converts the `BlockWitness` into a legacy `BlockWitness`.
        pub fn into_legacy(self) -> crate::legacy_types::BlockWitness {
            crate::legacy_types::BlockWitness {
                chain_id: self.chain_id,
                header: self.header.into(),
                pre_state_root: self.prev_state_root,
                transaction: self.transactions.into_iter().map(Into::into).collect(),
                withdrawals: self
                    .withdrawals
                    .map(|w| w.into_iter().map(Into::into).collect()),
                #[cfg(not(feature = "scroll"))]
                block_hashes: self.block_hashes,
                states: self.states,
                codes: self.codes,
            }
        }

        /// Build a reth block
        pub fn into_reth_block(self) -> Result<RecoveredBlock<Block>, SignatureError> {
            let senders = self
                .transactions
                .iter()
                .map(|tx| tx.recover_signer())
                .collect::<Result<Vec<_>, _>>()
                .expect("Failed to recover signer");

            let body = BlockBody {
                transactions: self.transactions,
                ommers: vec![],
                withdrawals: self.withdrawals,
            };

            Ok(RecoveredBlock::new_unhashed(
                Block {
                    header: self.header,
                    body,
                },
                senders,
            ))
        }
    }
}
pub use witness::BlockWitness;

#[cfg(test)]
#[cfg(feature = "scroll")]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    #[rstest::rstest]
    fn serde_scroll_blocks_roundtrip(
        #[files("../../testdata/scroll_witness/**/*.json")]
        #[mode = path]
        path: PathBuf,
    ) {
        let file_content = std::fs::read_to_string(path).unwrap();
        let witness: BlockWitness = serde_json::from_str(&file_content).unwrap();
        let serialized = serde_json::to_string(&witness).unwrap();
        let deserialized: BlockWitness = serde_json::from_str(&serialized).unwrap();
        assert_eq!(witness, deserialized);
    }

    #[rstest::rstest]
    fn serde_scroll_blocks_legacy_compatibility(
        #[files("../../testdata/scroll_witness/**/*.json")]
        #[mode = path]
        path: PathBuf,
    ) {
        let file_content = std::fs::read_to_string(&path).unwrap();
        let witness: BlockWitness = serde_json::from_str(&file_content).unwrap();

        let base_dir = path
            .ancestors()
            .find(|p| p.file_name().unwrap() == OsStr::new("testdata"))
            .unwrap();
        let filename = path.file_name().unwrap();
        let harfork = path.parent().unwrap().file_name().unwrap();
        let legacy_path = base_dir
            .join("legacy")
            .join("scroll_witness")
            .join(harfork)
            .join(filename);
        let legacy_content = std::fs::read_to_string(legacy_path).unwrap();
        let mut legacy_witness: crate::legacy_types::BlockWitness =
            serde_json::from_str(&legacy_content).unwrap();
        legacy_witness.states = Vec::from_iter(BTreeSet::from_iter(legacy_witness.states));
        legacy_witness.codes = Vec::from_iter(BTreeSet::from_iter(legacy_witness.codes));

        let mut legacy_converted = witness.into_legacy();
        legacy_converted.states = Vec::from_iter(BTreeSet::from_iter(legacy_converted.states));
        legacy_converted.codes = Vec::from_iter(BTreeSet::from_iter(legacy_converted.codes));
        assert_eq!(legacy_converted, legacy_witness);
    }
}
