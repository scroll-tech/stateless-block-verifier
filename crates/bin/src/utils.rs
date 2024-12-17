use sbv::trie::KeccakKeyHasher;
use sbv::{
    chainspec::{Chain, ChainSpecProvider, WellKnownChainSpecProvider},
    core::{EvmExecutorBuilder, VerificationError},
    primitives::{BlockHeader, BlockWitness, Bytes, B256},
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

    let mut outcome = EvmExecutorBuilder::default()
        .chain_spec(chain_spec)
        .code_db(&mut code_db)
        .nodes_provider(&mut nodes_provider)
        .witness(&witness)
        .build()
        .execute()
        .inspect_err(|e| {
            dev_error!(
                "Error occurs when executing block #{}: {e:?}",
                witness.header().number(),
            );

            update_metrics_counter!(verification_error);
        })?;

    let revm_post_state_root = outcome.post_state_root;

    #[cfg(feature = "profiling")]
    if let Ok(report) = guard.report().build() {
        let dir = std::env::temp_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("profiling");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("block-{}.svg", witness.header().number()));
        let file = std::fs::File::create(&path).unwrap();
        report.flamegraph(file).unwrap();
        dev_info!("Profiling report saved to: {:?}", path);
    }

    if witness.header().state_root() != revm_post_state_root {
        dev_error!(
            "Block #{} root mismatch: root after in trace = {:x}, root after in revm = {:x}",
            witness.header().number(),
            witness.header().state_root(),
            revm_post_state_root
        );

        update_metrics_counter!(verification_error);

        return Err(VerificationError::RootMismatch {
            root_trace: witness.header().state_root(),
            root_revm: revm_post_state_root,
        });
    }
    dev_info!("Block #{} verified successfully", witness.header().number(),);
    Ok(())
}
