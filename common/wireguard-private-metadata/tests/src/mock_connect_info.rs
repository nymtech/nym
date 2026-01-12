// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::Request;
use axum::http::request::Parts;
use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};
use tower::Layer;
use tower::Service;

#[derive(Clone)]
pub struct DummyConnectInfo {
    // store it as atomic i32 to avoid having to use locks to read and set the value
    address: Arc<AtomicU32>,
}

impl Display for DummyConnectInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.address().fmt(f)
    }
}

impl DummyConnectInfo {
    pub fn new() -> Self {
        let dummy_ip = Ipv4Addr::new(1, 2, 3, 4);
        DummyConnectInfo {
            address: Arc::new(AtomicU32::new(dummy_ip.to_bits())),
        }
    }

    #[allow(clippy::panic)]
    pub fn set(&self, address: IpAddr) {
        let IpAddr::V4(v4_address) = address else {
            // it would be relatively easy to support ipv6 with multiple atomics,
            // but I didn't feel it was needed at the time
            panic!("ipv6 not supported")
        };

        self.address.store(v4_address.to_bits(), Ordering::Relaxed);
    }

    pub fn address(&self) -> SocketAddr {
        let bits = self.address.load(Ordering::Relaxed);
        let ipv4 = Ipv4Addr::from(bits);

        SocketAddr::new(IpAddr::V4(ipv4), 1791)
    }

    pub fn ip(&self) -> IpAddr {
        self.address().ip()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for DummyConnectInfo
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    #[allow(clippy::panic)]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(info) = parts.extensions.get::<DummyConnectInfo>() {
            Ok(info.clone())
        } else {
            // this is a test code so that's fine
            panic!("DummyConnectInfo not set")
        }
    }
}

#[derive(Clone)]
pub struct MockConnectInfoLayer {
    info: DummyConnectInfo,
}

impl MockConnectInfoLayer {
    pub fn new(info: DummyConnectInfo) -> Self {
        Self { info }
    }
}

impl<S> Layer<S> for MockConnectInfoLayer {
    type Service = MockConnectInfoMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MockConnectInfoMiddleware {
            inner,
            info: self.info.clone(),
        }
    }
}

#[derive(Clone)]
pub struct MockConnectInfoMiddleware<S> {
    inner: S,
    info: DummyConnectInfo,
}

impl<S, ReqBody> Service<Request<ReqBody>> for MockConnectInfoMiddleware<S>
where
    S: Service<Request<ReqBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        req.extensions_mut().insert(self.info.clone());
        self.inner.call(req)
    }
}
