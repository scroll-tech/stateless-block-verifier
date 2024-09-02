use mpt_zktrie::ZktrieState;
use sbv_core::{EvmExecutorBuilder, HardforkConfig, VerificationError};
use sbv_primitives::BlockTrace;
use sbv_utils::post_check;

pub fn verify<T: BlockTrace + Clone>(
    l2_trace: T,
    fork_config: &HardforkConfig,
    disable_checks: bool,
) -> Result<(), VerificationError> {
    measure_duration_histogram!(
        total_block_verification_duration_microseconds,
        verify_inner(l2_trace, fork_config, disable_checks)
    )
}

fn verify_inner<T: BlockTrace + Clone>(
    l2_trace: T,
    fork_config: &HardforkConfig,
    disable_checks: bool,
) -> Result<(), VerificationError> {
    dev_trace!("{l2_trace:#?}");
    let root_after = l2_trace.root_after();

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

    let mut zktrie_state = cycle_track!(
        {
            let old_root = l2_trace.root_before();
            let mut zktrie_state = ZktrieState::construct(old_root);
            measure_duration_histogram!(
                build_zktrie_state_duration_microseconds,
                l2_trace.build_zktrie_state(&mut zktrie_state)
            );
            zktrie_state
        },
        "build ZktrieState"
    );

    let mut executor = EvmExecutorBuilder::new(&zktrie_state)
        .hardfork_config(*fork_config)
        .with_execute_hooks(|hooks| {
            if !disable_checks {
                hooks.add_post_tx_execution_handler(|executor, tx_id| {
                    if let Some(execution_result) = l2_trace.execution_results(tx_id) {
                        post_check(executor.db(), execution_result);
                    } else {
                        dev_warn!("No execution result found in trace but post check is enabled");
                    }
                })
            }
        })
        .build(&l2_trace)?;

    // TODO: change to Result::inspect_err when sp1 toolchain >= 1.76
    #[allow(clippy::map_identity)]
    #[allow(clippy::manual_inspect)]
    executor.handle_block(&l2_trace).map_err(|e| {
        dev_error!(
            "Error occurs when executing block #{}({:?}): {e:?}",
            l2_trace.number(),
            l2_trace.block_hash()
        );

        update_metrics_counter!(verification_error);
        e
    })?;
    let revm_root_after = executor.commit_changes(&mut zktrie_state);

    #[cfg(feature = "profiling")]
    if let Ok(report) = guard.report().build() {
        let dir = std::env::temp_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("profiling");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("block-{}.svg", l2_trace.number()));
        let file = std::fs::File::create(&path).unwrap();
        report.flamegraph(file).unwrap();
        dev_info!("Profiling report saved to: {:?}", path);
    }

    if root_after != revm_root_after {
        dev_error!(
            "Block #{}({:?}) root mismatch: root after in trace = {root_after:x}, root after in revm = {revm_root_after:x}",
            l2_trace.number(),
            l2_trace.block_hash(),
        );

        update_metrics_counter!(verification_error);

        return Err(VerificationError::RootMismatch {
            root_trace: root_after,
            root_revm: revm_root_after,
        });
    }
    dev_info!(
        "Block #{}({}) verified successfully",
        l2_trace.number(),
        l2_trace.block_hash(),
    );
    Ok(())
}
