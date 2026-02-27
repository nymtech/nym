// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Shared helpers for the TCP/TLS smolmix examples.
//!
//! Provides a [`TlsOverTcp`] adapter that bridges rustls with smoltcp TCP
//! sockets, plus common utilities like [`init_logging`].
//!
//! The UDP example (`dns_udp`) does not depend on this module — it has no need
//! for TLS and manages its own setup.
//!
//! `TlsOverTcp` is a candidate for promotion into the library proper (e.g.
//! behind an optional `tls` feature flag) once the API stabilises, similar to
//! how `tokio-rustls` wraps a `TcpStream`.

use rustls::{pki_types::ServerName, ClientConfig, ClientConnection};
use smoltcp::socket::tcp;
use std::io::{self, Read, Write};
use std::sync::Arc;

/// Convenience alias used throughout the examples.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Initialise the tracing subscriber using nym-bin-common defaults.
pub fn init_logging() {
    nym_bin_common::logging::setup_tracing_logger();
}

/// Minimal TLS-over-TCP adapter for examples.
///
/// Bridges a rustls [`ClientConnection`] with a smoltcp [`tcp::Socket`] by
/// shuttling data between the two in a synchronous, non-blocking fashion.
///
/// Typical usage in a polling loop:
///
/// ```text
/// tls.read_tls(socket)?;   // socket -> TLS engine
/// tls.write_tls(socket)?;  // TLS engine -> socket
/// ```
pub struct TlsOverTcp {
    pub conn: ClientConnection,
}

#[allow(dead_code)]
impl TlsOverTcp {
    /// Create a new TLS client connection for `domain`.
    ///
    /// Loads the system-wide webpki root certificates and performs no client
    /// authentication. The returned connection is ready to begin the handshake
    /// once data starts flowing through [`write_tls`](Self::write_tls) /
    /// [`read_tls`](Self::read_tls).
    pub fn new(domain: &str) -> Result<Self, BoxError> {
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let server_name = ServerName::try_from(domain)?.to_owned();
        let conn = ClientConnection::new(Arc::new(config), server_name)?;

        Ok(Self { conn })
    }

    /// Drain pending TLS output into the TCP socket.
    ///
    /// Call this after any operation that may produce TLS records (handshake
    /// messages, application data, alerts) to push them onto the wire.
    pub fn write_tls(&mut self, socket: &mut tcp::Socket) -> Result<(), BoxError> {
        let mut buf = [0u8; 4096];
        while self.conn.wants_write() {
            match self.conn.write_tls(&mut buf.as_mut_slice()) {
                Ok(n) if n > 0 => {
                    socket
                        .send_slice(&buf[..n])
                        .map_err(|e| format!("TCP send: {e}"))?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Feed available TCP data into the TLS engine and process it.
    ///
    /// Reads whatever the socket has buffered, passes it through
    /// `rustls::ConnectionCommon::read_tls`, and then calls
    /// `process_new_packets` so the engine can advance its state machine
    /// (handshake, decryption, etc.).
    pub fn read_tls(&mut self, socket: &mut tcp::Socket) -> Result<(), BoxError> {
        if socket.can_recv() {
            let _ = socket.recv(|chunk| {
                if !chunk.is_empty() {
                    let _ = self.conn.read_tls(&mut io::Cursor::new(&mut *chunk));
                    let _ = self.conn.process_new_packets();
                }
                (chunk.len(), ())
            });
        }
        Ok(())
    }

    /// Encrypt `data` as TLS application data and flush it to the socket.
    pub fn send(&mut self, data: &[u8], socket: &mut tcp::Socket) -> Result<(), BoxError> {
        self.conn.writer().write_all(data)?;
        self.write_tls(socket)
    }

    /// Read and decrypt any available TLS application data from the socket.
    ///
    /// Returns the decrypted bytes (may be empty if nothing is available yet).
    pub fn recv(&mut self, socket: &mut tcp::Socket) -> Result<Vec<u8>, BoxError> {
        self.read_tls(socket)?;
        let mut result = Vec::new();
        let mut buf = vec![0u8; 4096];
        match self.conn.reader().read(&mut buf) {
            Ok(n) if n > 0 => result.extend_from_slice(&buf[..n]),
            _ => {}
        }
        Ok(result)
    }
}
