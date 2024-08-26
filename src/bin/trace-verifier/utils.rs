use eth_types::l2_types::BlockTrace;
#[cfg(feature = "metrics")]
use stateless_block_verifier::metrics;
use stateless_block_verifier::{post_check, EvmExecutorBuilder, HardforkConfig, VerificationError};

pub fn verify(
    l2_trace: &BlockTrace,
    fork_config: &HardforkConfig,
    disable_checks: bool,
    log_error: bool,
) -> Result<(), VerificationError> {
    measure_duration_histogram!(
        total_block_verification_duration_microseconds,
        verify_inner(l2_trace, fork_config, disable_checks, log_error)
    )
}

fn verify_inner(
    l2_trace: &BlockTrace,
    fork_config: &HardforkConfig,
    disable_checks: bool,
    log_error: bool,
) -> Result<(), VerificationError> {
    dev_trace!("{l2_trace:#?}");
    let root_after = l2_trace.storage_trace.root_after;

    // or with v2 trace
    // let v2_trace = BlockTraceV2::from(l2_trace.clone());

    // or with rkyv zero copy
    // let serialized = rkyv::to_bytes::<BlockTraceV2, 4096>(&v2_trace).unwrap();
    // let archived = unsafe { rkyv::archived_root::<BlockTraceV2>(&serialized[..]) };
    // let archived = rkyv::check_archived_root::<BlockTraceV2>(&serialized[..]).unwrap();

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let mut executor = EvmExecutorBuilder::new()
        .hardfork_config(*fork_config)
        .with_execute_hooks(|hooks| {
            let l2_trace = l2_trace.clone();
            if !disable_checks {
                hooks.add_post_tx_execution_handler(move |executor, tx_id| {
                    post_check(executor.db(), &l2_trace.execution_results[tx_id]);
                })
            }
        })
        .build(&l2_trace);

    // TODO: change to Result::inspect_err when sp1 toolchain >= 1.76
    if let Err(e) = executor.handle_block(&l2_trace) {
        dev_error!(
            "Error occurs when verifying block {:?}: {e:?}",
            l2_trace.header.hash.unwrap()
        );

        #[cfg(feature = "metrics")]
        metrics::REGISTRY.verification_error.inc();

        return Err(e);
    }
    let revm_root_after = executor.commit_changes();

    #[cfg(feature = "profiling")]
    if let Ok(report) = guard.report().build() {
        let dir = std::env::temp_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("profiling");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!(
            "block-{}.svg",
            l2_trace.header.number.unwrap().as_u64()
        ));
        let file = std::fs::File::create(&path).unwrap();
        report.flamegraph(file).unwrap();
        dev_info!("Profiling report saved to: {:?}", path);
    }

    if root_after != revm_root_after {
        dev_error!(
            "Block #{}({:?}) root mismatch: root after in trace = {root_after:x}, root after in revm = {revm_root_after:x}",
            l2_trace.header.number.unwrap().as_u64(),
            l2_trace.header.hash.unwrap()
        );

        #[cfg(feature = "metrics")]
        metrics::REGISTRY.verification_error.inc();

        if !log_error {
            std::process::exit(1);
        }

        return Err(VerificationError::RootMismatch {
            root_trace: root_after,
            root_revm: revm_root_after,
        });
    }
    dev_info!(
        "Block #{}({}) verified successfully",
        l2_trace.header.number.unwrap().as_u64(),
        l2_trace.header.hash.unwrap()
    );
    Ok(())
}
