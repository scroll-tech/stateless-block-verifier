use eth_types::l2_types::BlockTrace;
use eth_types::ToWord;
use stateless_block_verifier::{
    dev_error, dev_info, dev_trace, dev_warn, post_check, EvmExecutorBuilder, HardforkConfig,
    VerificationError,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tiny_keccak::{Hasher, Keccak};

pub fn verify(
    l2_trace: &BlockTrace,
    fork_config: &HardforkConfig,
    disable_checks: bool,
    tx_bytes_hasher: Option<Rc<RefCell<Keccak>>>,
    log_error: bool,
) -> Result<(), VerificationError> {
    static BLOCK_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static LAST_TIME: LazyLock<Mutex<Instant>> = LazyLock::new(|| Mutex::new(Instant::now()));

    dev_trace!("{:#?}", l2_trace);
    let root_after = l2_trace.storage_trace.root_after.to_word();

    // or with v2 trace
    // let v2_trace = BlockTraceV2::from(l2_trace.clone());

    // or with rkyv zero copy
    // let serialized = rkyv::to_bytes::<BlockTraceV2, 4096>(&v2_trace).unwrap();
    // let archived = unsafe { rkyv::archived_root::<BlockTraceV2>(&serialized[..]) };
    // let archived = rkyv::check_archived_root::<BlockTraceV2>(&serialized[..]).unwrap();

    let now = Instant::now();

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

            if let Some(hasher) = tx_bytes_hasher {
                hooks.add_tx_rlp_handler(move |_, rlp| {
                    hasher.borrow_mut().update(rlp);
                });
            }
        })
        .build(&l2_trace);
    let revm_root_after = executor.handle_block(&l2_trace)?.to_word();

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

    let elapsed = now.elapsed();

    if root_after != revm_root_after {
        dev_error!("Root after in trace: {:x}", root_after);
        dev_error!("Root after in revm: {:x}", revm_root_after);
        dev_error!("Root mismatch");

        if !log_error {
            std::process::exit(1);
        }
        return Err(VerificationError::RootMismatch {
            root_trace: root_after,
            root_revm: revm_root_after,
        });
    }

    dev_info!("Root matches in: {} ms", elapsed.as_millis());

    let block_counter = BLOCK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if block_counter > 50 {
        let mut last_time = LAST_TIME.lock().unwrap();
        let blocks = BLOCK_COUNTER.swap(0, std::sync::atomic::Ordering::SeqCst);
        let elapsed = last_time.elapsed().as_secs_f64();
        let bps = blocks as f64 / elapsed;

        dev_warn!("Verifying avg speed: {:.2} bps", bps);
        *last_time = Instant::now();
    }

    Ok(())
}
