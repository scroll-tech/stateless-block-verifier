use clap::Args;

#[derive(Args)]
pub struct DumpWitnessCommand {
    #[arg(long, help = "Chain id")]
    rpc: String,
}

impl DumpWitnessCommand {
    pub async fn run(self) -> anyhow::Result<()> {
        Ok(())
    }
}
