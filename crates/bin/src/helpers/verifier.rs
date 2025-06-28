use anyhow::anyhow;
#[cfg(feature = "dev")]
use sbv::helpers::tracing;
use sbv::{
    core::{EvmDatabase, EvmExecutor, VerificationError},
    kv::nohash::NoHashMap,
    primitives::{
        chainspec::{Chain, get_chain_spec_or_build},
        ext::{BlockWitnessExt, BlockWitnessRethExt},
    },
    trie::BlockWitnessTrieExt,
};
use std::{
    collections::BTreeMap,
    panic::{UnwindSafe, catch_unwind},
};

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.number()), err))]
pub fn verify_catch_panics<
    T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt + UnwindSafe,
>(
    witness: T,
) -> anyhow::Result<u64> {
    catch_unwind(|| verify(witness))
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

#[cfg_attr(feature = "dev", tracing::instrument(skip_all, fields(block_number = %witness.number()), err))]
pub fn verify<T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt>(
    witness: T,
) -> Result<u64, VerificationError> {
    measure_duration_millis!(
        total_block_verification_duration_milliseconds,
        verify_inner(witness)
    )
}

fn verify_inner<T: BlockWitnessRethExt + BlockWitnessTrieExt + BlockWitnessExt>(
    witness: T,
) -> Result<u64, VerificationError> {
    dev_trace!("{witness:#?}");

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let chain_spec = get_chain_spec_or_build(Chain::from_id(witness.chain_id()), |_spec| {
        #[cfg(feature = "scroll")]
        {
            use sbv::primitives::hardforks::{ForkCondition, ScrollHardfork};
            _spec
                .inner
                .hardforks
                .insert(ScrollHardfork::EuclidV2, ForkCondition::Timestamp(0));
            _spec
                .inner
                .hardforks
                .insert(ScrollHardfork::Feynman, ForkCondition::Never);
        }
    });

    let mut code_db = NoHashMap::default();
    witness.import_codes(&mut code_db);
    let mut nodes_provider = NoHashMap::default();
    witness.import_nodes(&mut nodes_provider).unwrap();
    #[cfg(not(feature = "scroll"))]
    let block_hashes = {
        let mut block_hashes = NoHashMap::default();
        witness.import_block_hashes(&mut block_hashes);
        block_hashes
    };
    #[cfg(feature = "scroll")]
    let block_hashes = &sbv::kv::null::NullProvider;
    let mut db = EvmDatabase::new_from_root(
        code_db,
        witness.pre_state_root(),
        &nodes_provider,
        &block_hashes,
    )?;

    let block = witness.build_reth_block()?;
    let compression_ratios = witness.compression_ratios_iter();

    let output = EvmExecutor::new(chain_spec, &db, &block, compression_ratios)
        .execute()
        .inspect_err(|_e| {
            dev_error!(
                "Error occurs when executing block #{}: {_e:?}",
                block.number
            );

            update_metrics_counter!(verification_error);
        })?;

    db.update(
        &nodes_provider,
        BTreeMap::from_iter(output.state.state.clone()).iter(),
    )?;
    let post_state_root = db.commit_changes();

    #[cfg(feature = "profiling")]
    if let Ok(report) = guard.report().build() {
        let dir = std::env::temp_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("profiling");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("block-{}.svg", block.number));
        let file = std::fs::File::create(&path).unwrap();
        report.flamegraph(file).unwrap();
        dev_info!("Profiling report saved to: {:?}", path);
    }

    if block.state_root != post_state_root {
        dev_error!(
            "Block #{} root mismatch: root after in trace = {:x}, root after in reth = {:x}",
            block.number,
            block.state_root,
            post_state_root
        );

        update_metrics_counter!(verification_error);

        return Err(VerificationError::root_mismatch(
            block.state_root,
            post_state_root,
        ));
    }
    dev_info!("Block #{} verified successfully", block.number);

    Ok(output.gas_used)
}
