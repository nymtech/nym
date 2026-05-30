// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! HTTP/1.1 client on hyper 1.x.

use std::io;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::client::conn::http1;

use crate::error::FetchError;

// Browser-shape header shim defaults.
//
// `mixFetch` ships these as fallbacks when the caller didn't set the header
// itself; caller-supplied values always win. Many CDNs (cloudflare bot
// management) and host policies (wikimedia's UA policy) reject requests
// that lack browser-canonical headers. See README "Browser-shape header
// shim" for the rationale, limits, and fingerprinting caveats.
//
// `DEFAULT_USER_AGENT` is pinned to a recent Chrome-on-Linux UA. Bump it
// when the Chrome major in the wild drifts far enough that this string
// starts looking suspicious (heuristic: more than 6 majors stale).
pub(crate) const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

pub(crate) const DEFAULT_ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,\
     image/avif,image/webp,*/*;q=0.8";

pub(crate) const DEFAULT_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9";

// `identity` rather than `gzip, deflate, br` because hyper 1.x in our wasm
// build doesn't carry a decompressor; advertising compression would surface
// gzip bytes to the caller un-decoded. Trade slightly less browser-shape
// for body-correctness.
pub(crate) const DEFAULT_ACCEPT_ENCODING: &str = "identity";

/// Parsed HTTP response.
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// `futures::io` to `hyper::rt` adapter. hyper hands us uninitialised memory;
/// `futures::io::AsyncRead` needs `&mut [u8]`, so we zero before passing.
struct HyperIoAdapter<T>(T);

impl<T: AsyncRead + Unpin> hyper::rt::Read for HyperIoAdapter<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        // SAFETY: `init_slice` initialises every byte before the caller sees it.
        let uninit_slice = unsafe { buf.as_mut() };
        let slice = init_slice(uninit_slice);

        match Pin::new(&mut self.get_mut().0).poll_read(cx, slice) {
            Poll::Ready(Ok(n)) => {
                // SAFETY: poll_read wrote `n` initialised bytes into `slice`,
                // which aliases the first `n` bytes of `buf`.
                unsafe { buf.advance(n) };
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: AsyncWrite + Unpin> hyper::rt::Write for HyperIoAdapter<T> {
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
/// The returned `bool` is whether the stream is poolable.
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
    crate::util::debug_log!("[http] sending {method} request via hyper...");

    let uri: http::Uri = url
        .as_str()
        .parse()
        .map_err(|e| FetchError::Http(format!("URI conversion: {e}")))?;
    let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    let host = uri
        .authority()
        .ok_or_else(|| FetchError::Http("URL has no authority for Host header".into()))?
        .as_str();

    let body_bytes = body.map(Bytes::copy_from_slice).unwrap_or_default();
    let mut builder = http::Request::builder()
        .method(method)
        .uri(path)
        .header("Host", host)
        .header("Connection", "keep-alive");

    // Track which browser-shape headers the caller has already set so the
    // shim below doesn't clobber explicit intent.
    let mut has_content_length = false;
    let mut has_user_agent = false;
    let mut has_accept = false;
    let mut has_accept_language = false;
    let mut has_accept_encoding = false;

    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_str());
        if name.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        } else if name.eq_ignore_ascii_case("user-agent") {
            has_user_agent = true;
        } else if name.eq_ignore_ascii_case("accept") {
            has_accept = true;
        } else if name.eq_ignore_ascii_case("accept-language") {
            has_accept_language = true;
        } else if name.eq_ignore_ascii_case("accept-encoding") {
            has_accept_encoding = true;
        }
    }

    if body.is_some() && !has_content_length {
        builder = builder.header("Content-Length", body_bytes.len().to_string());
    }

    // Browser-shape header shim. Definitions + rationale at the top of
    // this file.
    if !has_user_agent {
        builder = builder.header("User-Agent", DEFAULT_USER_AGENT);
    }
    if !has_accept {
        builder = builder.header("Accept", DEFAULT_ACCEPT);
    }
    if !has_accept_language {
        builder = builder.header("Accept-Language", DEFAULT_ACCEPT_LANGUAGE);
    }
    if !has_accept_encoding {
        builder = builder.header("Accept-Encoding", DEFAULT_ACCEPT_ENCODING);
    }

    let req = builder
        .body(Full::new(body_bytes))
        .map_err(|e| FetchError::Http(format!("failed to build request: {e}")))?;

    // Dump request headers (debug-only) so we can verify the shim and any
    // caller-supplied headers actually made it onto the wire. Matches the
    // response-header dump below for symmetry.
    for (k, v) in req.headers().iter() {
        crate::util::debug_log!(
            "[http] -> {}: {}",
            k.as_str(),
            v.to_str().unwrap_or("<non-ascii>")
        );
    }

    // Perform HTTP/1 handshake; hyper takes ownership of the IO
    let (mut sender, conn) = http1::handshake(HyperIoAdapter(stream))
        .await
        .map_err(FetchError::Hyper)?;

    // Spawn the connection driver. The driver only completes once the
    // request/response exchange is over AND the sender is dropped, at which
    // point `without_shutdown()` returns the IO so we can pool it.
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
        Some(len) => crate::util::debug_log!(
            "[http] {status} {status_text}; collecting body ({len} bytes)..."
        ),
        None => crate::util::debug_log!(
            "[http] {status} {status_text}; collecting body (chunked/unknown size)..."
        ),
    }

    // Dump response headers for diagnostics.
    for (k, v) in &response_headers {
        crate::util::debug_log!("[http]   {k}: {v}");
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
                        crate::util::debug_log!(
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

    crate::util::debug_log!(
        "[http] body complete: {} bytes, reusable={reusable}",
        body_data.len()
    );

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
