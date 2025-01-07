use sbv::{
    core::{EvmDatabase, EvmExecutor, VerificationError},
    kv::nohash::NoHashMap,
    primitives::{
        chainspec::{get_chain_spec, Chain},
        ext::BlockWitnessExt,
        BlockWitness,
    },
    trie::BlockWitnessTrieExt,
};

pub fn verify<T: BlockWitness + BlockWitnessTrieExt + BlockWitnessExt>(
    witness: T,
) -> Result<(), VerificationError> {
    measure_duration_millis!(
        total_block_verification_duration_milliseconds,
        verify_inner(witness)
    )
}

fn verify_inner<T: BlockWitness + BlockWitnessTrieExt + BlockWitnessExt>(
    witness: T,
) -> Result<(), VerificationError> {
    dev_trace!("{witness:#?}");

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let chain_spec = get_chain_spec(Chain::from_id(witness.chain_id())).unwrap();

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

    let output = EvmExecutor::new(chain_spec, &db, &block)
        .execute()
        .inspect_err(|e| {
            dev_error!("Error occurs when executing block #{}: {e:?}", block.number);

            update_metrics_counter!(verification_error);
        })?;

    db.update(&nodes_provider, output.state.state.iter())?;
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
    Ok(())
}
