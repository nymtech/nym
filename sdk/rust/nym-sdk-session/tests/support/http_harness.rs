// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! HTTP-level fault injection: a minimal mock nym-api whose routes are
//! independently scriptable as healthy / slow / erroring / **hanging**
//! (accept the connection, read the request, never respond — the exact
//! client-visible behavior observed against mainnet's aggregated
//! expiration-date-signatures endpoint).
//!
//! Hand-rolled on a raw `TcpListener` (design D2): the critical `Hang` mode
//! must hold an accepted connection open indefinitely, which is awkward to
//! express in off-the-shelf mock servers and trivial here. Only the sliver of
//! HTTP/1.1 the real `nym_http_api_client` GETs need is implemented.

#![allow(clippy::expect_used, clippy::unwrap_used, dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// How a route behaves. Keys are exact paths (no query string), e.g.
/// `/v1/ecash/aggregated-expiration-date-signatures`.
#[derive(Clone)]
pub enum FaultMode {
    /// 200 with the given JSON body.
    Healthy(String),
    /// 200 with the given JSON body, after a delay.
    Slow(Duration, String),
    /// An HTTP error status with an empty body.
    Error(u16),
    /// Accept, read the request, never respond (the observed mainnet outage).
    Hang,
}

type Routes = Arc<Mutex<HashMap<String, FaultMode>>>;

/// One mock nym-api server. Dropping it aborts the accept loop (and thereby
/// releases any connections parked in `Hang`).
pub struct MockNymApi {
    url: String,
    routes: Routes,
    accept_task: tokio::task::JoinHandle<()>,
}

impl MockNymApi {
    /// Bind on an ephemeral localhost port and serve `routes`.
    pub async fn spawn(routes: HashMap<String, FaultMode>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let routes: Routes = Arc::new(Mutex::new(routes));
        let accept_routes = routes.clone();
        let accept_task = tokio::spawn(async move {
            loop {
                let Ok((conn, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(handle_connection(conn, accept_routes.clone()));
            }
        });
        MockNymApi {
            url: format!("http://{addr}/"),
            routes,
            accept_task,
        }
    }

    /// Base URL (the DKG `announce_address` for this mock signer).
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Re-script a route at runtime.
    pub fn set(&self, path: impl Into<String>, mode: FaultMode) {
        self.routes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(path.into(), mode);
    }
}

impl Drop for MockNymApi {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

/// Serve one connection: parse the request line of each GET, look up the
/// route's current mode, respond (or don't). Handles sequential requests on a
/// kept-alive connection.
async fn handle_connection(mut conn: TcpStream, routes: Routes) {
    let mut buf = Vec::with_capacity(2048);
    loop {
        // Read until the end of the request head.
        let mut chunk = [0u8; 1024];
        let head_end = loop {
            if let Some(pos) = find_head_end(&buf) {
                break pos;
            }
            match conn.read(&mut chunk).await {
                Ok(0) | Err(_) => return, // peer closed
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
        };

        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
        buf.drain(..head_end + 4);
        let path = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .map(|target| target.split('?').next().unwrap_or(target).to_string())
            .unwrap_or_default();

        let mode = routes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&path)
            .cloned();

        match mode {
            Some(FaultMode::Healthy(body)) => {
                if respond_json(&mut conn, 200, &body).await.is_err() {
                    return;
                }
            }
            Some(FaultMode::Slow(delay, body)) => {
                tokio::time::sleep(delay).await;
                if respond_json(&mut conn, 200, &body).await.is_err() {
                    return;
                }
            }
            Some(FaultMode::Error(status)) => {
                if respond_json(&mut conn, status, "{}").await.is_err() {
                    return;
                }
            }
            Some(FaultMode::Hang) => {
                // The observed outage: connection accepted, request consumed,
                // no response ever. Parked until the client (or test) gives up
                // and the connection/server is dropped.
                std::future::pending::<()>().await;
            }
            None => {
                if respond_json(&mut conn, 404, "{}").await.is_err() {
                    return;
                }
            }
        }
    }
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

async fn respond_json(conn: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let reason = if status == 200 { "OK" } else { "Error" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    conn.write_all(response.as_bytes()).await
}
