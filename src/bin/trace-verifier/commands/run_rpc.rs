use clap::Args;
use eth_types::l2_types::BlockTrace;
use ethers_core::types::BlockNumber;
use ethers_providers::{Http, Middleware, Provider};
use stateless_block_verifier::EvmExecutor;
use std::str::FromStr;
use url::Url;

#[derive(Args)]
pub struct RunRpcCommand {
    /// RPC URL
    #[arg(short, long, default_value = "http://localhost:8545")]
    url: Url,
    /// Start Block number
    #[arg(short, long, default_value = "latest")]
    start_block: StartBlockSpec,
    /// End block number
    #[arg(short, long)]
    end_block: Option<u64>,
}

#[derive(Debug, Copy, Clone)]
pub enum StartBlockSpec {
    Latest,
    Number(u64),
}

impl RunRpcCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        log::info!("Running RPC command with url: {}", self.url);
        let provider = Provider::new(Http::new(self.url));

        let start_block = match self.start_block {
            StartBlockSpec::Latest => provider.get_block_number().await?.as_u64(),
            StartBlockSpec::Number(n) => n,
        };

        let mut evm_executor: Option<EvmExecutor> = None;
        let mut current_block = start_block;
        loop {
            // exit when we reach the end block, or infinitely if no end block is specified
            if let Some(end_block) = self.end_block {
                if current_block > end_block {
                    break;
                }
            }

            let trace: BlockTrace = provider
                .request(
                    "scroll_getBlockTraceByNumberOrHash",
                    [format!("0x{:x}", current_block)],
                )
                .await?;

            log::info!(
                "load trace for block #{current_block}({}), root after: {:?}",
                trace.header.hash.unwrap(),
                trace.storage_trace.root_after,
            );

            if evm_executor.is_none() {
                log::info!("Initializing EVM executor for the first time");
                evm_executor = Some(EvmExecutor::new(&trace));
                log::info!("EVM executor initialized");
            }

            match evm_executor {
                Some(ref mut executor) => {
                    let revm_root_after = executor.handle_block(&trace);
                    log::info!("Root after calculated by revm: {:x}", revm_root_after);
                    if revm_root_after != trace.storage_trace.root_after {
                        log::error!("Root mismatch");
                        std::process::exit(1);
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}

impl FromStr for StartBlockSpec {
    type Err = <u64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "latest" => Ok(StartBlockSpec::Latest),
            s => Ok(StartBlockSpec::Number(s.parse()?)),
        }
    }
}
