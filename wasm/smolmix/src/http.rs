// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Minimal HTTP/1.1 client built on httparse.
//!
//! Three stages:
//! 1. Request serialisation — build `METHOD /path HTTP/1.1\r\n...` into a buffer
//! 2. Response header parsing — read until `\r\n\r\n`, feed to httparse
//! 3. Body reading — Content-Length, chunked transfer, or read-until-EOF
//!
//! Supports connection reuse: `request()` returns `(HttpResponse, bool)` where
//! the bool indicates whether the connection can be pooled for subsequent
//! requests (deterministic body boundary + no `Connection: close`).

use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::FetchError;

/// Maximum header section size (16 KiB). Prevents unbounded allocation
/// if a server sends endless headers.
const MAX_HEADER_SIZE: usize = 16_384;

/// Read buffer chunk size.
const READ_CHUNK: usize = 4096;

/// Maximum number of headers httparse will parse.
const MAX_HEADERS: usize = 64;

/// Parsed HTTP response.
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    /// Headers with original casing preserved (for forwarding to the browser).
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Send an HTTP/1.1 request and read the complete response.
///
/// Returns `(response, reusable)` where `reusable` is `true` when the
/// connection can be returned to a pool: the body was read via a
/// deterministic boundary (Content-Length or chunked) and the server
/// did not send `Connection: close`.
///
/// Generic over any stream implementing `futures::io::{AsyncRead, AsyncWrite}`.
/// Works equally with a plain `WasmTcpStream` (HTTP) or a
/// `futures_rustls::client::TlsStream<WasmTcpStream>` (HTTPS).
pub async fn request<S>(
    stream: &mut S,
    method: &str,
    url: &url::Url,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> Result<(HttpResponse, bool), FetchError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    nym_wasm_utils::console_log!("[http] sending {method} request...");
    send_request(stream, method, url, headers, body).await?;
    nym_wasm_utils::console_log!("[http] request sent, reading response...");
    read_response(stream).await
}

// ---------------------------------------------------------------------------
// Request serialisation
// ---------------------------------------------------------------------------

/// Format and send a complete HTTP/1.1 request.
///
/// Builds the entire request into a single buffer before writing,
/// minimising the number of TLS records when the stream is encrypted.
async fn send_request<S>(
    stream: &mut S,
    method: &str,
    url: &url::Url,
    headers: &[(String, String)],
    body: Option<&[u8]>,
) -> Result<(), FetchError>
where
    S: AsyncWrite + Unpin,
{
    let mut buf = Vec::with_capacity(256);

    // Request line: METHOD /path?query HTTP/1.1
    let path = match url.query() {
        Some(q) => format!("{}?{q}", url.path()),
        None => url.path().to_string(),
    };
    buf.extend_from_slice(format!("{method} {path} HTTP/1.1\r\n").as_bytes());

    // Host header (include port only if non-default)
    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap_or("")),
        None => url.host_str().unwrap_or("").to_string(),
    };
    buf.extend_from_slice(format!("Host: {host}\r\n").as_bytes());

    // User-supplied headers
    let mut has_content_length = false;
    for (name, value) in headers {
        buf.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        if name.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        }
    }

    // Auto-add Content-Length for bodies (unless caller set it)
    if let Some(b) = body {
        if !has_content_length {
            buf.extend_from_slice(format!("Content-Length: {}\r\n", b.len()).as_bytes());
        }
    }

    buf.extend_from_slice(b"Connection: keep-alive\r\n");

    // End of headers
    buf.extend_from_slice(b"\r\n");

    // Body (if any)
    if let Some(b) = body {
        buf.extend_from_slice(b);
    }

    stream.write_all(&buf).await?;
    stream.flush().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Response header parsing
// ---------------------------------------------------------------------------

/// Read and parse an HTTP/1.1 response (headers + body).
///
/// Returns `(response, reusable)` — see `request()`.
async fn read_response<S>(stream: &mut S) -> Result<(HttpResponse, bool), FetchError>
where
    S: AsyncRead + Unpin,
{
    let mut buf = Vec::with_capacity(READ_CHUNK);
    let mut tmp = [0u8; READ_CHUNK];

    // Phase 1: Read until we see the header/body boundary (\r\n\r\n).
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            nym_wasm_utils::console_error!(
                "[http] connection closed before headers complete (got {} bytes so far)",
                buf.len()
            );
            return Err(FetchError::Http(
                "connection closed before headers complete".into(),
            ));
        }
        buf.extend_from_slice(&tmp[..n]);

        if buf.len() > MAX_HEADER_SIZE {
            return Err(FetchError::Http("headers exceed 16 KiB limit".into()));
        }

        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }

    // Phase 2: Parse headers with httparse.
    let mut headers_buf = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut parsed = httparse::Response::new(&mut headers_buf);
    let body_offset = match parsed.parse(&buf)? {
        httparse::Status::Complete(n) => n,
        httparse::Status::Partial => {
            return Err(FetchError::Http("incomplete HTTP response headers".into()));
        }
    };

    let status = parsed.code.unwrap_or(0);
    let status_text = parsed.reason.unwrap_or("").to_string();

    // Preserve original header casing — browsers may rely on it, and the
    // Headers constructor is case-insensitive anyway.
    let response_headers: Vec<(String, String)> = parsed
        .headers
        .iter()
        .map(|h| {
            (
                h.name.to_string(),
                String::from_utf8_lossy(h.value).to_string(),
            )
        })
        .collect();

    // Phase 3: Read the body.
    let body_strategy = if get_header(&response_headers, "content-length").is_some() {
        "content-length"
    } else if get_header(&response_headers, "transfer-encoding")
        .map(|v| v.to_ascii_lowercase().contains("chunked"))
        .unwrap_or(false)
    {
        "chunked"
    } else {
        "read-until-eof"
    };
    nym_wasm_utils::console_log!("[http] {status} {status_text} — body via {body_strategy}");

    let initial_body = buf[body_offset..].to_vec();
    let (body, deterministic) = read_body(stream, &response_headers, initial_body).await?;

    // Connection is reusable when we know exactly where this response ends
    // (Content-Length or chunked) AND the server didn't ask us to close.
    let server_close = get_header(&response_headers, "connection")
        .map(|v| v.eq_ignore_ascii_case("close"))
        .unwrap_or(false);
    let reusable = deterministic && !server_close;

    Ok((
        HttpResponse {
            status,
            status_text,
            headers: response_headers,
            body,
        },
        reusable,
    ))
}

// ---------------------------------------------------------------------------
// Body reading strategies
// ---------------------------------------------------------------------------

/// Dispatch to the appropriate body reader based on response headers.
///
/// Returns `(body, deterministic)` where `deterministic` is `true` when
/// Content-Length or chunked encoding provided a clear message boundary
/// (meaning the stream is positioned at the start of the next response).
async fn read_body<S>(
    stream: &mut S,
    headers: &[(String, String)],
    initial: Vec<u8>,
) -> Result<(Vec<u8>, bool), FetchError>
where
    S: AsyncRead + Unpin,
{
    if let Some(len_str) = get_header(headers, "content-length") {
        let len: usize = len_str
            .trim()
            .parse()
            .map_err(|_| FetchError::Http(format!("invalid Content-Length: {len_str}")))?;
        let body = read_content_length(stream, len, initial).await?;
        return Ok((body, true));
    }

    if let Some(te) = get_header(headers, "transfer-encoding") {
        if te.to_ascii_lowercase().contains("chunked") {
            let body = read_chunked(stream, initial).await?;
            return Ok((body, true));
        }
    }

    // Default: read until the server closes the connection.
    let body = read_until_eof(stream, initial).await?;
    Ok((body, false))
}

/// Read exactly `total_len` bytes of body.
async fn read_content_length<S>(
    stream: &mut S,
    total_len: usize,
    initial: Vec<u8>,
) -> Result<Vec<u8>, FetchError>
where
    S: AsyncRead + Unpin,
{
    let mut body = initial;
    if body.len() >= total_len {
        body.truncate(total_len);
        return Ok(body);
    }

    let remaining = total_len - body.len();
    let mut rest = vec![0u8; remaining];
    stream.read_exact(&mut rest).await?;
    body.extend_from_slice(&rest);
    Ok(body)
}

/// Parse chunked transfer encoding.
///
/// Format per chunk: `<hex-size>\r\n<data>\r\n`
/// Terminated by: `0\r\n\r\n`
async fn read_chunked<S>(stream: &mut S, initial: Vec<u8>) -> Result<Vec<u8>, FetchError>
where
    S: AsyncRead + Unpin,
{
    let mut raw = initial;
    let mut body = Vec::new();

    loop {
        // Ensure we have a complete chunk-size line.
        while !has_crlf(&raw) {
            let n = read_more(stream, &mut raw).await?;
            if n == 0 {
                return Err(FetchError::Http("unexpected EOF in chunked body".into()));
            }
        }

        // Parse the hex chunk size (ignore optional chunk extensions after ';').
        let crlf_pos = raw
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| FetchError::Http("missing CRLF in chunk header".into()))?;

        let size_line = std::str::from_utf8(&raw[..crlf_pos])
            .map_err(|_| FetchError::Http("invalid chunk size encoding".into()))?;
        let size_hex = size_line.split(';').next().unwrap_or("0").trim();
        let chunk_size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| FetchError::Http(format!("invalid chunk size: {size_hex}")))?;

        // Consume chunk-size line.
        raw = raw[crlf_pos + 2..].to_vec();

        // Zero-size chunk signals end of body.
        if chunk_size == 0 {
            break;
        }

        // Read chunk data + trailing CRLF.
        let needed = chunk_size + 2;
        while raw.len() < needed {
            let n = read_more(stream, &mut raw).await?;
            if n == 0 {
                return Err(FetchError::Http("unexpected EOF in chunk data".into()));
            }
        }

        body.extend_from_slice(&raw[..chunk_size]);
        raw = raw[chunk_size + 2..].to_vec();
    }

    Ok(body)
}

/// Read until the server closes the connection (Connection: close fallback).
///
/// Treats `UnexpectedEof` as a clean close — this is what rustls returns
/// when the peer closes the TLS connection without sending a `close_notify`
/// alert.  Technically a protocol violation, but extremely common (CDNs,
/// reverse proxies, many HTTP servers).  The data buffered so far is valid.
async fn read_until_eof<S>(stream: &mut S, initial: Vec<u8>) -> Result<Vec<u8>, FetchError>
where
    S: AsyncRead + Unpin,
{
    let mut body = initial;
    let mut tmp = [0u8; READ_CHUNK];
    loop {
        match stream.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&tmp[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(body)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Case-insensitive header lookup.
fn get_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Check if the buffer contains a CRLF sequence.
fn has_crlf(buf: &[u8]) -> bool {
    buf.windows(2).any(|w| w == b"\r\n")
}

/// Read a chunk from the stream into the buffer. Returns bytes read.
async fn read_more<S>(stream: &mut S, buf: &mut Vec<u8>) -> Result<usize, FetchError>
where
    S: AsyncRead + Unpin,
{
    let mut tmp = [0u8; READ_CHUNK];
    let n = stream.read(&mut tmp).await?;
    buf.extend_from_slice(&tmp[..n]);
    Ok(n)
}
