use crate::utils;
use clap::Args;
use eth_types::l2_types::BlockTrace;
use ethers_providers::{Http, Middleware, Provider};
use futures::future::OptionFuture;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
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
    /// Do not exit on verification failure, log the error and continue
    #[arg(short, long)]
    log_error: Option<PathBuf>,
    /// Path to a file containing a list of blocks separated by newlines to verify
    #[arg(
        short,
        long,
        conflicts_with = "start_block",
        conflicts_with = "end_block"
    )]
    block_list: Option<PathBuf>,
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

        let error_log = OptionFuture::from(
            self.log_error
                .as_ref()
                .map(|path| tokio::fs::File::create(path)),
        )
        .await
        .transpose()?
        .map(|f| Arc::new(Mutex::new(f)));

        let handles = {
            let mut handles = Vec::with_capacity(self.parallel);
            for idx in 0..self.parallel {
                let _provider = provider.clone();
                let rx = rx.clone();
                let is_log_error = error_log.is_some();
                let error_log = error_log.clone();
                let handle = tokio::spawn(async move {
                    while let Ok(block_number) = rx.recv().await {
                        let l2_trace: BlockTrace = _provider
                            .request(
                                "scroll_getBlockTraceByNumberOrHash",
                                [format!("0x{:x}", block_number)],
                            )
                            .await?;

                        info!(
                            "worker#{idx}: load trace for block #{block_number}({:?})",
                            l2_trace.header.hash.unwrap()
                        );

                        let success = tokio::task::spawn_blocking(move || {
                            utils::verify(l2_trace, disable_checks, is_log_error)
                        })
                        .await?;

                        if !success {
                            let mut guard = error_log.as_ref().unwrap().lock().await;
                            guard
                                .write_all(format!("{block_number}\n").as_bytes())
                                .await?;
                        }
                    }
                    Ok::<_, anyhow::Error>(())
                });
                handles.push(handle);
            }
            handles
        };

        if let Some(block_list) = self.block_list {
            let block_list = tokio::fs::read_to_string(block_list).await?;
            for line in block_list.lines() {
                let block_number = line.trim().parse()?;
                tx.send(block_number).await?;
            }
        } else {
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
        }

        tx.close();
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
