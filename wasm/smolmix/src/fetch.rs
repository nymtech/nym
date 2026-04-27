// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Fetch orchestrator — wires DNS → TCP → TLS → HTTP and handles JS interop.
//!
//! Extracts `RequestInit` fields from JS via `js_sys::Reflect` (no serde/Tsify),
//! then serialises the `HttpResponse` into a plain JS object for transfer across
//! the Comlink worker boundary.
//!
//! The TS layer reconstructs a native browser `Response` from this object:
//! `new Response(body, { status, statusText, headers })`.

use std::net::SocketAddr;

use js_sys::{Object, Reflect, Uint8Array};
use url::Url;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::dns;
use crate::error::FetchError;
use crate::http::{self, HttpResponse};
use crate::tls;
use crate::tunnel::{PooledConn, WasmTunnel};

/// Options extracted from a JS `RequestInit` object.
struct FetchInit {
    method: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

/// Execute a fetch request through the mixnet tunnel.
///
/// Tries the connection pool first (reusing a TCP+TLS session from a prior
/// request to the same origin). On pool miss, performs DNS → TCP → TLS from
/// scratch. After the HTTP exchange, reusable connections are returned to the
/// pool so the next request to the same host skips the ~14s setup.
///
/// Returns a JS object `{ body: Uint8Array, status, statusText, headers }`
/// suitable for transferring across the Comlink worker boundary.
pub async fn fetch(
    tunnel: &WasmTunnel,
    url_str: &str,
    init: &JsValue,
) -> Result<JsValue, FetchError> {
    let url = Url::parse(url_str)?;
    let opts = parse_init(init)?;

    let host = url
        .host_str()
        .ok_or_else(|| FetchError::Http("URL has no host".into()))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| FetchError::Http("URL has no port and scheme is unknown".into()))?;
    let is_https = url.scheme() == "https";

    nym_wasm_utils::console_log!("[fetch] {} {} ({})", opts.method, url_str, url.scheme());

    // Per-origin lock: concurrent requests to the same (host, port) queue
    // behind one connection instead of stampeding with parallel TCP+TLS
    // handshakes (which triggers server-side rate limiting / Cloudflare drops).
    let origin_lock = tunnel.origin_lock(host, port);
    let _guard = origin_lock.lock().await;

    // 1. Try pool, otherwise DNS → TCP → TLS
    let mut conn = match tunnel.take_pooled(host, port) {
        Some(c) => {
            nym_wasm_utils::console_log!("[fetch] pool HIT for {host}:{port}");
            c
        }
        None => {
            nym_wasm_utils::console_log!("[fetch] pool MISS for {host}:{port} — new connection");
            new_connection(tunnel, host, port, is_https).await?
        }
    };

    // 2. HTTP request + response
    let (response, reusable) = http::request(
        &mut conn,
        &opts.method,
        &url,
        &opts.headers,
        opts.body.as_deref(),
    )
    .await?;

    nym_wasm_utils::console_log!(
        "[fetch] {} {} ({} bytes, reusable={})",
        response.status,
        response.status_text,
        response.body.len(),
        reusable
    );

    // 3. Return connection to pool if reusable
    if reusable {
        nym_wasm_utils::console_log!("[fetch] returning connection to pool for {host}:{port}");
        tunnel.return_to_pool(host.to_string(), port, conn);
    }

    // 4. Serialise to a plain JS object for the Comlink boundary
    serialise_response(&response)
}

/// Create a fresh connection: DNS resolve → TCP connect → optional TLS.
pub(crate) async fn new_connection(
    tunnel: &WasmTunnel,
    host: &str,
    port: u16,
    is_https: bool,
) -> Result<PooledConn, FetchError> {
    let ip = dns::resolve(tunnel, host).await?;
    let addr = SocketAddr::new(ip, port);

    nym_wasm_utils::console_log!("[fetch] TCP connecting to {addr}...");
    let tcp = tunnel.tcp_connect(addr).await.map_err(FetchError::Io)?;
    nym_wasm_utils::console_log!("[fetch] TCP connected to {addr}");

    if is_https {
        nym_wasm_utils::console_log!("[fetch] TLS handshake with '{host}'...");
        let tls_stream = tls::connect(tcp, host).await?;
        nym_wasm_utils::console_log!("[fetch] TLS handshake complete with '{host}'");
        Ok(PooledConn::Tls(tls_stream))
    } else {
        Ok(PooledConn::Plain(tcp))
    }
}

// ---------------------------------------------------------------------------
// RequestInit extraction (via js_sys::Reflect, no serde)
// ---------------------------------------------------------------------------

/// Extract method, headers, and body from a JS `RequestInit` object.
///
/// Unknown fields are silently ignored — callers pass a standard `RequestInit`
/// and we pick out what we understand. This means new `RequestInit` fields
/// "just work" as we add support for them.
fn parse_init(init: &JsValue) -> Result<FetchInit, FetchError> {
    // Handle undefined/null init (bare GET request)
    if init.is_undefined() || init.is_null() {
        return Ok(FetchInit {
            method: "GET".into(),
            headers: Vec::new(),
            body: None,
        });
    }

    let method = Reflect::get(init, &JsValue::from_str("method"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "GET".into());

    let headers = extract_headers(init)?;
    let body = extract_body(init)?;

    Ok(FetchInit {
        method,
        headers,
        body,
    })
}

/// Extract headers from a plain JS object `{ "Header-Name": "value" }`.
///
/// TODO(v1.1): Also handle `Headers` objects and `[string, string][]` arrays.
fn extract_headers(init: &JsValue) -> Result<Vec<(String, String)>, FetchError> {
    let headers_val = match Reflect::get(init, &JsValue::from_str("headers")) {
        Ok(v) if !v.is_undefined() && !v.is_null() => v,
        _ => return Ok(Vec::new()),
    };

    let mut result = Vec::new();

    if let Some(obj) = headers_val.dyn_ref::<Object>() {
        let keys = Object::keys(obj);
        for i in 0..keys.length() {
            let key_val = keys.get(i);
            if let Some(key) = key_val.as_string() {
                if let Ok(val) = Reflect::get(obj, &key_val) {
                    if let Some(val_str) = val.as_string() {
                        result.push((key, val_str));
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Extract the request body. Supports string, Uint8Array, and ArrayBuffer.
///
/// TODO(v1.1): FormData, Blob, ReadableStream.
fn extract_body(init: &JsValue) -> Result<Option<Vec<u8>>, FetchError> {
    let body_val = match Reflect::get(init, &JsValue::from_str("body")) {
        Ok(v) if !v.is_undefined() && !v.is_null() => v,
        _ => return Ok(None),
    };

    // String body → UTF-8 bytes
    if let Some(s) = body_val.as_string() {
        return Ok(Some(s.into_bytes()));
    }

    // Uint8Array → copy to Vec<u8>
    if let Some(arr) = body_val.dyn_ref::<Uint8Array>() {
        return Ok(Some(arr.to_vec()));
    }

    // ArrayBuffer → wrap in Uint8Array → copy to Vec<u8>
    if let Some(buf) = body_val.dyn_ref::<js_sys::ArrayBuffer>() {
        let arr = Uint8Array::new(buf);
        return Ok(Some(arr.to_vec()));
    }

    Err(FetchError::Http(
        "unsupported body type (expected string, Uint8Array, or ArrayBuffer)".into(),
    ))
}

// ---------------------------------------------------------------------------
// Response serialisation (Rust → JS plain object)
// ---------------------------------------------------------------------------

/// Serialise an `HttpResponse` into a plain JS object for Comlink transfer.
///
/// Shape: `{ body: Uint8Array, status: number, statusText: string, headers: object }`
///
/// The TS layer reconstructs a native browser `Response` from this:
/// ```js
/// new Response(raw.body, { status: raw.status, statusText: raw.statusText, headers })
/// ```
fn serialise_response(resp: &HttpResponse) -> Result<JsValue, FetchError> {
    let obj = Object::new();
    let body = Uint8Array::from(resp.body.as_slice());

    set_prop(&obj, "body", &body)?;
    set_prop(&obj, "status", &JsValue::from(resp.status))?;
    set_prop(&obj, "statusText", &JsValue::from_str(&resp.status_text))?;

    let headers_obj = Object::new();
    for (k, v) in &resp.headers {
        set_prop(&headers_obj, k, &JsValue::from_str(v))?;
    }
    set_prop(&obj, "headers", &headers_obj)?;

    Ok(obj.into())
}

/// Helper: set a property on a JS object via `Reflect.set`.
fn set_prop(obj: &Object, key: &str, val: &JsValue) -> Result<(), FetchError> {
    Reflect::set(obj, &JsValue::from_str(key), val)
        .map(|_| ())
        .map_err(|e| FetchError::Js(format!("failed to set '{key}': {e:?}")))
}
