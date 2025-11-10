use alloy::{
    providers::{ProviderBuilder, RootProvider},
    rpc::client::ClientBuilder,
    transports::layers::{RetryBackoffLayer, ThrottleLayer},
};
use clap::Args;
use sbv::primitives::types::Network;
use std::{future::Future, num::ParseIntError, str::FromStr};
use url::Url;

pub mod verifier;

#[derive(Debug, Args)]
pub struct RpcArgs {
    #[arg(long, help = "URL to the RPC server, defaults to localhost:8545")]
    pub rpc: Url,

    // Retry parameters
    #[arg(long, help = "Maximum number of retries", default_value = "10")]
    pub max_rate_limit_retries: u32,
    #[arg(long, help = "Initial backoff in milliseconds", default_value = "100")]
    pub initial_backoff: u64,
    #[arg(long, help = "Compute units per second", default_value = "100")]
    pub compute_units_per_second: u64,

    // Throttling parameters
    #[arg(long, help = "Requests per second to throttle", default_value = "5")]
    pub requests_per_second: u32,
}

impl RpcArgs {
    /// Construct a provider from the rpc arguments
    pub fn into_provider(self) -> RootProvider<Network> {
        dev_info!("Using RPC: {}", self.rpc);

        let client = ClientBuilder::default()
            .layer(RetryBackoffLayer::new(
                self.max_rate_limit_retries,
                self.initial_backoff,
                self.compute_units_per_second,
            ))
            .layer(ThrottleLayer::new(self.requests_per_second))
            .http(self.rpc);
        ProviderBuilder::<_, _, Network>::default().connect_client(client)
    }
}

/// defer run in async
pub fn run_async<F: Future>(future: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(future)
}

/// Helper type to support variants of either a single block number or a range of blocks.
///
/// Valid syntax for [`NumberOrRange`] is either:
/// - `1234`: for a single block number
/// - `1234..1243`: for a block range
#[derive(Debug, Clone)]
pub enum NumberOrRange {
    /// Block number.
    Number(u64),
    /// Range of blocks from start to end.
    Range(std::ops::Range<u64>),
}

impl From<NumberOrRange> for std::ops::Range<u64> {
    fn from(value: NumberOrRange) -> Self {
        match value {
            NumberOrRange::Range(range) => range,
            NumberOrRange::Number(block) => std::ops::Range {
                start: block,
                end: block + 1,
            },
        }
    }
}

/// Error variants encountered while parsing [`NumberOrRange`] from CLI args.
#[derive(Debug, Clone)]
pub enum NumberOrRangeParseError {
    /// Invalid syntax for a block number of range.
    InvalidSyntax,
    /// Error while parsing string to integer.
    ParseInt(ParseIntError),
    /// Invalid values for a block range.
    InvalidRange { start: u64, end: u64 },
}

impl From<ParseIntError> for NumberOrRangeParseError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}

impl std::fmt::Display for NumberOrRangeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSyntax => write!(
                f,
                "Invalid syntax for block number or range. Example for expected syntax: `1234` or `1234..1243`"
            ),
            Self::ParseInt(e) => write!(f, "Failed to parse integer: {e}"),
            Self::InvalidRange { start, end } => {
                write!(f, "Invalid block range: end={end} <= start={start}")
            }
        }
    }
}

impl std::error::Error for NumberOrRangeParseError {}

impl FromStr for NumberOrRange {
    type Err = NumberOrRangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once("..") {
            Some((start_str, end_str)) => {
                let start = start_str.parse()?;
                let end = end_str.parse()?;

                (end > start)
                    .then_some(Self::Range(std::ops::Range { start, end }))
                    .ok_or(NumberOrRangeParseError::InvalidRange { start, end })
            }
            None => s
                .parse()
                .map(Self::Number)
                .map_err(|_| NumberOrRangeParseError::InvalidSyntax),
        }
    }
}
