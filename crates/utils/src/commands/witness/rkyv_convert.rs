use clap::Args;
use rkyv::{rancor, vec::ArchivedVec};
use sbv::primitives::types::{ArchivedBlockWitness, BlockWitness};
use std::path::PathBuf;

#[derive(Args)]
pub struct RkyvConvertCommand {
    /// Path to the witness json file
    witnesses: Vec<PathBuf>,
    /// Make chunk
    #[arg(long, help = "Make single chunk rkyv instead of multiple blocks")]
    chunk: bool,
    /// Output directory
    #[arg(long)]
    out_dir: Option<PathBuf>,
}

impl RkyvConvertCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        if self.witnesses.is_empty() {
            anyhow::bail!("No witness files provided");
        }
        let mut witnesses = Vec::new();
        for path in self.witnesses.iter() {
            let witness = std::fs::File::open(path)?;
            let witness: BlockWitness = serde_json::from_reader(witness)?;
            witnesses.push(witness);
        }

        if self.chunk {
            if !witnesses.windows(2).all(|w| w[0].chain_id == w[1].chain_id) {
                anyhow::bail!("All witnesses must have the same chain id in chunk mode");
            }
            if !witnesses
                .windows(2)
                .all(|w| w[0].header.number + 1 == w[1].header.number)
            {
                anyhow::bail!("All witnesses must have sequential block numbers in chunk mode");
            }

            let serialized = rkyv::to_bytes::<rancor::Error>(&witnesses)?;
            let _ =
                rkyv::access::<ArchivedVec<ArchivedBlockWitness>, rancor::Error>(&serialized[..])?;

            let start_block_number = witnesses[0].header.number;
            let chunk_size = witnesses.len();
            let filename = format!("chunk-{}-{}.rkyv", start_block_number, chunk_size);
            let path = if let Some(ref out_dir) = self.out_dir {
                out_dir
            } else {
                self.witnesses[0].parent().unwrap()
            };
            let rkyv_path = path.join(filename);
            std::fs::write(&rkyv_path, serialized)?;
            eprintln!(
                "Converted {} witnesses to chunk {}",
                chunk_size,
                rkyv_path.display()
            );
        } else {
            for (witness, path) in witnesses.into_iter().zip(self.witnesses.into_iter()) {
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
        }
        Ok(())
    }
}
