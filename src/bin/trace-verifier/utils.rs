use eth_types::l2_types::BlockTrace;
use eth_types::ToWord;
use stateless_block_verifier::{EvmExecutor, HardforkConfig};
use std::time::Instant;

pub fn verify(
    l2_trace: BlockTrace,
    fork_config: &HardforkConfig,
    disable_checks: bool,
    log_error: bool,
) -> bool {
    static BLOCK_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    static LAST_TIME: std::sync::Mutex<Instant> = std::sync::Mutex::new(Instant::now());

    trace!("{:#?}", l2_trace);
    let root_after = l2_trace.storage_trace.root_after.to_word();
    info!("Root after in trace: {:x}", root_after);

    let now = Instant::now();

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let mut executor = EvmExecutor::new(&l2_trace, &fork_config, disable_checks);
    let revm_root_after = executor.handle_block(&l2_trace).to_word();

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
        info!("Profiling report saved to: {:?}", path);
    }

    info!("Root after in revm: {:x}", revm_root_after);
    let elapsed = now.elapsed();

    if root_after != revm_root_after {
        error!("Root mismatch");
        if !log_error {
            std::process::exit(1);
        }
        return false;
    }
    info!("Root matches in: {} ms", elapsed.as_millis());

    let block_counter = BLOCK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if block_counter > 50 {
        let mut last_time = LAST_TIME.lock().unwrap();
        let blocks = BLOCK_COUNTER.swap(0, std::sync::atomic::Ordering::SeqCst);
        let elapsed = last_time.elapsed().as_secs_f64();
        let bps = blocks as f64 / elapsed;
        info!("Blocks per second: {:.2}", bps);
        *last_time = Instant::now();
    }

    true
}
