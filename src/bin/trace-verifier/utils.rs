use eth_types::l2_types::BlockTrace;
use eth_types::ToWord;
use stateless_block_verifier::EvmExecutor;

pub fn verify(l2_trace: BlockTrace) {
    let root_after = l2_trace.storage_trace.root_after.to_word();
    log::info!("Root after in trace: {:x}", root_after);

    let mut executor = EvmExecutor::new(&l2_trace);
    let revm_root_after = executor.handle_block(&l2_trace).to_word();
    log::info!("Root after in revm: {:x}", revm_root_after);

    if root_after != revm_root_after {
        log::error!("Root mismatch");
        std::process::exit(1);
    }
    log::info!("Root matches");
}
