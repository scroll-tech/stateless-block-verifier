use crate::helpers::tower::ConcurrencyLimitLayer;
use alloy::{
    network::primitives::BlockTransactionsKind,
    providers::{Provider, ProviderBuilder},
    rpc::client::ClientBuilder,
    transports::layers::RetryBackoffLayer,
};
use clap::Args;
use console::{Emoji, style};
use indicatif::{HumanBytes, HumanDuration};
use rkyv::rancor;
#[cfg(not(feature = "scroll"))]
use sbv::primitives::types::RpcBlock;
use sbv::primitives::{
    chainspec::{Chain, NamedChain},
    ext::ProviderExt,
    types::{BlockHeader, BlockWitness, Transaction},
};
use std::{path::PathBuf, time::Instant};
use url::Url;

#[derive(Args)]
pub struct DumpWitnessCommand {
    #[arg(
        long,
        help = "URL to the RPC server",
        default_value = "http://localhost:8545"
    )]
    rpc: Url,
    #[arg(long, help = "Block number")]
    block: u64,
    #[arg(long, help = "Ancestor blocks", default_value_t = 256)]
    ancestors: u64,
    #[arg(long, help = "Output directory", default_value_os_t = std::env::current_dir().unwrap())]
    out_dir: PathBuf,
    #[arg(long, help = "Output json")]
    json: bool,
    #[arg(long, help = "Output rkyv")]
    rkyv: bool,

    // Concurrency Limit
    #[arg(
        long,
        help = "Concurrency Limit: maximum number of concurrent requests",
        default_value = "10"
    )]
    max_concurrency: usize,

    // Retry parameters
    #[arg(
        long,
        help = "Retry Backoff: maximum number of retries",
        default_value = "10"
    )]
    max_retry: u32,
    #[arg(
        long,
        help = "Retry Backoff: backoff duration in milliseconds",
        default_value = "100"
    )]
    backoff: u64,
    #[arg(
        long,
        help = "Retry Backoff: compute units per second",
        default_value = "100"
    )]
    cups: u64,
}

impl DumpWitnessCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        let started = Instant::now();

        if self.out_dir.is_file() {
            anyhow::bail!("Output path is a file");
        }
        std::fs::create_dir_all(&self.out_dir)?;
        if !self.json && !self.rkyv {
            anyhow::bail!("No output format specified");
        }

        if self.ancestors < 1 || self.ancestors > 256 {
            anyhow::bail!("Invalid ancestor blocks count");
        }

        let mut steps = 1;
        let total_steps =
            4 + self.json as usize + self.rkyv as usize + cfg!(feature = "scroll") as usize;

        let retry_layer = RetryBackoffLayer::new(self.max_retry, self.backoff, self.cups);
        let limit_layer = ConcurrencyLimitLayer::new(self.max_concurrency);
        let client = ClientBuilder::default()
            .layer(retry_layer)
            .layer(limit_layer)
            .http(self.rpc);
        let provider = ProviderBuilder::new().on_client(client);

        let chain_id = provider.get_chain_id().await?;
        eprintln!(
            "{} {}Chain ID: {}",
            style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
            Emoji("🔗  ", ""),
            chain_id
        );
        steps += 1;

        if !cfg!(feature = "scroll") {
            let chain = Chain::from(chain_id);
            if chain == Chain::from_named(NamedChain::Scroll)
                || chain == Chain::from_named(NamedChain::ScrollSepolia)
            {
                eprintln!(
                    "      {}Scroll feature is not enabled, but the chain is Scroll or ScrollSepolia",
                    Emoji("⚠️  ", "")
                );
            }
        }

        let block = provider
            .get_block_by_number(self.block.into(), BlockTransactionsKind::Full)
            .await
            .expect("transport error")
            .expect("block not found");
        eprintln!(
            "{} {}Block#{} feched: {} txns, {} withdrawals, state root = {}",
            style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
            Emoji("📖  ", ""),
            block.header.number,
            block.transactions.len(),
            block
                .withdrawals
                .as_ref()
                .map(|w| w.len())
                .unwrap_or_default(),
            block.header.state_root
        );
        steps += 1;

        #[cfg(feature = "scroll")]
        let block = {
            let roots = provider
                .scroll_disk_root(self.block.into())
                .await
                .expect("transport error");
            eprintln!(
                "{} {}Patch block header state root to MPT root = {}",
                style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
                Emoji("🔧  ", ""),
                roots.disk_root
            );
            let mut block = block;
            assert_eq!(block.header.state_root, roots.header_root, "should same");
            block.header.state_root = roots.disk_root;
            steps += 1;
            block
        };

        let execution_witness = provider
            .debug_execution_witness(self.block.into())
            .await
            .expect("transport error");
        eprintln!(
            "{} {}Execution witness: {} states, {} codes",
            style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
            Emoji("👁️‍🗨️  ", ""),
            execution_witness.state.len(),
            execution_witness.codes.len()
        );
        steps += 1;

        #[cfg(not(feature = "scroll"))]
        let (ancestor_blocks, pre_state_root) = {
            eprintln!(
                "{} {}Fetching ancestor blocks...",
                style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
                Emoji("🚚  ", ""),
            );
            if self.ancestors != 256 {
                eprintln!(
                    "      {}As requested, not all 256 ancestor blocks will be fetched, incomplete bloch hashes may cause verification failure",
                    Emoji("⚠️  ", "")
                );
            }
            let ancestor_blocks =
                dump_ancestor_blocks(provider.clone(), self.block, self.ancestors).await;
            let pre_state_root = ancestor_blocks[0].header.state_root;
            (ancestor_blocks, pre_state_root)
        };
        #[cfg(feature = "scroll")]
        let pre_state_root = {
            eprintln!(
                "      {}Scroll feature enabled, ancestor blocks will not be fetched",
                Emoji("⚠️  ", "")
            );
            let pre_state_root = provider
                .scroll_disk_root((self.block - 1).into())
                .await
                .expect("transport error")
                .disk_root;
            pre_state_root
        };
        steps += 1;

        let mut states = execution_witness.state.into_values().collect::<Vec<_>>();
        states.sort();
        let mut codes = execution_witness.codes.into_values().collect::<Vec<_>>();
        codes.sort();
        let witness = BlockWitness {
            chain_id,
            header: BlockHeader::from(block.header),
            pre_state_root,
            transaction: block
                .transactions
                .into_transactions()
                .map(Transaction::from_alloy)
                .collect(),
            #[cfg(not(feature = "scroll"))]
            block_hashes: ancestor_blocks
                .into_iter()
                .map(|b: RpcBlock| b.header.hash)
                .collect(),
            withdrawals: block
                .withdrawals
                .map(|w| w.iter().map(From::from).collect()),
            states,
            codes,
        };

        if self.json {
            let json = serde_json::to_string_pretty(&witness)?;
            let path = self.out_dir.join(format!("{}.json", self.block));
            std::fs::write(&path, json)?;
            let size = HumanBytes(std::fs::metadata(&path)?.len());
            println!(
                "{} {}JSON witness({}) saved to {}",
                style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
                Emoji("📃  ", ""),
                size,
                path.display()
            );
            steps += 1;
        }

        if self.rkyv {
            let serialized = rkyv::to_bytes::<rancor::Error>(&witness)?;
            let path = self.out_dir.join(format!("{}.rkyv", self.block));
            std::fs::write(&path, serialized)?;
            let size = HumanBytes(std::fs::metadata(&path)?.len());
            println!(
                "{} {}rkyv witness({}) saved to {}",
                style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
                Emoji("🏛  ", ""),
                size,
                path.display()
            );
        }

        println!(
            "{} Done in {}",
            Emoji("✨ ", ":-)"),
            HumanDuration(started.elapsed())
        );
        Ok(())
    }
}

#[cfg(not(feature = "scroll"))]
async fn dump_ancestor_blocks<
    P: Provider<T> + Clone + 'static,
    T: alloy::transports::Transport + Clone,
>(
    provider: P,
    block: u64,
    ancestors: u64,
) -> Vec<RpcBlock> {
    let pb = indicatif::ProgressBar::new(ancestors);
    let mut joinset = tokio::task::JoinSet::new();
    for i in 1..=ancestors {
        let Some(block_number) = block.checked_sub(i) else {
            break;
        };
        let pb = pb.clone();
        let provider = provider.clone();
        joinset.spawn(async move {
            let block = provider
                .get_block_by_number(block_number.into(), BlockTransactionsKind::Hashes)
                .await;
            pb.inc(1);
            block
        });
    }
    let mut ancestor_blocks = joinset
        .join_all()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("transport error")
        .into_iter()
        .map(|b| b.expect("block not found"))
        .collect::<Vec<_>>();
    ancestor_blocks.sort_by_key(|b| std::cmp::Reverse(b.header.number)); // JoinSet is unordered
    pb.finish_and_clear();
    ancestor_blocks
}
