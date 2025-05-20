//! Verifier helpers
use anyhow::anyhow;
use sbv_core::{EvmDatabase, EvmExecutor, VerificationError};
#[cfg(feature = "dev")]
use sbv_helpers::tracing;
use sbv_kv::nohash::NoHashMap;
use sbv_primitives::{
    B256, BlockWitness, Bytes,
    chainspec::{Chain, ChainSpec, get_chain_spec},
    ext::{BlockWitnessChunkExt, BlockWitnessExt},
    types::reth::{Block, BlockWitnessRethExt, RecoveredBlock},
};
use sbv_trie::{BlockWitnessTrieExt, TrieNode};
use std::{
    panic::{UnwindSafe, catch_unwind},
    sync::Arc,
};

/// The code database provider
pub type CodeDb = NoHashMap<B256, Bytes>;

/// The trie nodes provider
pub type NodesProvider = NoHashMap<B256, TrieNode>;

/// The block hash provider for ethereum
#[cfg(not(feature = "scroll"))]
pub type BlockHashProvider = NoHashMap<u64, B256>;

/// The no-op block hash provider for scroll
#[cfg(feature = "scroll")]
pub type BlockHashProvider = sbv_kv::null::NullProvider;

/// The output of the verification process
#[derive(Debug, Clone)]
pub struct VerifyOutput {
    /// The gas used measured by the executor
    pub gas_used: u64,
    /// The chainspec built from the witnesses
    pub chain_spec: Arc<ChainSpec>,
    /// The root of the withdraw tree
    #[cfg(feature = "scroll")]
    pub withdraw_root: B256,
    /// The built blocks from the witnesses
    pub blocks: Vec<RecoveredBlock<Block>>,
}

/// Run verify witness and catches panics.
#[cfg_attr(
    feature = "dev",
    tracing::instrument(
        skip_all,
        fields(start_block = witnesses.first().map(|w| w.number()), end_block = witnesses.last().map(|w| w.number())),
        err
    )
)]
pub fn verify_catch_panics<'a, T>(witnesses: &'a [T]) -> anyhow::Result<VerifyOutput>
where
    T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt,
    &'a [T]: UnwindSafe,
{
    catch_unwind(|| verify(witnesses))
        .map_err(|e| {
            e.downcast_ref::<&str>()
                .map(|s| anyhow!("task panics with: {s}"))
                .or_else(|| {
                    e.downcast_ref::<String>()
                        .map(|s| anyhow!("task panics with: {s}"))
                })
                .unwrap_or_else(|| anyhow!("task panics"))
        })
        .and_then(|r| r.map_err(anyhow::Error::from))
}

/// Run verify witness.
#[cfg_attr(
    feature = "dev",
    tracing::instrument(
        skip_all,
        fields(start_block = witnesses.first().map(|w| w.number()), end_block = witnesses.last().map(|w| w.number())),
        err
    )
)]
pub fn verify<T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt>(
    witnesses: &[T],
) -> Result<VerifyOutput, VerificationError> {
    measure_duration_millis!(
        total_block_verification_duration_milliseconds,
        verify_inner(witnesses)
    )
}

/// Make providers for the witnesses
pub fn make_providers<W: BlockWitness>(
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
    #[cfg(not(feature = "scroll"))]
    let block_hashes = {
        let mut block_hashes =
            NoHashMap::with_capacity_and_hasher(witnesses.len(), Default::default());
        witnesses.import_block_hashes(&mut block_hashes);
        block_hashes
    };
    #[cfg(feature = "scroll")]
    let block_hashes = sbv_kv::null::NullProvider;

    (code_db, nodes_provider, block_hashes)
}

fn verify_inner<W: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt>(
    witnesses: &[W],
) -> Result<VerifyOutput, VerificationError> {
    dev_trace!("{witnesses:#?}");

    if witnesses.is_empty() {
        return Err(VerificationError::EmptyChunk);
    }
    if !witnesses.has_same_chain_id() {
        return Err(VerificationError::ExpectSameChainId);
    }
    if !witnesses.has_seq_block_number() {
        return Err(VerificationError::ExpectSequentialBlockNumber);
    }

    let _start_block = witnesses.first().unwrap().number();
    let _last_block = witnesses.last().unwrap().number();

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let pre_state_root = witnesses[0].pre_state_root();
    let chain = Chain::from_id(witnesses[0].chain_id());

    let chain_spec = get_chain_spec(chain).unwrap_or_else(|| {
        dev_warn!("chain not found, defaults to dev");
        #[cfg(not(feature = "scroll"))]
        {
            sbv_primitives::chainspec::DEV.clone()
        }
        #[cfg(feature = "scroll")]
        {
            sbv_primitives::chainspec::SCROLL_DEV.clone()
        }
    });

    let (code_db, nodes_provider, block_hashes) = make_providers(witnesses);
    #[allow(clippy::redundant_locals)]
    let nodes_provider = manually_drop_on_zkvm!(nodes_provider);
    #[allow(clippy::redundant_locals)]
    let block_hashes = manually_drop_on_zkvm!(block_hashes);

    let mut db = manually_drop_on_zkvm!(EvmDatabase::new_from_root(
        code_db,
        pre_state_root,
        &nodes_provider,
        block_hashes
    )?);

    let mut gas_used = 0;
    let blocks = witnesses
        .iter()
        .map(|w| w.build_reth_block())
        .collect::<Result<Vec<_>, _>>()?;

    for block in blocks.iter() {
        let output =
            manually_drop_on_zkvm!(EvmExecutor::new(chain_spec.clone(), &db, block).execute()?);
        gas_used += output.gas_used;
        db.update(&nodes_provider, output.state.state.iter())?;
    }

    let post_state_root = db.commit_changes();
    #[cfg(feature = "scroll")]
    let withdraw_root = db.withdraw_root()?;

    #[cfg(feature = "profiling")]
    if let Ok(report) = guard.report().build() {
        let dir = std::env::temp_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("profiling");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("block-{_start_block}-{_last_block}.svg"));
        let file = std::fs::File::create(&path).unwrap();
        report.flamegraph(file).unwrap();
        dev_info!("Profiling report saved to: {:?}", path);
    }

    let expect_post_state_root = witnesses.last().unwrap().post_state_root();
    if expect_post_state_root != post_state_root {
        dev_error!(
            "Block #{_start_block}-{_last_block} root mismatch: root after in trace = {expect_post_state_root:x}, root after in reth = {post_state_root:x}",
        );

        update_metrics_counter!(verification_error);

        return Err(VerificationError::root_mismatch(
            expect_post_state_root,
            post_state_root,
        ));
    }
    dev_info!("Block #{_start_block}-{_last_block} verified successfully");

    Ok(VerifyOutput {
        gas_used,
        chain_spec,
        #[cfg(feature = "scroll")]
        withdraw_root,
        blocks,
    })
}
