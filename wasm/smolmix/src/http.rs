// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! HTTP/1.1 client built on hyper 1.x (tokio-free).
//!
//! hyper 1.6+ makes tokio optional. With `default-features = false` and
//! `features = ["client", "http1"]`, it compiles on wasm32. The only glue
//! needed is `FuturesIo<T>`: an adapter from `futures::io::{AsyncRead,
//! AsyncWrite}` to `hyper::rt::{Read, Write}`.
//!
//! The connection driver is spawned via `wasm_bindgen_futures::spawn_local`.
//! After the request/response exchange, the IO is recovered through
//! `conn.without_shutdown()` so the underlying stream can be returned to
//! the connection pool.

use std::io;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::client::conn::http1;

use crate::error::FetchError;

/// Parsed HTTP response.
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Adapter: `futures::io::{AsyncRead, AsyncWrite}` → `hyper::rt::{Read, Write}`.
///
/// hyper's `Read` trait hands us a `ReadBufCursor` backed by uninitialised
/// memory. `futures::io::AsyncRead` requires an initialised `&mut [u8]`, so
/// we zero the buffer first. This is a cheap memset per read call.
struct FuturesIo<T>(T);

impl<T: AsyncRead + Unpin> hyper::rt::Read for FuturesIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        // Safety: we initialise the buffer below before passing to poll_read.
        let uninit_slice = unsafe { buf.as_mut() };
        // Zero the uninitialised memory so futures::io::AsyncRead is happy.
        let slice = init_slice(uninit_slice);

        match Pin::new(&mut self.get_mut().0).poll_read(cx, slice) {
            Poll::Ready(Ok(n)) => {
                unsafe { buf.advance(n) };
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: AsyncWrite + Unpin> hyper::rt::Write for FuturesIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().0).poll_close(cx)
    }
}

/// Zero a `MaybeUninit<u8>` slice and return it as `&mut [u8]`.
fn init_slice(buf: &mut [MaybeUninit<u8>]) -> &mut [u8] {
    for b in buf.iter_mut() {
        b.write(0);
    }
    // Safety: we just initialised every element.
    unsafe { &mut *(buf as *mut [MaybeUninit<u8>] as *mut [u8]) }
}

/// Send an HTTP/1.1 request and read the complete response.
///
/// Takes ownership of the stream (hyper's handshake consumes it) and
/// returns it as the third tuple element. The `bool` indicates whether
/// the connection can be pooled (deterministic body boundary + no
/// `Connection: close`).
pub async fn request<S>(
    stream: S,
    method: &str,
    url: &url::Url,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> Result<(HttpResponse, bool, S), FetchError>
where
    S: AsyncRead + AsyncWrite + Unpin + 'static,
{
    nym_wasm_utils::console_log!("[http] sending {method} request via hyper...");

    // Build the HTTP request
    let path = match url.query() {
        Some(q) => format!("{}?{q}", url.path()),
        None => url.path().to_string(),
    };

    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap_or("")),
        None => url.host_str().unwrap_or("").to_string(),
    };

    let body_bytes = body.map(Bytes::copy_from_slice).unwrap_or_default();
    let mut builder = http::Request::builder()
        .method(method)
        .uri(&path)
        .header("Host", &host)
        .header("Connection", "keep-alive");

    let mut has_content_length = false;
    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_str());
        if name.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        }
    }

    if body.is_some() && !has_content_length {
        builder = builder.header("Content-Length", body_bytes.len().to_string());
    }

    let req = builder
        .body(Full::new(body_bytes))
        .map_err(|e| FetchError::Http(format!("failed to build request: {e}")))?;

    // Perform HTTP/1 handshake — hyper takes ownership of the IO
    let (mut sender, conn) = http1::handshake(FuturesIo(stream))
        .await
        .map_err(FetchError::Hyper)?;

    // Spawn the connection driver. When we drop `sender`, the connection
    // completes and `without_shutdown()` returns the IO for reuse.
    let (parts_tx, parts_rx) = futures::channel::oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        let result = conn.without_shutdown().await;
        let _ = parts_tx.send(result);
    });

    // Send the request
    let response = sender.send_request(req).await.map_err(FetchError::Hyper)?;

    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    // Collect response headers
    let response_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Reusable unless the server signals `Connection: close`.
    let server_close = response_headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("connection") && v.eq_ignore_ascii_case("close"));
    let reusable = !server_close;

    // Log headers immediately so we know the server responded, even if
    // the body takes a long time to stream through the mixnet.
    let content_length = response_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<u64>().ok());

    match content_length {
        Some(len) => nym_wasm_utils::console_log!(
            "[http] {status} {status_text} — collecting body ({len} bytes)..."
        ),
        None => nym_wasm_utils::console_log!(
            "[http] {status} {status_text} — collecting body (chunked/unknown size)..."
        ),
    }

    // Dump response headers for diagnostics.
    for (k, v) in &response_headers {
        nym_wasm_utils::console_log!("[http]   {k}: {v}");
    }

    // Read body frame-by-frame to log progress (large mixnet downloads
    // can take 30s+ with no visible output otherwise).
    let mut body = response.into_body();
    let mut body_data = Vec::new();
    let expected = content_length.unwrap_or(0);
    let mut next_log_at: usize = 4096;

    loop {
        match body.frame().await {
            Some(Ok(frame)) => {
                if let Ok(data) = frame.into_data() {
                    let chunk_len = data.len();
                    body_data.extend_from_slice(&data);
                    if body_data.len() >= next_log_at {
                        nym_wasm_utils::console_log!(
                            "[http] progress: {} / {expected} bytes (chunk={chunk_len})",
                            body_data.len(),
                        );
                        next_log_at = body_data.len() + 4096;
                    }
                }
            }
            Some(Err(e)) => return Err(FetchError::Hyper(e)),
            None => break,
        }
    }

    nym_wasm_utils::console_log!(
        "[http] body complete: {} bytes, reusable={reusable}",
        body_data.len()
    );

    // Content preview — text for UTF-8-valid bodies, hex for binary
    if !body_data.is_empty() {
        let preview_len = body_data.len().min(200);
        let chunk = &body_data[..preview_len];
        let suffix = if body_data.len() > 200 { "..." } else { "" };

        if let Ok(text) = std::str::from_utf8(chunk) {
            nym_wasm_utils::console_log!("[http] body: {text}{suffix}");
        } else {
            nym_wasm_utils::console_log!(
                "[http] body ({} bytes): {}",
                body_data.len(),
                crate::hex_preview(&body_data, 64)
            );
        }
    }

    // Drop sender to signal the connection driver to complete
    drop(sender);

    // Recover the underlying stream from the connection driver
    let parts = parts_rx
        .await
        .map_err(|_| FetchError::Http("connection driver dropped".into()))?
        .map_err(FetchError::Hyper)?;
    let stream = parts.io.0;

    Ok((
        HttpResponse {
            status,
            status_text,
            headers: response_headers,
            body: body_data,
        },
        reusable,
        stream,
    ))
}
