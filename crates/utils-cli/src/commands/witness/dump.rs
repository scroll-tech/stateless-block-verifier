use crate::helpers::tower::ConcurrencyLimitLayer;
use alloy::{
    providers::ProviderBuilder, rpc::client::ClientBuilder, transports::layers::RetryBackoffLayer,
};
use clap::Args;
use console::{Emoji, style};
use indicatif::{HumanBytes, HumanDuration, ProgressBar, ProgressStyle};
use rkyv::rancor;
use sbv_primitives::types::Network;
use sbv_utils::rpc::ProviderExt;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use url::Url;

#[derive(Debug, Args)]
pub struct DumpWitnessCommand {
    #[arg(
        long,
        help = "URL to the RPC server",
        default_value = "http://localhost:8545"
    )]
    pub rpc: Url,
    #[arg(long, help = "Block number")]
    pub block: u64,
    #[arg(long, help = "Ancestor blocks", default_value_t = 256)]
    #[cfg(not(feature = "scroll"))]
    pub ancestors: usize,
    #[arg(long, help = "Output directory", default_value_os_t = std::env::current_dir().unwrap())]
    pub out_dir: PathBuf,
    #[arg(long, help = "Output json")]
    pub json: bool,
    #[arg(long, help = "Output rkyv")]
    pub rkyv: bool,

    // Concurrency Limit
    #[arg(
        long,
        help = "Concurrency Limit: maximum number of concurrent requests",
        default_value = "10"
    )]
    pub max_concurrency: usize,

    // Retry parameters
    #[arg(
        long,
        help = "Retry Backoff: maximum number of retries",
        default_value = "10"
    )]
    pub max_retry: u32,
    #[arg(
        long,
        help = "Retry Backoff: backoff duration in milliseconds",
        default_value = "100"
    )]
    pub backoff: u64,
    #[arg(
        long,
        help = "Retry Backoff: compute units per second",
        default_value = "100"
    )]
    pub cups: u64,
}

impl DumpWitnessCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        let started = Instant::now();

        if self.out_dir.is_file() {
            anyhow::bail!("Output path is a file");
        }
        std::fs::create_dir_all(&self.out_dir)?;
        if !self.json && !self.rkyv {
            eprintln!("{}No output format specified", Emoji("⚠️  ", ""));
        }

        #[cfg(not(feature = "scroll"))]
        if self.ancestors < 1 || self.ancestors > 256 {
            anyhow::bail!("Invalid ancestor blocks count");
        }

        let mut steps = 1;
        let total_steps = 1 + self.json as usize + self.rkyv as usize;

        let retry_layer = RetryBackoffLayer::new(self.max_retry, self.backoff, self.cups);
        let limit_layer = ConcurrencyLimitLayer::new(self.max_concurrency);
        let client = ClientBuilder::default()
            .layer(retry_layer)
            .layer(limit_layer)
            .http(self.rpc);

        let provider = ProviderBuilder::<_, _, Network>::default().on_client(client);

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::with_template("{prefix}{msg} {spinner}").unwrap());
        pb.set_prefix(format!(
            "{} {}",
            style(format!("[{}/{}]", steps, total_steps)).bold().dim(),
            Emoji("🔗  ", "")
        ));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(format!("Dumping witness for block {}", self.block));
        steps += 1;

        #[cfg(not(feature = "scroll"))]
        let witness = provider
            .dump_block_witness(self.block.into(), Some(self.ancestors))
            .await?;
        #[cfg(feature = "scroll")]
        let witness = provider.dump_block_witness(self.block.into()).await?;

        pb.finish_with_message(format!("Dumped witness for block {}", self.block));
        println!();

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
