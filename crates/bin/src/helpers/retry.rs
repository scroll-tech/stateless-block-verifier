use alloy::transports::{TransportError, layers};
use std::time::Duration;

#[derive(Debug)]
pub struct RateLimitRetryPolicy;

impl layers::RetryPolicy for RateLimitRetryPolicy {
    fn should_retry(&self, _error: &TransportError) -> bool {
        true
    }

    fn backoff_hint(&self, error: &TransportError) -> Option<Duration> {
        layers::RateLimitRetryPolicy.backoff_hint(error)
    }
}
