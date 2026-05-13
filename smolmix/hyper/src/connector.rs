// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

//! [`tower::Service<Uri>`] connector routing TCP + TLS through a [`Tunnel`].

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use hyper::Uri;
use hyper_util::rt::TokioIo;
use tower::Service;

use smolmix::Tunnel;
use smolmix_dns::Resolver;
use smolmix_tls::TlsConnector;

use crate::tls_stream::MaybeTlsStream;

/// A hyper connector that routes TCP (and optionally TLS) through a [`Tunnel`].
///
/// Implements [`tower::Service<Uri>`] so it plugs directly into hyper-util's `Client`.
/// DNS resolution also goes through the tunnel, preventing hostname leaks.
///
/// DNS lookups are cached internally via a shared [`smolmix_dns::Resolver`] —
/// repeat requests to the same host reuse cached records (subject to TTL).
///
/// Use this directly when you need a custom body type (e.g. `Full<Bytes>` for
/// POST requests) — see the [crate-level docs](crate#sending-request-bodies-post-put-etc)
/// for an example.
#[derive(Clone)]
pub struct SmolmixConnector {
    tunnel: Tunnel,
    tls: TlsConnector,
    resolver: Arc<Resolver>,
}

impl SmolmixConnector {
    /// Create a new connector for the given tunnel.
    ///
    /// Sets up a TLS connector with the standard webpki root certificates and
    /// a DNS resolver that caches lookups across requests.
    pub fn new(tunnel: &Tunnel) -> Self {
        Self {
            tunnel: tunnel.clone(),
            tls: smolmix_tls::connector(),
            resolver: Arc::new(Resolver::new(tunnel)),
        }
    }
}

impl Service<Uri> for SmolmixConnector {
    type Response = TokioIo<MaybeTlsStream>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let tunnel = self.tunnel.clone();
        let tls = self.tls.clone();
        let resolver = self.resolver.clone();

        Box::pin(async move {
            let scheme = uri.scheme_str().unwrap_or("https");
            let host = uri
                .host()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "URI missing host"))?
                .to_owned();
            let port = uri
                .port_u16()
                .unwrap_or(if scheme == "https" { 443 } else { 80 });

            let addrs = resolver.resolve(&host, port).await?;
            let addr = addrs
                .into_iter()
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, "no addresses"))?;

            let tcp = tunnel
                .tcp_connect(addr)
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            let stream = if scheme == "https" {
                let tls_stream = smolmix_tls::connect_with(&tls, tcp, &host).await?;
                MaybeTlsStream::Tls { inner: tls_stream }
            } else {
                MaybeTlsStream::Plain { inner: tcp }
            };

            Ok(TokioIo::new(stream))
        })
    }
}
