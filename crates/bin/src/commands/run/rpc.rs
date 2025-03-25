use crate::helpers::{RpcArgs, verifier::verify_catch_panics};
use clap::Args;
use pumps::{Concurrency, Pipeline};
use sbv::{primitives::BlockWitness, utils::rpc::ProviderExt};
use std::{
    iter,
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Args, Debug)]
pub struct RunRpcCommand {
    #[arg(long, help = "start block number")]
    pub start_block: u64,
    #[command(flatten)]
    pub rpc_args: RpcArgs,
}

impl RunRpcCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        let max_concurrency = self.rpc_args.max_concurrency;
        let provider = self.rpc_args.into_provider();
        let running = Arc::new(AtomicBool::new(true));

        let blocks = {
            let running = running.clone();
            iter::successors(Some(self.start_block), move |n| {
                if running.load(std::sync::atomic::Ordering::SeqCst) {
                    Some(n + 1)
                } else {
                    dev_info!(
                        "received stop signal, stop emitting new blocks, current block: #{n}"
                    );
                    None
                }
            })
        };

        let (_out, h) = Pipeline::from_iter(blocks)
            .filter_map(
                move |block_number| {
                    let provider = provider.clone();
                    async move {
                        loop {
                            match provider.dump_block_witness(block_number.into()).await {
                                Ok(Some(w)) => {
                                    dev_info!("dumped block witness for #{block_number}");
                                    return Some(w);
                                }
                                Ok(None) => {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500))
                                        .await;
                                }
                                Err(e) => {
                                    dev_error!(
                                        "failed to dump block witness for #{block_number}: {e:?}"
                                    );
                                    return None;
                                }
                            }
                        }
                    }
                },
                Concurrency::concurrent_unordered(max_concurrency),
            )
            .backpressure(max_concurrency)
            .filter_map(
                |witness| async move {
                    let number = witness.number();
                    if let Err(e) =
                        tokio::task::spawn_blocking(move || verify_catch_panics(witness)).await
                    {
                        dev_error!("cannot join verification task #{number}: {e:?}");
                    }
                    None::<()>
                },
                Concurrency::concurrent_unordered(usize::MAX),
            )
            .build();

        tokio::signal::ctrl_c().await?;
        running.store(false, std::sync::atomic::Ordering::SeqCst);
        tokio::select! {
            _ = h => {
                dev_info!("pipeline finished");
            }
            _ = tokio::signal::ctrl_c() => {
                dev_info!("received ctrl-c again, force stop");
            }
        }
        Ok(())
    }
}
