use alloy::{
    providers::{ProviderBuilder, RootProvider},
    rpc::client::ClientBuilder,
    transports::layers::{RetryBackoffLayer, ThrottleLayer},
};
use clap::Args;
use sbv::primitives::types::Network;
use std::future::Future;
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
