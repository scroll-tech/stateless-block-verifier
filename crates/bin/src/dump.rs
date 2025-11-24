use crate::helpers::{NumberOrRange, RpcArgs};
use alloy::providers::RootProvider;
use clap::Args;
use console::Emoji;
use eyre::{Context, ContextCompat};
use indicatif::{HumanBytes, HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use sbv::{primitives::types::Network, utils::rpc::ProviderExt};
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const INFO_ICON: Emoji = Emoji(" 🔗 ", " [+] ");
const ERR_ICON: Emoji = Emoji(" ❌ ", " [x] ");
const COMPLETED_ICON: Emoji = Emoji(" ✅ ", " [v] ");
const SAD_ICON: Emoji = Emoji(" ⚠️ ", " :( ");
const SPARKLE_ICON: Emoji = Emoji(" ✨ ", " :) ");

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

        let ok = dump_range(
            provider,
            self.block.into(),
            self.out_dir,
            #[cfg(not(feature = "scroll"))]
            self.ancestors,
        )
        .await;

        let elapsed = HumanDuration(started.elapsed());
        if ok {
            println!("{SPARKLE_ICON} Done in {elapsed}");
        } else {
            println!("{SAD_ICON} Completed with errors in {elapsed}",);
        }

        Ok(())
    }
}

static PB_STYLE: LazyLock<ProgressStyle> =
    LazyLock::new(|| ProgressStyle::with_template("{prefix}{msg} {spinner}").expect("infallible"));

async fn dump_range(
    provider: RootProvider<Network>,
    range: std::ops::Range<u64>,
    out_dir: PathBuf,
    #[cfg(not(feature = "scroll"))] ancestors: usize,
) -> bool {
    let mut set = tokio::task::JoinSet::new();

    let multi_progress_bar = MultiProgress::new();

    let mut ok = true;
    let mut pb_map = HashMap::new();

    for block in range {
        let provider = provider.clone();
        let out_dir = out_dir.clone();
        let progress_bar = multi_progress_bar.add(ProgressBar::new_spinner());
        let handle = {
            let progress_bar = progress_bar.clone();
            set.spawn(async move {
                dump(
                    provider,
                    block,
                    out_dir.as_path(),
                    #[cfg(not(feature = "scroll"))]
                    ancestors,
                    progress_bar,
                )
                .await
            })
        };
        pb_map.insert(handle.id(), progress_bar);
    }

    while let Some(result) = set.join_next_with_id().await {
        match result {
            Err(e) => {
                let pb = pb_map.remove(&e.id()).expect("progress bar exists");
                pb.set_prefix(format!("{ERR_ICON}"));
                pb.finish_with_message(format!("Dump task failed: {e}"));
                ok = false;
            }
            Ok((_, false)) => {
                ok = false;
            }
            _ => { /* ok */ }
        }
    }
    ok
}

async fn dump(
    provider: RootProvider<Network>,
    block: u64,
    out_dir: &Path,
    #[cfg(not(feature = "scroll"))] ancestors: usize,
    pb: ProgressBar,
) -> bool {
    pb.set_style(PB_STYLE.clone());
    pb.set_prefix(format!("{INFO_ICON}"));
    pb.set_message(format!("Dumping witness for block {block}"));
    pb.enable_steady_tick(Duration::from_millis(100));

    match dump_inner(
        provider,
        block,
        out_dir,
        #[cfg(not(feature = "scroll"))]
        ancestors,
    )
    .await
    {
        Ok((path, size)) => {
            pb.set_prefix(format!("{COMPLETED_ICON}"));
            pb.finish_with_message(format!("Witness: {size} saved to {p}", p = path.display()));
            true
        }
        Err(e) => {
            pb.set_prefix(format!("{ERR_ICON}"));
            pb.finish_with_message(format!("Failed to dump witness for block {block}: {e}"));
            false
        }
    }
}

async fn dump_inner(
    provider: RootProvider<Network>,
    block: u64,
    out_dir: &Path,
    #[cfg(not(feature = "scroll"))] ancestors: usize,
) -> eyre::Result<(PathBuf, HumanBytes)> {
    #[cfg(not(feature = "scroll"))]
    let witness = provider
        .dump_block_witness(block)
        .ancestors(ancestors)
        .send()
        .await?
        .context("block not found")?;
    #[cfg(feature = "scroll")]
    let witness = provider
        .dump_block_witness(block)
        .send()
        .await?
        .context("block not found")?;

    let json = serde_json::to_string_pretty(&witness)?;
    let path = out_dir.join(format!("{block}.json"));
    tokio::fs::write(&path, json).await?;
    let size = HumanBytes(tokio::fs::metadata(&path).await?.len());
    Ok((path, size))
}
