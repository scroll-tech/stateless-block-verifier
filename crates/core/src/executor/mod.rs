use crate::{database::EvmDatabase, error::VerificationError};
use reth_evm::execute::{BlockExecutorProvider, Executor};
use reth_evm_ethereum::execute::EthExecutorProvider;
use reth_execution_types::{BlockExecutionInput, ExecutionOutcome};
use reth_primitives::{proofs, BlockWithSenders, Receipts, TransactionSigned};
use revm::{
    db::{AccountState, CacheDB},
    primitives::{AccountInfo, BlobExcessGasAndPrice, BlockEnv, Bytes, Env, TxEnv, KECCAK_EMPTY},
};
use sbv_chainspec::{revm_spec, ChainSpec, Head};
use sbv_kv::KeyValueStore;
use sbv_primitives::alloy_consensus::constants::GWEI_TO_WEI;
use sbv_primitives::alloy_consensus::TxEnvelope;
use sbv_primitives::types::{AlloyHeader, AlloyWithdrawal, AlloyWithdrawals, TypedTransaction};
use sbv_primitives::{BlockHeader, BlockWitness, Withdrawal, B256, U256};
use sbv_trie::{PartialStateTrie, TrieAccount, TrieNode};
use std::fmt::Debug;
use std::sync::Arc;

mod builder;
pub use builder::EvmExecutorBuilder;

/// EVM executor that handles the block.
#[derive(Debug)]
pub struct EvmExecutor<CodeDb, NodesProvider, Witness> {
    chain_spec: Arc<ChainSpec>,
    db: EvmDatabase<CodeDb, NodesProvider>,
    witness: Witness,
}

/// Block execution result
#[derive(Debug)]
pub struct BlockExecutionOutcome {
    /// Gas used in this block
    pub gas_used: u64,
    /// State after
    pub post_state_root: B256,
    /// RLP bytes of transactions
    #[cfg(feature = "scroll")]
    pub tx_rlps: Vec<Bytes>,
}

impl<
        CodeDb: KeyValueStore<B256, Bytes>,
        NodesProvider: KeyValueStore<B256, TrieNode>,
        Witness: BlockWitness,
    > EvmExecutor<CodeDb, NodesProvider, Witness>
{
    /// Handle the block with the given witness
    pub fn execute(self) -> Result<BlockExecutionOutcome, VerificationError> {
        #[allow(clippy::let_and_return)]
        let gas_used = measure_duration_millis!(
            handle_block_duration_milliseconds,
            cycle_track!(self.execute_inner(), "handle_block")
        )?;

        #[cfg(feature = "metrics")]
        sbv_helpers::metrics::REGISTRY.block_counter.inc();

        Ok(gas_used)
    }

    #[inline(always)]
    fn execute_inner(mut self) -> Result<BlockExecutionOutcome, VerificationError> {
        let header = self.witness.header();
        let txs = self
            .witness
            .build_typed_transactions()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let block = reth_primitives::Block {
            header: AlloyHeader {
                parent_hash: header.parent_hash(),
                ommers_hash: header.parent_hash(),
                beneficiary: header.beneficiary(),
                state_root: header.state_root(),
                transactions_root: header.transactions_root(),
                receipts_root: header.receipts_root(),
                logs_bloom: header.logs_bloom(),
                difficulty: header.difficulty(),
                number: header.number(),
                gas_limit: header.gas_limit(),
                gas_used: header.gas_used(),
                timestamp: header.timestamp(),
                extra_data: header.extra_data().clone(),
                mix_hash: header.mix_hash().unwrap(),
                nonce: header.nonce().unwrap(),
                base_fee_per_gas: header.base_fee_per_gas(),
                withdrawals_root: header.withdrawals_root(),
                blob_gas_used: header.blob_gas_used(),
                excess_blob_gas: header.excess_blob_gas(),
                parent_beacon_block_root: header.parent_beacon_block_root(),
                requests_hash: header.requests_hash(),
                target_blobs_per_block: header.target_blobs_per_block(),
            },
            body: reth_primitives::BlockBody {
                transactions: txs
                    .iter()
                    .cloned()
                    .map(|tx| {
                        let TypedTransaction::Enveloped(tx) = tx else {
                            unimplemented!("scroll tx")
                        };
                        match tx {
                            TxEnvelope::Legacy(tx) => TransactionSigned::from(tx),
                            TxEnvelope::Eip2930(tx) => TransactionSigned::from(tx),
                            TxEnvelope::Eip1559(tx) => TransactionSigned::from(tx),
                            TxEnvelope::Eip4844(tx) => {
                                let (tx, sig, hash) = tx.into_parts();
                                TransactionSigned::new(tx.tx().clone().into(), sig, hash)
                            }
                            TxEnvelope::Eip7702(tx) => TransactionSigned::from(tx),
                            _ => unimplemented!("unknown tx type"),
                        }
                    })
                    .collect(),
                ommers: vec![],
                withdrawals: self.witness.withdrawals_iter().map(|w| {
                    AlloyWithdrawals::new(
                        w.map(|w| AlloyWithdrawal {
                            index: w.index(),
                            validator_index: w.validator_index(),
                            address: w.address(),
                            amount: w.amount(),
                        })
                        .collect(),
                    )
                }),
            },
        };
        let senders = txs
            .iter()
            .enumerate()
            .map(|(idx, tx)| {
                tx.get_or_recover_signer()
                    .map_err(|e| VerificationError::InvalidSignature { idx, source: e })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let block_with_senders = BlockWithSenders::new_unchecked(block, senders);
        let difficulty = self.witness.header().difficulty();
        let input = BlockExecutionInput::new(&block_with_senders, difficulty);
        let output = EthExecutorProvider::ethereum(self.chain_spec.clone())
            .executor(CacheDB::new(&self.db))
            .execute(input)
            .unwrap();

        self.db
            .state
            .update(&self.db.nodes_provider, output.state.state);
        let post_state_root = self.db.state.commit_state();

        Ok(BlockExecutionOutcome {
            gas_used: output.gas_used,
            post_state_root,
        })
        // let block_number = header.number();
        // dev_debug!("handle block #{block_number}");
        //
        // let spec_id = revm_spec(
        //     &self.chain_spec,
        //     &Head {
        //         number: block_number,
        //         hash: header.hash(),
        //         timestamp: header.timestamp(),
        //         difficulty: header.difficulty(),
        //         ..Default::default()
        //     },
        // );
        // dev_trace!("use spec id {spec_id:?}");
        //
        // // FIXME: scroll needs migrate on curie block
        //
        // let mut env = Box::<Env>::default();
        // env.cfg.chain_id = self.chain_spec.chain.id();
        // env.block = BlockEnv {
        //     number: U256::from_limbs([block_number, 0, 0, 0]),
        //     coinbase: header.beneficiary(),
        //     timestamp: U256::from_limbs([header.timestamp(), 0, 0, 0]),
        //     gas_limit: U256::from_limbs([header.gas_limit(), 0, 0, 0]),
        //     basefee: U256::from_limbs([header.base_fee_per_gas().unwrap_or_default(), 0, 0, 0]),
        //     difficulty: header.difficulty(),
        //     prevrandao: Some(header.prevrandao()),
        //     blob_excess_gas_and_price: header.excess_blob_gas().map(BlobExcessGasAndPrice::new),
        // };
        //
        // let mut gas_used = 0;
        // let mut tx_rlps = Vec::with_capacity(self.witness.num_transactions());
        //
        // for (idx, tx) in self.witness.build_typed_transactions().enumerate() {
        //     cycle_tracker_start!("handle tx");
        //
        //     dev_trace!("handle {idx}th tx");
        //
        //     let tx = tx.map_err(|e| VerificationError::InvalidSignature { idx, source: e })?;
        //     let caller = tx
        //         .get_or_recover_signer()
        //         .map_err(|e| VerificationError::InvalidSignature { idx, source: e })?;
        //     let rlp_bytes = tx.rlp();
        //     tx_rlps.push(rlp_bytes.clone());
        //
        //     dev_trace!("{tx:#?}");
        //     let mut env = env.clone();
        //     env.tx = TxEnv {
        //         caller,
        //         gas_limit: tx.gas_limit(),
        //         gas_price: tx
        //             .effective_gas_price(header.base_fee_per_gas().unwrap_or_default())
        //             .map(U256::from)
        //             .ok_or_else(|| VerificationError::InvalidGasPrice {
        //                 tx_hash: *tx.tx_hash(),
        //             })?,
        //         transact_to: tx.kind(),
        //         value: tx.value(),
        //         data: tx.input(),
        //         nonce: tx.nonce(),
        //         chain_id: tx.chain_id(),
        //         access_list: tx.access_list().cloned().unwrap_or_default().0,
        //         gas_priority_fee: tx.max_priority_fee_per_gas().map(U256::from),
        //         blob_hashes: tx
        //             .blob_versioned_hashes()
        //             .map(|hashes| hashes.to_vec())
        //             .unwrap_or_default(),
        //         max_fee_per_blob_gas: tx.max_fee_per_blob_gas().map(U256::from),
        //         authorization_list: tx.authorization_list().map(|auths| auths.to_vec().into()),
        //         #[cfg(feature = "scroll")]
        //         scroll: revm::primitives::ScrollFields {
        //             is_l1_msg: tx.is_l1_msg(),
        //             rlp_bytes: Some(rlp_bytes),
        //         },
        //     };
        //
        //     #[cfg(feature = "scroll")]
        //     if tx.is_l1_msg() {
        //         env.cfg.disable_base_fee = true; // disable base fee for l1 msg
        //     }
        //
        //     dev_trace!("{env:#?}");
        //
        //     {
        //         let mut revm = cycle_track!(
        //             revm::Evm::builder()
        //                 .with_spec_id(spec_id)
        //                 .with_db(&mut self.db)
        //                 .with_env(env)
        //                 // .with_external_context(CustomPrintTracer::default())
        //                 // .append_handler_register(inspector_handle_register)
        //                 .build(),
        //             "build Evm"
        //         );
        //
        //         dev_trace!("handler cfg: {:?}", revm.handler.cfg);
        //
        //         let result = measure_duration_millis!(
        //             transact_commit_duration_milliseconds,
        //             cycle_track!(revm.transact_commit(), "transact_commit").map_err(|e| {
        //                 VerificationError::EvmExecution {
        //                     tx_hash: *tx.tx_hash(),
        //                     source: e,
        //                 }
        //             })?
        //         );
        //
        //         gas_used += result.gas_used();
        //
        //         dev_trace!("{result:#?}");
        //     }
        //
        //     dev_debug!("handle {idx}th tx done");
        //     cycle_tracker_end!("handle tx");
        // }
    }
}
