use clap::Args;
use sbv::primitives::types::{BlockHeader, BlockWitness, ExecutionWitness, RpcBlock, Transaction};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct CreateWitnessCommand {
    #[arg(long, help = "Chain id")]
    chain_id: u64,
    #[arg(long, help = "Path to file rpc result of `eth_getBlockBy*`")]
    prev_block: PathBuf,
    #[arg(long, help = "Path to file rpc result of `eth_getBlockBy*`")]
    block: PathBuf,
    #[arg(long, help = "Path to file rpc result of `debug_executionWitness`")]
    witness: PathBuf,
    #[arg(long, help = "Path to output file")]
    out: Option<PathBuf>,
}

fn deserialize<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> anyhow::Result<T> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let value = serde_json::from_reader(reader)?;
    Ok(value)
}

impl CreateWitnessCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        let block: RpcBlock = deserialize(&self.block)?;
        eprintln!("Creating witness for block {}", block.header.number);
        let prev_block: RpcBlock = deserialize(&self.prev_block)?;
        if prev_block.header.number + 1 != block.header.number {
            anyhow::bail!("Blocks are not consecutive");
        }
        eprintln!("Previous state root: {}", prev_block.header.state_root);
        let witness: ExecutionWitness = deserialize(&self.witness)?;

        let witness = BlockWitness {
            chain_id: self.chain_id,
            header: BlockHeader::from(block.header),
            pre_state_root: prev_block.header.state_root,
            transaction: block
                .transactions
                .into_transactions()
                .map(Transaction::from_alloy)
                .collect(),
            withdrawals: block
                .withdrawals
                .map(|w| w.iter().map(From::from).collect()),
            states: witness.state.into_values().collect(),
            codes: witness.codes.into_values().collect(),
        };

        let file =
            std::fs::File::create(self.out.unwrap_or_else(|| PathBuf::from("witness.json")))?;
        serde_json::to_writer_pretty(file, &witness)?;

        eprintln!("Witness created successfully");
        Ok(())
    }
}
