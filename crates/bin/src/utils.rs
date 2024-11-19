use sbv::primitives::zk_trie::db::NodeDb;
use sbv::primitives::zk_trie::hash::keccak::Keccak;
use sbv::primitives::zk_trie::hash::HashSchemeKind;
use sbv::{
    core::{EvmExecutorBuilder, HardforkConfig, VerificationError},
    primitives::{zk_trie::db::kv::HashMapDb, zk_trie::hash::poseidon::Poseidon, Block},
};

pub fn verify<T: Block + Clone>(
    l2_trace: T,
    fork_config: &HardforkConfig,
    hash_scheme: HashSchemeKind,
) -> Result<(), VerificationError> {
    measure_duration_millis!(
        total_block_verification_duration_milliseconds,
        verify_inner(l2_trace, fork_config, hash_scheme)
    )
}

fn verify_inner<T: Block + Clone>(
    l2_trace: T,
    fork_config: &HardforkConfig,
    hash_scheme: HashSchemeKind,
) -> Result<(), VerificationError> {
    dev_trace!("{l2_trace:#?}");
    let root_before = l2_trace.root_before();
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

    let mut zktrie_db = cycle_track!(
        {
            let mut zktrie_db = NodeDb::new(HashMapDb::default());
            measure_duration_millis!(
                build_zktrie_db_duration_milliseconds,
                l2_trace
                    .build_zktrie_db(&mut zktrie_db, hash_scheme)
                    .unwrap()
            );
            zktrie_db
        },
        "build ZktrieState"
    );
    let mut code_db = HashMapDb::default();

    let builder = EvmExecutorBuilder::new(&mut code_db, &mut zktrie_db)
        .hardfork_config(*fork_config)
        .chain_id(l2_trace.chain_id());

    let revm_root_after = match hash_scheme {
        HashSchemeKind::Poseidon => {
            let mut executor = builder.hash_scheme(Poseidon).build(root_before)?;

            executor.insert_codes(&l2_trace)?;

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
            executor.commit_changes()?
        }
        HashSchemeKind::Keccak => {
            let mut executor = builder.hash_scheme(Keccak).build(root_before)?;

            executor.insert_codes(&l2_trace)?;

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
            executor.commit_changes()?
        }
    };

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
