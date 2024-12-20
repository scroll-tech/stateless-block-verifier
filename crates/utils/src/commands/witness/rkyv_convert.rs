use clap::Args;
use rkyv::rancor;
use sbv::primitives::types::BlockWitness;
use std::path::PathBuf;

#[derive(Args)]
pub struct RkyvConvertCommand {
    /// Path to the witness json file
    witnesses: Vec<PathBuf>,
    /// Output directory
    #[arg(long)]
    out_dir: Option<PathBuf>,
}

impl RkyvConvertCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        if self.witnesses.is_empty() {
            anyhow::bail!("No witness files provided");
        }
        for path in self.witnesses.into_iter() {
            let witness = std::fs::File::open(&path)?;
            let witness: BlockWitness = serde_json::from_reader(witness)?;
            let serialized = rkyv::to_bytes::<rancor::Error>(&witness)?;
            let path = if let Some(ref out_dir) = self.out_dir {
                out_dir.join(path.file_name().unwrap())
            } else {
                path
            };
            let rkyv_path = path.with_extension("rkyv");
            std::fs::write(&rkyv_path, serialized)?;
            eprintln!("Converted {} to {}", path.display(), rkyv_path.display());
        }
        Ok(())
    }
}
