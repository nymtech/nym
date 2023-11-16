use std::task::{Context, Poll};

use hyper::Body;
use tonic::body::BoxBody;
use tower::{Layer, Service};

#[tonic::async_trait]
pub trait Auth: Clone + Send + Sync {
    async fn is_authenticated(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct ControllerAuthLayer<P: Auth> {
    auth: P,
}

impl<P: Auth> ControllerAuthLayer<P> {
    pub fn new(auth: P) -> Self {
        Self { auth }
    }
}

const ALLOWED_UNAUTHORIZED_PATHS: [&str; 2] = [
    "/nymvpn.controller.ControllerService/AccountSignIn",
    "/nymvpn.controller.ControllerService/IsAuthenticated",
];

#[derive(Debug, Clone)]
pub struct ControllerAuthMiddleware<S, P: Auth> {
    auth: P,
    inner: S,
}

impl<S, P: Auth> Layer<S> for ControllerAuthLayer<P> {
    type Service = ControllerAuthMiddleware<S, P>;
    fn layer(&self, inner: S) -> Self::Service {
        ControllerAuthMiddleware {
            auth: self.auth.clone(),
            inner,
        }
    }
}

impl<S, P> Service<hyper::Request<Body>> for ControllerAuthMiddleware<S, P>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    P: Auth + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: hyper::Request<Body>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let auth = self.auth.clone();
        Box::pin(async move {
            if ALLOWED_UNAUTHORIZED_PATHS.contains(&req.uri().path())
                || auth.is_authenticated().await
            {
                return inner.call(req).await;
            }

            Ok(tonic::Status::unauthenticated("please sign in first").to_http())
        })
    }
}
