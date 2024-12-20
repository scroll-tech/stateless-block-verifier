use crate::helpers::tower::ConcurrencyLimitLayer;
use alloy::network::primitives::BlockTransactionsKind;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::client::ClientBuilder;
use alloy::transports::layers::RetryBackoffLayer;
use clap::Args;
use console::{style, Emoji};
use indicatif::{HumanBytes, HumanDuration, ProgressBar};
use rkyv::rancor;
use sbv::primitives::types::{BlockHeader, BlockWitness, ExecutionWitness, Transaction};
use std::path::PathBuf;
use std::time::Instant;
use tokio::task::JoinSet;
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
        let total_steps = 4 + self.json as usize + self.rkyv as usize;

        let retry_layer = RetryBackoffLayer::new(self.max_retry, self.backoff, self.cups);
        let limit_layer = ConcurrencyLimitLayer::new(self.max_concurrency);
        let client = ClientBuilder::default()
            .layer(limit_layer)
            .layer(retry_layer)
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

        let execution_witness = provider
            .raw_request::<_, ExecutionWitness>(
                "debug_executionWitness".into(),
                (format!("0x{:x}", self.block),),
            )
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
        let pb = ProgressBar::new(self.ancestors);
        let mut joinset = JoinSet::new();
        for i in 1..=self.ancestors {
            let Some(block_number) = self.block.checked_sub(i) else {
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
        pb.finish_and_clear();
        steps += 1;

        let witness = BlockWitness {
            chain_id,
            header: BlockHeader::from(block.header),
            pre_state_root: ancestor_blocks[0].header.state_root,
            transaction: block
                .transactions
                .into_transactions()
                .map(Transaction::from_alloy)
                .collect(),
            block_hashes: ancestor_blocks.into_iter().map(|b| b.header.hash).collect(),
            withdrawals: block
                .withdrawals
                .map(|w| w.iter().map(From::from).collect()),
            states: execution_witness.state.into_values().collect(),
            codes: execution_witness.codes.into_values().collect(),
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
