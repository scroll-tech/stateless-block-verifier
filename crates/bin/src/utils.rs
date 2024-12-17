use sbv::{
    core::{BlockExecutionOutcome, EvmDatabase, EvmExecutor, VerificationError},
    primitives::{
        chainspec::{get_chain_spec, Chain},
        BlockWitness, Bytes, B256,
    },
    trie::{decode_nodes, TrieNode},
};
use std::collections::HashMap;

pub fn verify<T: BlockWitness>(witness: T) -> Result<(), VerificationError> {
    measure_duration_millis!(
        total_block_verification_duration_milliseconds,
        verify_inner(witness)
    )
}

fn verify_inner<T: BlockWitness>(witness: T) -> Result<(), VerificationError> {
    dev_trace!("{witness:#?}");

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let chain_spec = get_chain_spec(Chain::from_id(witness.chain_id())).unwrap();

    let mut code_db = HashMap::<B256, Bytes>::new();
    witness.import_codes(&mut code_db);
    let mut nodes_provider = HashMap::<B256, TrieNode>::new();
    decode_nodes(&mut nodes_provider, witness.states_iter()).unwrap();
    let db = EvmDatabase::new_from_root(code_db, witness.pre_state_root(), nodes_provider);

    let block = witness.build_reth_block()?;

    let BlockExecutionOutcome {
        post_state_root, ..
    } = EvmExecutor::new(chain_spec, db, &block)
        .execute()
        .inspect_err(|e| {
            dev_error!("Error occurs when executing block #{}: {e:?}", block.number);

            update_metrics_counter!(verification_error);
        })?;

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
