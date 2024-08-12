use clap::Args;
use eth_types::l2_types::BlockTrace;
use serde::{Deserialize, Deserializer};
use stateless_block_verifier::HardforkConfig;
use std::path::PathBuf;

use crate::utils;

/// A wrapper around [`eth_types::l2_types::BlockTrace`] so that we can re-implement custom
/// deserializer for it.
struct WrappedBlockTrace(BlockTrace);

/// In the JSON format of [`eth_types::l2_types::BlockTrace`] appears a "chainID" key.
const JSON_KEY_CHAIN_ID: &str = "chainID";

impl<'de> Deserialize<'de> for WrappedBlockTrace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the input JSON to a generic `serde_json::Value`.
        let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

        // Try to handle the case where the JSON represents the struct itself. We know that the key
        // "chainID" is present in the block trace.
        if let Some(obj) = value.as_object() {
            if obj.contains_key(JSON_KEY_CHAIN_ID) {
                let block_trace: BlockTrace =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                return Ok(WrappedBlockTrace(block_trace));
            }
        }

        // Look for `BlockTrace` inside the JSON, if it is nested.
        if let Some(obj) = value.as_object() {
            for (_key, inner_value) in obj {
                if let Some(inner_obj) = inner_value.as_object() {
                    if inner_obj.contains_key(JSON_KEY_CHAIN_ID) {
                        let block_trace: BlockTrace = serde_json::from_value(inner_value.clone())
                            .map_err(serde::de::Error::custom)?;
                        return Ok(WrappedBlockTrace(block_trace));
                    }
                }
            }
        }

        // The fact that we are here means we could not find a "chainID" key anywhere in the JSON
        // file.
        Err(serde::de::Error::custom(
            "Invalid JSON format: could not find BlockTrace",
        ))
    }
}

#[derive(Args)]
pub struct RunFileCommand {
    /// Path to the trace file
    #[arg(short, long, default_value = "trace.json")]
    path: Vec<PathBuf>,
}

impl RunFileCommand {
    pub async fn run(
        self,
        fork_config: impl Fn(u64) -> HardforkConfig,
        disable_checks: bool,
    ) -> anyhow::Result<()> {
        for path in self.path {
            info!("Reading trace from {:?}", path);
            let trace_json = tokio::fs::read_to_string(&path).await?;
            let trace = {
                let wrapped_trace: WrappedBlockTrace = serde_json::from_str(&trace_json)?;
                wrapped_trace.0
            };
            let fork_config = fork_config(trace.chain_id);
            tokio::task::spawn_blocking(move || {
                utils::verify(trace, &fork_config, disable_checks, false)
            })
            .await?;
        }
        Ok(())
    }
}
