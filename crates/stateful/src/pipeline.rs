use crate::utils::patch_fix_block;
use crate::{retry_if_transport_error, Error, Result};
use alloy::providers::{Provider, ReqwestProvider};
use alloy::rpc::types::Block;
use sbv::primitives::{alloy_primitives::ChainId, types::AlloyTransaction, Address};
use std::collections::BinaryHeap;
use std::fmt::Debug;
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};
use tokio::sync::Mutex;

struct OrderedQueue {
    queue: BinaryHeap<FetchedBlock>,
    sender_index: u64,
}

/// Fetcher to fetch blocks from the provider.
#[derive(Clone, Debug)]
pub struct Fetcher {
    count: usize,
    provider: ReqwestProvider,
    coinbase: Address,
    chain_id: ChainId,

    queue: Arc<Mutex<OrderedQueue>>,
    fetcher_index: Arc<AtomicU64>,
    tx: tokio::sync::mpsc::Sender<Block<AlloyTransaction>>,
    shutdown: Arc<AtomicBool>,
}

#[derive(Eq, PartialEq)]
struct FetchedBlock(Block<AlloyTransaction>);

impl Fetcher {
    /// Spawn `count` fetchers to fetch blocks from the provider.
    pub fn spawn(
        count: usize,
        provider: ReqwestProvider,
        coinbase: Address,
        chain_id: ChainId,
        start_block: u64,
        shutdown: Arc<AtomicBool>,
    ) -> tokio::sync::mpsc::Receiver<Block<AlloyTransaction>> {
        let (tx, rx) = tokio::sync::mpsc::channel(count);

        let queue = OrderedQueue {
            queue: BinaryHeap::new(),
            sender_index: start_block,
        };

        let fetcher = Fetcher {
            count,
            provider,
            coinbase,
            chain_id,
            queue: Arc::new(Mutex::new(queue)),
            fetcher_index: Arc::new(AtomicU64::new(start_block)),
            tx,
            shutdown: shutdown.clone(),
        };

        for _ in 0..count {
            let fetcher = fetcher.clone();
            let shutdown = shutdown.clone();
            tokio::spawn(async move {
                fetcher.run().await.ok();
                shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
            });
        }

        rx
    }

    async fn run(self) -> Result<()> {
        while !self.shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            loop {
                let queue = self.queue.lock().await;
                // back pressure
                if queue.queue.len() >= self.count {
                    drop(queue);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
                break;
            }

            let block_number = self
                .fetcher_index
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            // wait for new block
            while let Ok(latest_chain_block_number) =
                retry_if_transport_error!(self.provider.get_block_number())
            {
                if block_number <= latest_chain_block_number {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            let mut block = retry_if_transport_error!(self
                .provider
                .raw_request::<_, Block<AlloyTransaction>>(
                    "eth_getBlockByNumber".into(),
                    (format!("0x{:x}", block_number), true),
                ))?;
            patch_fix_block(&mut block, self.coinbase, self.chain_id);

            dev_trace!(
                "block#{} block fetched, state root: {}",
                block.header.number,
                block.header.state_root
            );

            let mut queue = self.queue.lock().await;
            queue.queue.push(FetchedBlock(block));
            while queue
                .queue
                .peek()
                .map(|b| b.0.header.number == queue.sender_index)
                .unwrap_or_default()
            {
                let block = queue.queue.pop().unwrap().0;
                if self.tx.send(block).await.is_err() {
                    return Err(Error::PipelineShutdown);
                }
                queue.sender_index += 1;
            }
        }
        Ok(())
    }
}

impl Debug for OrderedQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderedQueue")
            .field("sender_index", &self.sender_index)
            .finish()
    }
}

impl Ord for FetchedBlock {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.header.number.cmp(&other.0.header.number).reverse()
    }
}

impl PartialOrd for FetchedBlock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
