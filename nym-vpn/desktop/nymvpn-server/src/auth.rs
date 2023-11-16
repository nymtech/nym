use std::task::{Context, Poll};

use hyper::{http::HeaderValue, Body};
use tonic::{body::BoxBody, transport::Channel};
use tower::Service;

#[tonic::async_trait]
pub trait TokenProvider: Clone + Send + Sync {
    async fn bearer_token(&self) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct AuthLayer<P: TokenProvider> {
    token_provider: P,
    inner: Channel,
}

impl<P: TokenProvider> AuthLayer<P> {
    pub fn new(token_provider: P, inner: Channel) -> Self {
        AuthLayer {
            token_provider,
            inner,
        }
    }
}

impl<P> Service<hyper::Request<BoxBody>> for AuthLayer<P>
where
    P: TokenProvider + 'static,
{
    type Response = hyper::Response<Body>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: hyper::Request<BoxBody>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let token_provider = self.token_provider.clone();
        Box::pin(async move {
            if let Some(bearer_token) = token_provider.bearer_token().await {
                req.headers_mut().insert(
                    "authorization",
                    HeaderValue::from_str(&format!("Bearer {bearer_token}")).unwrap(),
                );
            }
            Ok(inner.call(req).await?)
        })
    }
}
