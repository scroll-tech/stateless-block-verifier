use alloy::transports::{TransportError, layers};
use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub struct RateLimitRetryPolicy;

impl layers::RetryPolicy for RateLimitRetryPolicy {
    fn should_retry(&self, _error: &TransportError) -> bool {
        true
    }

    fn backoff_hint(&self, _error: &TransportError) -> Option<Duration> {
        None
    }
}
