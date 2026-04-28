// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

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

/// Maximum number of HTTP redirects to follow before giving up.
const MAX_REDIRECTS: u8 = 5;

/// Execute a fetch request through the mixnet tunnel.
///
/// Pool hit → reuse TCP+TLS session. Pool miss → DNS → TCP → TLS from scratch.
/// Reusable connections are returned to the pool after the HTTP exchange.
/// Follows redirects (301/302/303/307/308) up to `MAX_REDIRECTS` hops.
///
/// Returns `{ body: Uint8Array, status, statusText, headers }` for Comlink.
pub async fn fetch(
    tunnel: &WasmTunnel,
    url_str: &str,
    init: &JsValue,
) -> Result<JsValue, FetchError> {
    let opts = parse_init(init)?;
    let mut url = Url::parse(url_str)?;
    let mut method = opts.method.clone();
    let mut body = opts.body.clone();

    for redirect_count in 0..=MAX_REDIRECTS {
        let host = url
            .host_str()
            .ok_or_else(|| FetchError::Http("URL has no host".into()))?
            .to_string();
        let port = url
            .port_or_known_default()
            .ok_or_else(|| FetchError::Http("URL has no port and scheme is unknown".into()))?;
        let is_https = url.scheme() == "https";

        if redirect_count == 0 {
            nym_wasm_utils::console_log!("[fetch] {} {} ({})", method, url.as_str(), url.scheme());
        } else {
            nym_wasm_utils::console_log!(
                "[fetch] redirect #{redirect_count} → {host}:{port} ({})",
                url.scheme()
            );
        }

        // Per-origin lock: concurrent requests to the same (host, port) queue
        // behind one connection instead of stampeding with parallel TCP+TLS
        // handshakes (which triggers server-side rate limiting / Cloudflare drops).
        // Scoped to connection acquisition only — the HTTP exchange runs unlocked
        // so multiple in-flight requests can share the same origin concurrently.
        let (conn, from_pool) = {
            let origin_lock = tunnel.origin_lock(&host, port);
            nym_wasm_utils::console_log!("[fetch] acquiring origin lock for {host}:{port}...");
            let _guard = origin_lock.lock().await;
            nym_wasm_utils::console_log!("[fetch] origin lock ACQUIRED for {host}:{port}");

            let result = match tunnel.take_pooled(&host, port) {
                Some(c) => {
                    nym_wasm_utils::console_log!("[fetch] pool HIT for {host}:{port}");
                    (c, true)
                }
                None => {
                    nym_wasm_utils::console_log!(
                        "[fetch] pool MISS for {host}:{port} — new connection"
                    );
                    (new_connection(tunnel, &host, port, is_https).await?, false)
                }
            };

            nym_wasm_utils::console_log!("[fetch] origin lock RELEASED for {host}:{port}");
            result
        };

        // 2. HTTP request + response (hyper takes ownership of stream, returns it)
        //
        // Retry-on-stale: pooled connections can go stale if the server or IPR
        // closed the TCP connection while it sat idle. hyper surfaces this as
        // "operation was canceled" or a connection error on the first write.
        // When a pooled connection fails, discard it and retry once with a
        // fresh connection. Only pooled connections get this grace — a fresh
        // connection failure is a real error.
        let http_result = http::request(conn, &method, &url, &opts.headers, body.as_deref()).await;

        let (response, reusable, conn) = match http_result {
            Ok(result) => result,
            Err(stale_err) if from_pool => {
                nym_wasm_utils::console_log!(
                    "[fetch] pooled connection failed ({stale_err}), retrying with fresh connection"
                );
                let fresh = new_connection(tunnel, &host, port, is_https).await?;
                match http::request(fresh, &method, &url, &opts.headers, body.as_deref()).await {
                    Ok(result) => result,
                    Err(e) => {
                        nym_wasm_utils::console_error!(
                            "[fetch] fresh connection also failed: {e} (pooled failed with: {stale_err})"
                        );
                        return Err(e);
                    }
                }
            }
            Err(e) => return Err(e),
        };

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
            tunnel.return_to_pool(host, port, conn);
        }

        // 4. Follow redirects (3xx with Location header)
        if (300..400).contains(&response.status) {
            if let Some(location) = response
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("location"))
                .map(|(_, v)| v.clone())
            {
                nym_wasm_utils::console_log!("[fetch] {} → Location: {location}", response.status);

                // Resolve relative URLs against the current request URL
                url = url.join(&location).map_err(|e| {
                    FetchError::Http(format!("invalid redirect URL '{location}': {e}"))
                })?;

                // 301/302/303: switch to GET and drop body (RFC 7231)
                // 307/308: preserve method and body
                if matches!(response.status, 301 | 302 | 303) {
                    method = "GET".into();
                    body = None;
                }

                continue;
            }
        }

        // 5. Non-redirect — serialise and return
        return serialise_response(&response);
    }

    Err(FetchError::Http(format!(
        "too many redirects (>{MAX_REDIRECTS})"
    )))
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

// RequestInit extraction (via js_sys::Reflect, no serde)

/// Extract method, headers, and body from a JS `RequestInit` object.
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

// Response serialisation (Rust → JS plain object)

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
