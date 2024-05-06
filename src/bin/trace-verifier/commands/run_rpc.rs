use crate::utils;
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

        let mut current_block = start_block;
        loop {
            // exit when we reach the end block, or infinitely if no end block is specified
            if let Some(end_block) = self.end_block {
                if current_block > end_block {
                    break;
                }
            }

            let l2_trace: BlockTrace = provider
                .request(
                    "scroll_getBlockTraceByNumberOrHash",
                    [format!("0x{:x}", current_block)],
                )
                .await?;

            log::info!(
                "load trace for block #{current_block}({:?})",
                l2_trace.header.hash.unwrap()
            );

            tokio::task::spawn_blocking(move || utils::verify(l2_trace)).await?;

            current_block += 1;

            let mut exponential_backoff = 1;
            while provider.get_block_number().await?.as_u64() < current_block {
                if exponential_backoff == 1 {
                    log::info!("waiting for block #{}", current_block);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(exponential_backoff)).await;
                exponential_backoff *= 2;
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
