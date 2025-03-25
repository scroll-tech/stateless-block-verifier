use crate::helpers::tower::ConcurrencyLimitLayer;
use alloy::{
    providers::{ProviderBuilder, RootProvider},
    rpc::client::ClientBuilder,
    transports::layers::RetryBackoffLayer,
};
use clap::Args;
use sbv::primitives::types::Network;
use std::future::Future;
use url::Url;

/// Helper functions for the tower
pub mod tower;
pub mod verifier;

#[derive(Debug, Args)]
pub struct RpcArgs {
    #[arg(
        long,
        help = "URL to the RPC server",
        default_value = "http://localhost:8545"
    )]
    pub rpc: Url,

    // Concurrency Limit
    #[arg(
        long,
        help = "Concurrency Limit: maximum number of concurrent requests",
        default_value = "10"
    )]
    pub max_concurrency: usize,

    // Retry parameters
    #[arg(
        long,
        help = "Retry Backoff: maximum number of retries",
        default_value = "10"
    )]
    pub max_retry: u32,
    #[arg(
        long,
        help = "Retry Backoff: backoff duration in milliseconds",
        default_value = "100"
    )]
    pub backoff: u64,
    #[arg(
        long,
        help = "Retry Backoff: compute units per second",
        default_value = "100"
    )]
    pub cups: u64,
}

impl RpcArgs {
    /// Construct a provider from the rpc arguments
    pub fn into_provider(self) -> RootProvider<Network> {
        let retry_layer = RetryBackoffLayer::new(self.max_retry, self.backoff, self.cups);
        let limit_layer = ConcurrencyLimitLayer::new(self.max_concurrency);
        let client = ClientBuilder::default()
            .layer(retry_layer)
            .layer(limit_layer)
            .http(self.rpc);

        ProviderBuilder::<_, _, Network>::default().on_client(client)
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
