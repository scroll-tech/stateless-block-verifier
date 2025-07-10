use crate::helpers::{retry::RateLimitRetryPolicy, tower::ConcurrencyLimitLayer};
use alloy::{
    providers::{ProviderBuilder, RootProvider},
    rpc::client::ClientBuilder,
    transports::layers::RetryBackoffLayer,
};
use clap::Args;
use sbv::primitives::types::Network;
use std::future::Future;
use url::Url;

mod dump;
pub mod retry;
/// Helper functions for the tower
pub mod tower;
pub mod verifier;

#[cfg(feature = "scroll")]
const MAINNET_RPC: &str = "https://euclid-l2-mpt.scroll.systems";
#[cfg(feature = "scroll")]
const SEPOLIA_RPC: &str = "https://sepolia-rpc.scroll.io";
const LOCAL_RPC: &str = "http://localhost:8545";

#[derive(Debug, Args)]
pub struct RpcArgs {
    #[arg(long, help = "URL to the RPC server, defaults to localhost:8545")]
    pub rpc: Option<Url>,

    #[cfg_attr(
        feature = "scroll",
        arg(
            long,
            help = "using mainnet default rpc url: https://euclid-l2-mpt.scroll.systems"
        )
    )]
    pub mainnet: bool,
    #[cfg_attr(
        feature = "scroll",
        arg(
            long,
            help = "using sepolia default rpc url: https://sepolia-rpc.scroll.io"
        )
    )]
    pub sepolia: bool,

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
        #[cfg(feature = "scroll")]
        let rpc = self.rpc.unwrap_or_else(|| {
            if self.mainnet {
                MAINNET_RPC.parse().unwrap()
            } else if self.sepolia {
                SEPOLIA_RPC.parse().unwrap()
            } else {
                LOCAL_RPC.parse().unwrap()
            }
        });
        #[cfg(not(feature = "scroll"))]
        let rpc = self.rpc.unwrap_or_else(|| LOCAL_RPC.parse().unwrap());
        dev_info!("Using RPC: {}", rpc);
        let retry_layer = RetryBackoffLayer::new_with_policy(
            self.max_retry,
            self.backoff,
            self.cups,
            RateLimitRetryPolicy,
        );
        let limit_layer = ConcurrencyLimitLayer::new(self.max_concurrency);
        let client = ClientBuilder::default()
            .layer(limit_layer)
            .layer(retry_layer)
            .http(rpc);

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
