use alloy::providers::RootProvider;
use clap::Args;
use console::Emoji;
use eyre::Context;
use indicatif::{HumanBytes, HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use sbv::{primitives::types::Network, utils::rpc::ProviderExt};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::helpers::{NumberOrRange, RpcArgs};

#[derive(Debug, Args)]
pub struct DumpWitnessCommand {
    #[arg(
        long,
        help = "Block number or block range [start..end]",
        value_parser = clap::value_parser!(NumberOrRange),
    )]
    pub block: NumberOrRange,
    #[arg(long, help = "Ancestor blocks", default_value_t = 256)]
    #[cfg(not(feature = "scroll"))]
    pub ancestors: usize,
    #[arg(long, help = "Output directory", default_value_os_t = std::env::current_dir().unwrap())]
    pub out_dir: PathBuf,
    #[command(flatten)]
    pub rpc_args: RpcArgs,
}

impl DumpWitnessCommand {
    pub async fn run(self) -> eyre::Result<()> {
        let started = Instant::now();

        if self.out_dir.is_file() {
            eyre::bail!("Output path is a file");
        }
        std::fs::create_dir_all(&self.out_dir).context("create output directory")?;

        #[cfg(not(feature = "scroll"))]
        if self.ancestors < 1 || self.ancestors > 256 {
            eyre::bail!("Invalid ancestor blocks count");
        }

        let provider = self.rpc_args.into_provider();

        dump_range(
            provider,
            self.block.into(),
            self.out_dir,
            #[cfg(not(feature = "scroll"))]
            self.ancestors,
        )
        .await?;

        println!(
            "{} Done in {}",
            Emoji("✨ ", ":-)"),
            HumanDuration(started.elapsed())
        );

        Ok(())
    }
}

async fn dump_range(
    provider: RootProvider<Network>,
    range: std::ops::Range<u64>,
    out_dir: PathBuf,
    #[cfg(not(feature = "scroll"))] ancestors: usize,
) -> eyre::Result<()> {
    let mut set = tokio::task::JoinSet::new();

    let multi_progress_bar = MultiProgress::new();

    for block in range {
        let provider = provider.clone();
        let out_dir = out_dir.clone();
        let progress_bar = multi_progress_bar.add(ProgressBar::new_spinner());
        set.spawn(async move {
            if let Err(e) = dump(
                provider,
                block,
                out_dir.as_path(),
                #[cfg(not(feature = "scroll"))]
                ancestors,
                progress_bar,
            )
            .await
            {
                eprintln!("Error dumping witness for block {block}: {e}");
            }
        });
    }

    while let Some(result) = set.join_next().await {
        if let Err(e) = result {
            eprintln!("Dump task panicked: {e}");
        }
    }

    Ok(())
}

async fn dump(
    provider: RootProvider<Network>,
    block: u64,
    out_dir: &std::path::Path,
    #[cfg(not(feature = "scroll"))] ancestors: usize,
    pb: ProgressBar,
) -> eyre::Result<()> {
    pb.set_style(ProgressStyle::with_template("{prefix}{msg} {spinner}")?);
    pb.set_prefix(format!("{}", Emoji("🔗  ", "")));
    pb.set_message(format!("Dumping witness for block {block}"));
    pb.enable_steady_tick(Duration::from_millis(100));

    #[cfg(not(feature = "scroll"))]
    let witness = provider
        .dump_block_witness(block)
        .ancestors(ancestors)
        .send()
        .await
        .context("dump ethereum block witness")?;
    #[cfg(feature = "scroll")]
    let witness = provider
        .dump_block_witness(block)
        .send()
        .await
        .context("dump scroll block witness")?;

    let json = serde_json::to_string_pretty(&witness).context("serialize witness")?;
    let path = out_dir.join(format!("{block}.json"));
    tokio::fs::write(&path, json)
        .await
        .context("write json file")?;
    let size = HumanBytes(tokio::fs::metadata(&path).await?.len());

    pb.finish_with_message(format!(
        "JSON witness: {size} saved to {p}",
        p = path.display(),
    ));

    Ok(())
}
