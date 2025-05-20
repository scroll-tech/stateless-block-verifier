use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{TransportError, TransportFut, layers};
use std::time::Duration;
use tower::{Layer, Service};

/// A retry policy that always retries on errors.
#[derive(Debug, Copy, Clone)]
pub struct AlwaysRetryPolicy;

impl layers::RetryPolicy for AlwaysRetryPolicy {
    fn should_retry(&self, _error: &TransportError) -> bool {
        dev_trace!("going to retry on err: {_error}");
        true
    }

    fn backoff_hint(&self, _error: &TransportError) -> Option<Duration> {
        None
    }
}

/// Enforces a limit on the concurrent number of requests the underlying
/// service can handle.
///
/// Defaults to 5 concurrent requests.
#[derive(Debug, Clone)]
pub struct ConcurrencyLimitLayer {
    max: usize,
}

impl ConcurrencyLimitLayer {
    /// Create a new concurrency limit layer.
    pub const fn new(max: usize) -> Self {
        Self { max }
    }
}

impl Default for ConcurrencyLimitLayer {
    fn default() -> Self {
        Self::new(5)
    }
}

impl<S> Layer<S> for ConcurrencyLimitLayer {
    type Service = ConcurrencyLimit<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ConcurrencyLimit::new(inner, self.max)
    }
}

/// Enforces a limit on the concurrent number of requests the underlying
/// service can handle.
#[derive(Debug, Clone)]
pub struct ConcurrencyLimit<S> {
    inner: tower::limit::ConcurrencyLimit<S>,
}

impl<S> ConcurrencyLimit<S> {
    fn new(inner: S, max: usize) -> Self {
        ConcurrencyLimit {
            inner: tower::limit::ConcurrencyLimit::new(inner, max),
        }
    }
}

impl<S> Service<RequestPacket> for ConcurrencyLimit<S>
where
    S: Service<RequestPacket, Future = TransportFut<'static>, Error = TransportError>
        + Send
        + 'static
        + Clone,
{
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: RequestPacket) -> Self::Future {
        Box::pin(self.inner.call(request))
    }
}
