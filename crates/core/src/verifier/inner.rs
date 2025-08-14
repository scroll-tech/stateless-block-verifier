use crate::{EvmDatabase, EvmExecutor, VerificationError};
use sbv_kv::nohash::NoHashMap;
use sbv_primitives::{
    B256, BlockWitness, Bytes,
    chainspec::ChainSpec,
    ext::BlockWitnessExt,
    types::reth::primitives::{Block, RecoveredBlock},
};
use sbv_trie::{BlockWitnessTrieExt, TrieNode};
use std::{collections::BTreeMap, sync::Arc};

type CodeDb = NoHashMap<B256, Bytes>;

type NodesProvider = NoHashMap<B256, TrieNode>;

#[cfg(feature = "scroll")]
type BlockHashProvider = sbv_kv::null::NullProvider;
#[cfg(not(feature = "scroll"))]
type BlockHashProvider = NoHashMap<u64, B256>;

/// Create the providers needed for the EVM executor from a list of witnesses.
pub(super) fn make_providers<W: BlockWitness>(
    witnesses: &[W],
) -> (CodeDb, NodesProvider, BlockHashProvider) {
    let code_db = {
        // build code db
        let num_codes = witnesses.iter().map(|w| w.codes_iter().len()).sum();
        let mut code_db =
            NoHashMap::<B256, Bytes>::with_capacity_and_hasher(num_codes, Default::default());
        witnesses.import_codes(&mut code_db);
        code_db
    };
    let nodes_provider = {
        let num_states = witnesses.iter().map(|w| w.states_iter().len()).sum();
        let mut nodes_provider =
            NoHashMap::<B256, TrieNode>::with_capacity_and_hasher(num_states, Default::default());
        witnesses.import_nodes(&mut nodes_provider).unwrap();
        nodes_provider
    };
    #[cfg(feature = "scroll")]
    let block_hashes = sbv_kv::null::NullProvider;
    #[cfg(not(feature = "scroll"))]
    let block_hashes = {
        // build block hashes
        let num_blocks = witnesses.iter().map(|w| w.block_hashes_iter().len()).sum();
        let mut block_hashes =
            NoHashMap::<u64, B256>::with_capacity_and_hasher(num_blocks, Default::default());
        witnesses.import_block_hashes(&mut block_hashes);
        block_hashes
    };

    (code_db, nodes_provider, block_hashes)
}

#[derive(Clone)]
pub(super) struct ExecuteInnerArgs<'a, #[cfg(feature = "scroll")] I> {
    pub(super) code_db: &'a CodeDb,
    pub(super) nodes_provider: &'a NodesProvider,
    pub(super) block_hashes: &'a BlockHashProvider,
    pub(super) pre_state_root: B256,
    pub(super) blocks: &'a [RecoveredBlock<Block>],
    pub(super) chain_spec: Arc<ChainSpec>,
    pub(super) defer_commit: bool,
    #[cfg(feature = "scroll")]
    pub(super) compression_ratios: Option<I>,
}

#[cfg(feature = "scroll")]
pub(super) fn execute<II, I, R>(
    ExecuteInnerArgs {
        code_db,
        nodes_provider,
        block_hashes,
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
    R: Into<sbv_primitives::U256>,
{
    use itertools::Itertools;
    use sbv_primitives::hardforks::Hardfork;

    let mut gas_used = 0;

    let mut db = manually_drop_on_zkvm!(EvmDatabase::new_from_root(
        code_db,
        pre_state_root,
        nodes_provider,
        block_hashes
    )?);

    for zip in blocks
        .iter()
        .zip_longest(compression_ratios.into_iter().flat_map(|v| v.into_iter()))
    {
        let (block, compression_ratio) = match zip {
            itertools::EitherOrBoth::Both(block, compression_ratio) => (
                block,
                Some(compression_ratio.into_iter().map(|ratio| ratio.into())),
            ),
            itertools::EitherOrBoth::Left(block) => (block, None),
            itertools::EitherOrBoth::Right(_) => unreachable!(),
        };

        let output = manually_drop_on_zkvm!(
            EvmExecutor::new(chain_spec.clone(), &db, block, compression_ratio).execute()?
        );

        gas_used += output.gas_used;

        if chain_spec.is_fork_active_at_timestamp(Hardfork::Feynman, block.timestamp) {
            db.update(
                nodes_provider,
                BTreeMap::from_iter(output.state.state.clone()).iter(),
            )?
        } else {
            db.update(nodes_provider, output.state.state.clone().iter())?
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "scroll")] {

            } else {
                db.update(
                    nodes_provider,
                    BTreeMap::from_iter(output.state.state.clone()).iter(),
                )?
            }
        }

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
                    output.state,
                ));
            }
            dev_info!("Block #{} verified successfully", block.number);
        } else {
            dev_info!("Block #{} executed successfully", block.number);
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

#[cfg(not(feature = "scroll"))]
pub(super) fn execute(
    ExecuteInnerArgs {
        code_db,
        nodes_provider,
        block_hashes,
        pre_state_root,
        blocks,
        chain_spec,
        defer_commit,
    }: ExecuteInnerArgs,
) -> Result<(B256, u64), VerificationError> {
    let mut gas_used = 0;

    let mut db = manually_drop_on_zkvm!(EvmDatabase::new_from_root(
        code_db,
        pre_state_root,
        nodes_provider,
        block_hashes
    )?);

    for block in blocks.iter() {
        let output =
            manually_drop_on_zkvm!(EvmExecutor::new(chain_spec.clone(), &db, block).execute()?);
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
                    output.state,
                ));
            }
            dev_info!("Block #{} verified successfully", block.number);
        } else {
            dev_info!("Block #{} executed successfully", block.number);
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
    Ok((post_state_root, gas_used))
}
