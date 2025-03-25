use crate::helpers::{RpcArgs, verifier::verify_catch_panics};
use clap::Args;
use pumps::{Concurrency, Pipeline};
use sbv::{primitives::BlockWitness, utils::rpc::ProviderExt};
use std::{
    iter,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize},
    },
    time::Instant,
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

        let last_time = Mutex::new(Instant::now());
        let processed_blocks = Arc::new(AtomicUsize::new(0));

        let blocks = {
            let running = running.clone();
            iter::successors(Some(self.start_block), move |n| {
                if running.load(std::sync::atomic::Ordering::SeqCst) {
                    Some(n + 1)
                } else {
                    dev_warn!(
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
                                Err(_e) => {
                                    dev_error!(
                                        "failed to dump block witness for #{block_number}: {_e:?}"
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
            .map(
                |witness| async move {
                    let _number = witness.number();

                    match tokio::task::spawn_blocking(move || verify_catch_panics(witness))
                        .await
                        .map_err(anyhow::Error::from)
                        .and_then(|e| e)
                    {
                        Ok(_) => dev_info!("block#{_number} verified"),
                        Err(_e) => dev_info!("failed to verify block#{_number}: {_e:?}"),
                    }
                },
                Concurrency::concurrent_unordered(num_cpus::get()),
            )
            .filter_map(
                move |_| {
                    processed_blocks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if processed_blocks
                        .compare_exchange(
                            100,
                            0,
                            std::sync::atomic::Ordering::SeqCst,
                            std::sync::atomic::Ordering::SeqCst,
                        )
                        .is_ok()
                    {
                        let now = Instant::now();
                        let _elapsed = {
                            let mut last = last_time.lock().unwrap();
                            let elapsed = now.duration_since(*last);
                            *last = now;
                            elapsed.as_secs_f64()
                        };
                        dev_info!("bps: {:.2}", 100.0 / _elapsed);
                    }
                    async { None::<()> }
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
                dev_warn!("received ctrl-c again, force stop");
            }
        }
        Ok(())
    }
}
