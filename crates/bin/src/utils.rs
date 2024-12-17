use sbv::core::BlockExecutionOutcome;
use sbv::{
    chainspec::{Chain, ChainSpecProvider, WellKnownChainSpecProvider},
    core::{EvmExecutorBuilder, VerificationError},
    primitives::{BlockWitness, Bytes, B256},
    trie::TrieNode,
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

    let chain_spec = WellKnownChainSpecProvider::new(Chain::from_id(witness.chain_id()))
        .unwrap()
        .chain_spec();
    let mut code_db = HashMap::<B256, Bytes>::new();
    let mut nodes_provider = HashMap::<B256, TrieNode>::new();
    let block = witness.build_reth_block()?;

    let BlockExecutionOutcome {
        post_state_root, ..
    } = EvmExecutorBuilder::default()
        .chain_spec(chain_spec)
        .code_db(&mut code_db)
        .nodes_provider(&mut nodes_provider)
        .witness(&witness)
        .block(&block)
        .build()
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
