use alloy::{
    rpc::json_rpc::{RequestPacket, ResponsePacket},
    transports::{TransportError, TransportFut},
};
use tower::{Layer, Service};

/// Enforces a limit on the concurrent number of requests the underlying
/// service can handle.
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
    pub fn new(inner: S, max: usize) -> Self {
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
