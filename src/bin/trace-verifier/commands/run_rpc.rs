use crate::utils;
use clap::Args;
use eth_types::l2_types::BlockTrace;
use ethers_providers::{Http, Middleware, Provider};
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
    /// parallel worker count
    #[arg(short = 'j', long, default_value = "1")]
    parallel: usize,
}

#[derive(Debug, Copy, Clone)]
pub enum StartBlockSpec {
    Latest,
    Number(u64),
}

impl RunRpcCommand {
    pub async fn run(self, disable_checks: bool) -> anyhow::Result<()> {
        info!("Running RPC command with url: {}", self.url);
        let provider = Provider::new(Http::new(self.url));

        let start_block = match self.start_block {
            StartBlockSpec::Latest => provider.get_block_number().await?.as_u64(),
            StartBlockSpec::Number(n) => n,
        };

        let mut current_block = start_block;

        let (tx, rx) = async_channel::bounded(self.parallel);
        let handles = {
            let mut handles = Vec::with_capacity(self.parallel);
            for idx in 0..self.parallel {
                let _provider = provider.clone();
                let disable_checks = disable_checks;
                let rx = rx.clone();
                let handle = tokio::spawn(async move {
                    while let Ok(block_number) = rx.recv().await {
                        let l2_trace: BlockTrace = _provider
                            .request(
                                "scroll_getBlockTraceByNumberOrHash",
                                [format!("0x{:x}", block_number)],
                            )
                            .await?;

                        info!(
                            "worker#{idx}: load trace for block #{current_block}({:?})",
                            l2_trace.header.hash.unwrap()
                        );

                        tokio::task::spawn_blocking(move || {
                            utils::verify(l2_trace, disable_checks)
                        })
                        .await?;
                    }
                    Ok::<_, anyhow::Error>(())
                });
                handles.push(handle);
            }
            handles
        };

        loop {
            // exit when we reach the end block, or infinitely if no end block is specified
            if let Some(end_block) = self.end_block {
                if current_block > end_block {
                    break;
                }
            } else if current_block % 10 == 0 {
                let latest_block = provider.get_block_number().await?.as_u64();
                log::info!("distance to latest block: {}", latest_block - current_block);
            }

            tx.send(current_block).await?;
            current_block += 1;

            let mut exponential_backoff = 1;
            while provider.get_block_number().await?.as_u64() < current_block {
                if exponential_backoff == 1 {
                    info!("waiting for block #{}", current_block);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(exponential_backoff)).await;
                exponential_backoff *= 2;
            }
        }

        drop(tx);
        for handle in handles {
            handle.await??;
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
