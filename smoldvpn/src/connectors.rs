// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Connector adapters so application HTTP/gRPC stacks flow through the tunnel.
//!
//! [`TunnelConnector`] is a [`tower::Service<Uri>`] that resolves the target
//! in-tunnel (via the smol-core DNS resolver) and opens an in-tunnel
//! [`TcpStream`], returning a `hyper`-compatible IO handle. It plugs directly
//! into `tonic` (`Endpoint::connect_with_connector`) and `hyper-util`'s
//! `Client`. `reqwest` can be layered on top of the same `hyper` client.
//!
//! ```no_run
//! # async fn example_connect(tunnel: &nym_smoldvpn::Tunnel) -> Result<(), Box<dyn std::error::Error>> {
//! let channel = tonic::transport::Endpoint::from_static("http://10.0.0.1:50051")
//!     .connect_with_connector(tunnel.connector())
//!     .await?;
//! # Ok(())
//! # }
//! ```

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll};

use http::Uri;
use hyper_util::rt::TokioIo;
use nym_smol_core::{Stack, TcpStream};
use tower::Service;

use crate::error::DvpnError;

/// A `tower` connector that dials through the dVPN tunnel. Cloneable and cheap;
/// holds the tunnel's swappable stack handle so it keeps working across a
/// runtime MTU change (which rebuilds the stack).
#[derive(Clone)]
pub struct TunnelConnector {
    stack: Arc<RwLock<Arc<Stack>>>,
}

impl TunnelConnector {
    pub(crate) fn new(stack: Arc<RwLock<Arc<Stack>>>) -> Self {
        Self { stack }
    }
}

impl Service<Uri> for TunnelConnector {
    type Response = TokioIo<TcpStream>;
    type Error = DvpnError;
    type Future = Pin<Box<dyn Future<Output = Result<TokioIo<TcpStream>, DvpnError>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        // Snapshot the current stack (survives a runtime MTU swap).
        let stack = self.stack.read().expect("stack lock poisoned").clone();
        Box::pin(async move {
            let host = uri
                .host()
                .ok_or_else(|| DvpnError::Config("URI missing host".into()))?
                .to_string();
            let port = uri.port_u16().unwrap_or(match uri.scheme_str() {
                Some("http") => 80,
                _ => 443,
            });
            // An IP-literal host (including a bracketed IPv6 form from `Uri::host()`, e.g.
            // "[::1]") must connect directly — routing it through DNS would turn "10.0.0.1" into a
            // bogus A-query. Only real hostnames fall through to the in-tunnel resolver.
            let unbracketed = host
                .strip_prefix('[')
                .and_then(|h| h.strip_suffix(']'))
                .unwrap_or(&host);
            let stream = if let Ok(ip) = unbracketed.parse::<IpAddr>() {
                stack.tcp_connect(SocketAddr::new(ip, port)).await?
            } else {
                stack.tcp_connect_host(&host, port).await?
            };
            Ok(TokioIo::new(stream))
        })
    }
}
