use crate::{DatabaseError, EvmDatabase, EvmExecutor, VerificationError};
use itertools::Itertools;
use sbv_kv::{nohash::NoHashMap, null::NullProvider};
use sbv_primitives::types::BlockWitness;
use sbv_primitives::{
    B256, Bytes, U256,
    chainspec::ChainSpec,
    ext::{BlockWitnessChunkExt, BlockWitnessExt},
    types::reth::primitives::{Block, RecoveredBlock},
};
use sbv_trie::{BlockWitnessTrieExt, TrieNode};
use std::{collections::BTreeMap, sync::Arc};

/// State commit mode for the block witness verification process.
#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize),
    rkyv(derive(Debug, Hash, PartialEq, Eq))
)]
pub enum StateCommitMode {
    /// Commit state by chunk.
    Chunk,
    /// Commit state by block.
    Block,
    /// Use chunk mode first if it fails, fallback to block mode.
    Auto,
}

/// Result of the block witness verification process.
#[derive(Debug)]
pub struct VerifyResult {
    /// Recovered blocks from the witnesses.
    pub blocks: Vec<RecoveredBlock<Block>>,
    /// Pre-state root of the first block.
    pub pre_state_root: B256,
    /// Post-state root after executing the witnesses.
    pub post_state_root: B256,
    /// Withdrawal root after executing the witnesses.
    pub withdraw_root: B256,
    /// Gas used during the verification process.
    pub gas_used: u64,
}

/// Verify the block witness and return the gas used.
pub fn run(
    witnesses: Vec<BlockWitness>,
    chain_spec: Arc<ChainSpec>,
    state_commit_mode: StateCommitMode,
    compression_ratios: Option<
        impl IntoIterator<Item = impl IntoIterator<Item = impl Into<sbv_primitives::U256>>> + Clone,
    >,
) -> Result<VerifyResult, VerificationError> {
    if witnesses.is_empty() {
        return Err(VerificationError::EmptyWitnesses);
    }
    if !witnesses.has_same_chain_id() {
        return Err(VerificationError::ChainIdMismatch);
    }
    if !witnesses.has_seq_block_number() {
        return Err(VerificationError::NonSequentialWitnesses);
    }

    let (code_db, nodes_provider) = make_providers(&*witnesses);
    let code_db = manually_drop_on_zkvm!(code_db);
    let nodes_provider = manually_drop_on_zkvm!(nodes_provider);

    let pre_state_root = witnesses[0].prev_state_root;
    let blocks = witnesses
        .into_iter()
        .map(|w| {
            dev_trace!("{w:#?}");
            w.into_reth_block()
        })
        .collect::<Result<Vec<RecoveredBlock<Block>>, _>>()?;

    let mut args = ExecuteInnerArgs {
        code_db: &code_db,
        nodes_provider: &nodes_provider,
        pre_state_root,
        blocks: &blocks,
        chain_spec: chain_spec.clone(),
        defer_commit: true,
        compression_ratios,
    };

    let result = match state_commit_mode {
        StateCommitMode::Chunk | StateCommitMode::Block => {
            args.defer_commit = matches!(state_commit_mode, StateCommitMode::Chunk);
            execute(args)?
        }
        StateCommitMode::Auto => match execute(args.clone()) {
            Ok(result) => result,
            Err(VerificationError::Database(DatabaseError::PartialStateTrie(_e))) => {
                dev_warn!(
                    "Failed to execute with defer commit enabled: {_e}; retrying with defer commit disabled"
                );
                #[cfg(target_os = "zkvm")]
                {
                    println!("failed to update db: {_e}; retrying with defer commit disabled");
                }
                args.defer_commit = false;
                execute(args)?
            }
            Err(e) => return Err(e),
        },
    };

    let (post_state_root, withdraw_root, gas_used) = result;

    Ok(VerifyResult {
        blocks,
        pre_state_root,
        post_state_root,
        withdraw_root,
        gas_used,
    })
}

type CodeDb = NoHashMap<B256, Bytes>;
type NodesProvider = NoHashMap<B256, TrieNode>;

/// Create the providers needed for the EVM executor from a list of witnesses.
#[inline]
fn make_providers(witnesses: &[BlockWitness]) -> (CodeDb, NodesProvider) {
    let code_db = {
        // build code db
        let num_codes = witnesses.iter().map(|w| w.codes.len()).sum();
        let mut code_db =
            NoHashMap::<B256, Bytes>::with_capacity_and_hasher(num_codes, Default::default());
        witnesses.import_codes(&mut code_db);
        code_db
    };
    let nodes_provider = {
        let num_states = witnesses.iter().map(|w| w.states.len()).sum();
        let mut nodes_provider =
            NoHashMap::<B256, TrieNode>::with_capacity_and_hasher(num_states, Default::default());
        witnesses.import_nodes(&mut nodes_provider).unwrap();
        nodes_provider
    };

    (code_db, nodes_provider)
}

#[derive(Clone)]
pub(super) struct ExecuteInnerArgs<'a, I> {
    pub(super) code_db: &'a CodeDb,
    pub(super) nodes_provider: &'a NodesProvider,
    pub(super) pre_state_root: B256,
    pub(super) blocks: &'a [RecoveredBlock<Block>],
    pub(super) chain_spec: Arc<ChainSpec>,
    pub(super) defer_commit: bool,
    pub(super) compression_ratios: Option<I>,
}

#[inline]
fn execute<II, I, R>(
    ExecuteInnerArgs {
        code_db,
        nodes_provider,
        pre_state_root,
        blocks,
        chain_spec,
        defer_commit,
        compression_ratios,
    }: ExecuteInnerArgs<II>,
) -> Result<(B256, B256, u64), VerificationError>
where
    II: IntoIterator<Item = I>,
    I: IntoIterator<Item = R>,
    R: Into<U256>,
{
    let mut gas_used = 0;

    let mut db = manually_drop_on_zkvm!(EvmDatabase::new_from_root(
        code_db,
        pre_state_root,
        nodes_provider,
        NullProvider,
    )?);

    let mut execute_block = |block, compression_ratio| -> Result<(), VerificationError> {
        let output = manually_drop_on_zkvm!(
            EvmExecutor::new(chain_spec.clone(), &db, block, compression_ratio).execute()?
        );

        gas_used += output.gas_used;

        db.update(
            nodes_provider,
            BTreeMap::from_iter(output.state.state.clone()).iter(),
        )?;

        if !defer_commit {
            let post_state_root = db.commit_changes();
            if block.state_root != post_state_root {
                dev_error!(
                    "Block #{} root mismatch: root after in trace = {:x}, root after in reth = {:x}",
                    block.number,
                    block.state_root,
                    post_state_root
                );
                return Err(VerificationError::block_root_mismatch(
                    block.state_root,
                    post_state_root,
                    #[cfg(not(target_os = "zkvm"))]
                    output.state,
                ));
            }
            dev_info!("Block #{} verified successfully", block.number);
        } else {
            dev_info!("Block #{} executed successfully", block.number);
        }

        Ok(())
    };

    if let Some(compression_ratios) = compression_ratios {
        for (block, compression_ratios) in blocks.iter().zip_eq(compression_ratios) {
            execute_block(
                block,
                Some(compression_ratios.into_iter().map(|u| u.into())),
            )?;
        }
    } else {
        for block in blocks {
            execute_block(block, None)?;
        }
    }

    let post_state_root = db.commit_changes();
    let expected_state_root = blocks.last().unwrap().state_root;
    if expected_state_root != post_state_root {
        dev_error!(
            "Final state root mismatch: expected {expected_state_root:x}, found {post_state_root:x}",
        );
        return Err(VerificationError::chunk_root_mismatch(
            expected_state_root,
            post_state_root,
        ));
    }
    let withdraw_root = db.withdraw_root()?;
    Ok((post_state_root, withdraw_root, gas_used))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbv_primitives::{
        U256,
        chainspec::{Chain, build_chain_spec_force_hardfork},
        hardforks::Hardfork,
        types::BlockWitness,
    };

    #[rstest::rstest]
    fn test_euclid_v1(
        #[files("../../testdata/scroll_witness/euclidv1/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::Euclid);
        run(
            &[witness],
            chain_spec,
            StateCommitMode::Block,
            None::<Vec<Vec<U256>>>,
        )
        .unwrap();
    }

    #[rstest::rstest]
    fn test_euclid_v2(
        #[files("../../testdata/scroll_witness/euclidv2/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::EuclidV2);
        run(
            vec![witness],
            chain_spec,
            StateCommitMode::Block,
            None::<Vec<Vec<U256>>>,
        )
        .unwrap();
    }

    #[rstest::rstest]
    fn test_feynman(
        #[files("../../testdata/scroll_witness/feynman/**/*.json")]
        #[mode = str]
        witness_json: &str,
    ) {
        let witness: BlockWitness = serde_json::from_str(witness_json).unwrap();
        let chain_spec =
            build_chain_spec_force_hardfork(Chain::from_id(witness.chain_id), Hardfork::Feynman);
        run(
            vec![witness],
            chain_spec,
            StateCommitMode::Block,
            None::<Vec<Vec<U256>>>,
        )
        .unwrap();
    }
}
