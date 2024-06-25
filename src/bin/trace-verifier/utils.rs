use eth_types::forks::{hardfork_heights, HardforkId};
use eth_types::l2_types::BlockTrace;
use eth_types::ToWord;
use revm::primitives::SpecId;
use stateless_block_verifier::EvmExecutor;
use std::collections::HashMap;
use std::sync::LazyLock;

pub fn verify(
    l2_trace: BlockTrace,
    curie_block: Option<u64>,
    disable_checks: bool,
    log_error: bool,
) -> bool {
    static HARDFORK_HEIGHTS: LazyLock<HashMap<u64, u64>> = LazyLock::new(|| {
        hardfork_heights()
            .into_iter()
            .filter(|(fork_id, _, _)| *fork_id == HardforkId::Curie)
            .map(|(_fork_id, chain_id, block_number)| (chain_id, block_number))
            .collect()
    });

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

    let chain_id = l2_trace.chain_id;
    let block_number = l2_trace.header.number.unwrap().as_u64();
    let curie_block = curie_block
        .or_else(|| HARDFORK_HEIGHTS.get(&chain_id).copied())
        .expect("Curie block number not provided and not found in hardfork heights");

    let spec_id = if block_number < curie_block {
        SpecId::BERNOULLI
    } else {
        SpecId::CURIE
    };

    let mut executor = EvmExecutor::new(&l2_trace, spec_id, disable_checks);
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
    true
}
