use crate::utils;
use alloy::providers::{Provider, ProviderBuilder};
use clap::Args;
use futures::future::OptionFuture;
use sbv::{
    core::HardforkConfig,
    primitives::{types::BlockTrace, Block},
};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
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
    pub async fn run(self, fork_config: impl Fn(u64) -> HardforkConfig) -> anyhow::Result<()> {
        dev_info!("Running RPC command with url: {}", self.url);
        let provider = ProviderBuilder::new().on_http(self.url);

        let chain_id = provider.get_chain_id().await?;
        let fork_config = fork_config(chain_id);

        let start_block = match self.start_block {
            StartBlockSpec::Latest => provider.get_block_number().await?,
            StartBlockSpec::Number(n) => n,
        };

        let mut current_block = start_block;

        let (tx, rx) = async_channel::bounded(self.parallel);

        let error_log = OptionFuture::from(self.log_error.as_ref().map(tokio::fs::File::create))
            .await
            .transpose()?
            .map(|f| Arc::new(Mutex::new(f)));

        let mut handles = JoinSet::new();
        for _idx in 0..self.parallel {
            let _provider = provider.clone();
            let rx = rx.clone();
            handles.spawn(async move {
                while let Ok(block_number) = rx.recv().await {
                    let l2_trace = _provider
                        .raw_request::<_, BlockTrace>(
                            "scroll_getBlockTraceByNumberOrHash".into(),
                            (
                                format!("0x{:x}", block_number),
                                serde_json::json!({
                                    "ExcludeExecutionResults": true,
                                    "ExcludeTxStorageTraces": true,
                                    "StorageProofFormat": "flatten",
                                    "FlattenProofsOnly": true
                                }),
                            ),
                        )
                        .await
                        .map_err(|e| (block_number, e.into()))?;

                    dev_info!(
                        "worker#{_idx}: load trace for block #{block_number}({})",
                        l2_trace.block_hash()
                    );

                    tokio::task::spawn_blocking(move || utils::verify(&l2_trace, &fork_config))
                        .await
                        .expect("failed to spawn blocking task")
                        .map_err(|e| (block_number, e.into()))?;
                }
                Ok::<_, (u64, anyhow::Error)>(())
            });
        }

        // handle errors
        let error_handler = tokio::spawn(async move {
            let error_log = error_log.clone();
            while let Some(result) = handles.join_next().await {
                match result {
                    Err(_e) => {
                        dev_error!("failed to join handle: {_e:?}");
                    }
                    Ok(Err((block_number, e))) => {
                        dev_error!("Error occurs when verifying block #{block_number}: {e:?}");

                        if let Some(error_log) = error_log.as_ref() {
                            let mut guard = error_log.lock().await;
                            guard
                                .write_all(format!("{block_number}, {e:?}\n").as_bytes())
                                .await
                                .ok();
                        } else {
                            std::process::exit(-1);
                        }
                    }
                    Ok(Ok(())) => {}
                }
            }
        });

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
                    dev_info!(
                        "distance to latest block: {}",
                        provider.get_block_number().await? - current_block
                    );
                }

                tx.send(current_block).await?;
                current_block += 1;

                update_metrics_gauge!(fetched_rpc_block_height, current_block as i64);

                let mut exponential_backoff = 1;
                loop {
                    let latest_block = provider.get_block_number().await?;

                    update_metrics_gauge!(latest_rpc_block_height, latest_block as i64);

                    if latest_block > current_block {
                        break;
                    }

                    if exponential_backoff == 1 {
                        dev_info!("waiting for block #{}", current_block);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(exponential_backoff)).await;
                    exponential_backoff *= 2;
                }
            }
        }

        tx.close();
        drop(tx);
        error_handler.await?;

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
