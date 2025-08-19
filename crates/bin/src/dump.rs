use crate::helpers::RpcArgs;
use clap::Args;
use console::{Emoji, style};
use eyre::Context;
use indicatif::{HumanBytes, HumanDuration, ProgressBar, ProgressStyle};
use sbv::utils::rpc::ProviderExt;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Debug, Args)]
pub struct DumpWitnessCommand {
    #[arg(long, help = "Block number")]
    pub block: u64,
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

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::with_template("{prefix}{msg} {spinner}")?);
        pb.set_prefix(format!(
            "{} {}",
            style("[1/2]").bold().dim(),
            Emoji("🔗  ", "")
        ));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(format!("Dumping witness for block {}", self.block));

        #[cfg(not(feature = "scroll"))]
        let witness = provider
            .dump_block_witness(self.block)
            .ancestors(self.ancestors)
            .send()
            .await
            .context("dump ethereum block witness")?;
        #[cfg(feature = "scroll")]
        let witness = provider
            .dump_block_witness(self.block)
            .send()
            .await
            .context("dump scroll block witness")?;

        pb.finish_with_message(format!("Dumped witness for block {}", self.block));
        println!();

        let json = serde_json::to_string_pretty(&witness).context("serialize witness")?;
        let path = self.out_dir.join(format!("{}.json", self.block));
        std::fs::write(&path, json).context("write json file")?;
        let size = HumanBytes(std::fs::metadata(&path)?.len());
        println!(
            "{} {}JSON witness({}) saved to {}",
            style("[2/2]").bold().dim(),
            Emoji("📃  ", ""),
            size,
            path.display()
        );

        println!(
            "{} Done in {}",
            Emoji("✨ ", ":-)"),
            HumanDuration(started.elapsed())
        );
        Ok(())
    }
}
