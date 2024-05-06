use eth_types::l2_types::BlockTrace;
use eth_types::ToWord;
use stateless_block_verifier::EvmExecutor;

pub fn verify(l2_trace: BlockTrace, disable_checks: bool) {
    let root_after = l2_trace.storage_trace.root_after.to_word();
    info!("Root after in trace: {:x}", root_after);

    let now = std::time::Instant::now();
    let mut executor = EvmExecutor::new(&l2_trace, disable_checks);
    let revm_root_after = executor.handle_block(&l2_trace).to_word();
    info!("Root after in revm: {:x}", revm_root_after);
    let elapsed = now.elapsed();

    if root_after != revm_root_after {
        error!("Root mismatch");
        std::process::exit(1);
    }
    info!("Root matches in: {} ms", elapsed.as_millis());
}
