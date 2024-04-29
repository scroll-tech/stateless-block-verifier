use clap::Parser;
use eth_types::l2_types::BlockTrace;
use eth_types::ToWord;
use stateless_block_verifier::EvmExecutor;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Stateless Block Verifier")]
struct Cli {
    trace: PathBuf,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    let cli = Cli::parse();

    log::info!("Reading trace from {:?}", cli.trace);
    let trace = std::fs::read_to_string(cli.trace)?;
    let l2_trace: BlockTrace = serde_json::from_str(&trace).unwrap_or_else(|_| {
        #[derive(serde::Deserialize, Default, Debug, Clone)]
        pub struct BlockTraceJsonRpcResult {
            pub result: BlockTrace,
        }
        serde_json::from_str::<BlockTraceJsonRpcResult>(&trace)
            .unwrap()
            .result
    });

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

    Ok(())
}
