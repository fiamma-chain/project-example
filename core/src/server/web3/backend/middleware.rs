use std::{
    num::NonZeroU32,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use jsonrpsee::{
    server::middleware::rpc::{layer::ResponseFuture, RpcServiceT},
    types::{error::ErrorCode, ErrorObject, Request},
    MethodResponse,
};

use super::metadata::MethodCall;

use pin_project_lite::pin_project;

/// A rate-limiting middleware.
///
/// `jsonrpsee` will allocate the instance of this struct once per session.
pub(crate) struct LimitMiddleware<S> {
    inner: S,
    rate_limiter: Option<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl<S> LimitMiddleware<S> {
    pub(crate) fn new(inner: S, requests_per_minute_limit: Option<NonZeroU32>) -> Self {
        Self {
            inner,
            rate_limiter: requests_per_minute_limit
                .map(|limit| RateLimiter::direct(Quota::per_minute(limit))),
        }
    }
}

impl<'a, S> RpcServiceT<'a> for LimitMiddleware<S>
where
    S: Send + Clone + Sync + RpcServiceT<'a>,
{
    type Future = ResponseFuture<S::Future>;

    fn call(&self, request: Request<'a>) -> Self::Future {
        if let Some(rate_limiter) = &self.rate_limiter {
            let num_requests = NonZeroU32::MIN; // 1 request, no batches possible

            // Note: if required, we can extract data on rate limiting from the error.
            if rate_limiter.check_n(num_requests).is_err() {
                // METRICS.rate_limited[&self.transport].inc();

                let rp = MethodResponse::error(
                    request.id,
                    ErrorObject::borrowed(
                        ErrorCode::ServerError(
                            reqwest::StatusCode::TOO_MANY_REQUESTS.as_u16().into(),
                        )
                        .code(),
                        "Too many requests",
                        None,
                    ),
                );
                return ResponseFuture::ready(rp);
            }
        }
        ResponseFuture::future(self.inner.call(request))
    }
}

pin_project! {
    #[derive(Debug)]
    pub(crate) struct WithMethodCall<F> {
        call: MethodCall,
        #[pin]
        inner: F,
    }
}

impl<F: Future<Output = MethodResponse>> Future for WithMethodCall<F> {
    type Output = MethodResponse;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let projection = self.project();
        let guard = projection.call.set_as_current();
        match projection.inner.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(response) => {
                drop(guard);
                projection.call.observe_response(&response);
                Poll::Ready(response)
            }
        }
    }
}
