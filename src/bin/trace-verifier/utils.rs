use eth_types::l2_types::BlockTrace;
use eth_types::ToWord;
use stateless_block_verifier::EvmExecutor;

pub fn verify(l2_trace: BlockTrace, disable_checks: bool) {
    trace!("{:#?}", l2_trace);
    let root_after = l2_trace.storage_trace.root_after.to_word();
    info!("Root after in trace: {:x}", root_after);

    let now = std::time::Instant::now();

    #[cfg(feature = "profiling")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let mut executor = EvmExecutor::new(&l2_trace, disable_checks);
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
        std::process::exit(1);
    }
    info!("Root matches in: {} ms", elapsed.as_millis());
}
