// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum_client_ip::RightmostXForwardedFor;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::warn;

/// Best-effort client IP extractor.
///
/// Prefers the rightmost entry of `X-Forwarded-For` (set by a trusted reverse
/// proxy); falls back to the TCP peer address when the header is absent, and to
/// the unspecified address when neither is available (tests).
#[derive(Debug, Clone, Copy)]
pub struct ClientIpAddr(pub IpAddr);

impl<S> FromRequestParts<S> for ClientIpAddr
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Ok(RightmostXForwardedFor(ip)) =
            RightmostXForwardedFor::from_request_parts(parts, state).await
        {
            return Ok(ClientIpAddr(ip));
        }
        if let Ok(ConnectInfo(addr)) =
            ConnectInfo::<SocketAddr>::from_request_parts(parts, state).await
        {
            return Ok(ClientIpAddr(addr.ip()));
        }
        warn!("ClientIpAddr: no X-Forwarded-For or ConnectInfo found; using 0.0.0.0 fallback");
        Ok(ClientIpAddr(IpAddr::V4(Ipv4Addr::UNSPECIFIED)))
    }
}
