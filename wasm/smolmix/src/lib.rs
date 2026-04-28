// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! smolmix-wasm: drop-in browser networking over the Nym mixnet.
//!
//! Exposes two APIs that mirror the browser's native networking surface:
//!
//! - **`mixFetch(url, init)`** — drop-in `fetch()` replacement (HTTP/HTTPS)
//! - **`mixSocket(url, protocols, onEvent)`** — drop-in `WebSocket` replacement (WS/WSS)
//!
//! Both share the same mixnet tunnel (DNS → TCP → TLS), initialised once
//! via `setupMixTunnel(opts)` and torn down with `disconnectMixTunnel()`.

// All modules gated on wasm32 so `cargo check` on the host triple sees an empty crate.
#[cfg(target_arch = "wasm32")]
mod bridge;
#[cfg(target_arch = "wasm32")]
mod device;
#[cfg(target_arch = "wasm32")]
mod dns;
#[cfg(target_arch = "wasm32")]
mod error;
#[cfg(target_arch = "wasm32")]
mod fetch;
#[cfg(target_arch = "wasm32")]
mod http;
#[cfg(target_arch = "wasm32")]
mod ipr;
#[cfg(target_arch = "wasm32")]
mod reactor;
#[cfg(target_arch = "wasm32")]
mod tls;
#[cfg(target_arch = "wasm32")]
mod tunnel;

#[cfg(target_arch = "wasm32")]
pub use error::FetchError;
#[cfg(target_arch = "wasm32")]
pub use tunnel::WasmTunnel;

#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;
#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::future_to_promise;

/// Global tunnel singleton — set once by `setupMixTunnel`, stays in the OnceLock after shutdown.
#[cfg(target_arch = "wasm32")]
static TUNNEL: OnceLock<WasmTunnel> = OnceLock::new();

/// Active WebSocket handles — sender half of each connection's command channel.
#[cfg(target_arch = "wasm32")]
static WS_HANDLES: OnceLock<Mutex<HashMap<u32, WsHandle>>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static WS_NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// WASM entry point — installs the panic hook for readable stack traces.
#[wasm_bindgen(start)]
#[cfg(target_arch = "wasm32")]
pub fn main() {
    nym_wasm_utils::set_panic_hook();
}

/// Initialise the mixnet tunnel.
///
/// Must be called before `mixFetch` or `mixSocket`. The `opts` parameter accepts a JS object
/// with fields:
/// - `preferredIpr` (required): Nym address of the IPR exit node
/// - `clientId` (optional): storage namespace — randomise per session for clean state
/// - `forceTls` (optional, default `true`): use `wss://` for gateway connections
/// - `disablePoissonTraffic` (optional, default `false`): disable dummy traffic
/// - `disableCoverTraffic` (optional, default `false`): disable cover traffic loop
///
/// # Errors
///
/// Returns a rejected Promise if the tunnel is already initialised, if
/// `preferredIpr` is missing/invalid, or if the mixnet connection fails.
#[wasm_bindgen(js_name = "setupMixTunnel")]
#[cfg(target_arch = "wasm32")]
pub fn setup_mix_tunnel(opts: JsValue) -> js_sys::Promise {
    future_to_promise(async move {
        let result: Result<JsValue, FetchError> = async move {
            let ipr_str = js_sys::Reflect::get(&opts, &JsValue::from_str("preferredIpr"))
                .ok()
                .and_then(|v| v.as_string())
                .ok_or_else(|| FetchError::Tunnel("opts.preferredIpr is required".into()))?;

            let ipr_address: nym_wasm_client_core::Recipient = ipr_str
                .parse()
                .map_err(|e| FetchError::Tunnel(format!("invalid IPR address: {e}")))?;

            let client_id =
                js_string(&opts, "clientId").unwrap_or_else(|| "smolmix-wasm".to_string());
            let force_tls = js_bool(&opts, "forceTls", true);
            let disable_poisson = js_bool(&opts, "disablePoissonTraffic", false);
            let disable_cover = js_bool(&opts, "disableCoverTraffic", false);

            let tunnel_opts = tunnel::TunnelOpts {
                ipr_address,
                client_id,
                force_tls,
                disable_poisson_traffic: disable_poisson,
                disable_cover_traffic: disable_cover,
            };

            let tun = WasmTunnel::new(tunnel_opts).await?;

            TUNNEL
                .set(tun)
                .map_err(|_| FetchError::Tunnel("tunnel already initialised".into()))?;

            Ok(JsValue::UNDEFINED)
        }
        .await;
        result.map_err(Into::into)
    })
}

/// Read a bool from a JS object, defaulting if absent.
#[cfg(target_arch = "wasm32")]
fn js_bool(obj: &JsValue, key: &str, default: bool) -> bool {
    js_sys::Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// Read an optional string from a JS object.
#[cfg(target_arch = "wasm32")]
fn js_string(obj: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
}

/// Execute an HTTP request through the mixnet tunnel.
///
/// Returns `{ body: Uint8Array, status, statusText, headers }` — the TS
/// layer wraps this in a native `Response`.
#[wasm_bindgen(js_name = "mixFetch")]
#[cfg(target_arch = "wasm32")]
pub fn mix_fetch(url: String, init: JsValue) -> js_sys::Promise {
    future_to_promise(async move {
        let tunnel = TUNNEL.get().ok_or(FetchError::NotConnected)?;
        fetch::fetch(tunnel, &url, &init).await.map_err(Into::into)
    })
}

/// Disconnect from the mixnet. The tunnel becomes unusable until page reload.
#[wasm_bindgen(js_name = "disconnectMixTunnel")]
#[cfg(target_arch = "wasm32")]
pub fn disconnect_mix_tunnel() -> js_sys::Promise {
    future_to_promise(async {
        if let Some(tunnel) = TUNNEL.get() {
            tunnel.shutdown().await;
        }
        Ok(JsValue::UNDEFINED)
    })
}

// ---------------------------------------------------------------------------
// WebSocket
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
struct WsHandle {
    tx: futures::channel::mpsc::UnboundedSender<WsCommand>,
}

#[cfg(target_arch = "wasm32")]
enum WsCommand {
    Send(async_tungstenite::tungstenite::Message),
    Close(u16, String),
}

/// Open a WebSocket connection through the mixnet tunnel.
///
/// Performs DNS → TCP → TLS → HTTP 101 upgrade, then spawns a background
/// recv loop that pushes events to the `on_event` callback:
///
/// ```js
/// onEvent(handleId, "open",   protocol)   // upgrade complete
/// onEvent(handleId, "text",   string)      // text message from server
/// onEvent(handleId, "binary", Uint8Array)  // binary message from server
/// onEvent(handleId, "close",  info)        // connection closed
/// onEvent(handleId, "error",  message)     // unrecoverable error
/// ```
///
/// Returns a Promise that resolves to the handle ID (u32).
#[wasm_bindgen(js_name = "mixSocket")]
#[cfg(target_arch = "wasm32")]
pub fn mix_socket(url: String, protocols: JsValue, on_event: js_sys::Function) -> js_sys::Promise {
    future_to_promise(async move {
        let result: Result<JsValue, FetchError> = async {
            use async_tungstenite::tungstenite::client::IntoClientRequest;

            let tunnel = TUNNEL.get().ok_or(FetchError::NotConnected)?;

            let parsed = url::Url::parse(&url)
                .map_err(|e| FetchError::Http(format!("invalid WebSocket URL: {e}")))?;
            let host = parsed
                .host_str()
                .ok_or_else(|| FetchError::Http("URL has no host".into()))?;
            let port = parsed
                .port_or_known_default()
                .ok_or_else(|| FetchError::Http("URL has no port and scheme is unknown".into()))?;
            let is_tls = parsed.scheme() == "wss";
            let protocol_list = parse_protocols(&protocols);

            nym_wasm_utils::console_log!("[ws] connecting to {url}");

            // DNS → TCP → TLS (reuse the fetch pipeline)
            let conn = fetch::new_connection(tunnel, host, port, is_tls).await?;

            // Build upgrade request, add sub-protocols if specified
            let mut request = url.into_client_request()?;
            if !protocol_list.is_empty() {
                request.headers_mut().insert(
                    "Sec-WebSocket-Protocol",
                    protocol_list.join(", ").parse().unwrap(),
                );
            }

            // HTTP 101 upgrade — tungstenite handles key gen + accept verification
            let (ws_stream, response) = async_tungstenite::client_async(request, conn).await?;

            // Extract negotiated protocol from response headers
            let negotiated = response
                .headers()
                .get("sec-websocket-protocol")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            nym_wasm_utils::console_log!("[ws] upgrade complete (protocol={negotiated:?})");

            // Command channel (JS → background task)
            let (tx, rx) = futures::channel::mpsc::unbounded();
            let handle_id = WS_NEXT_ID.fetch_add(1, Ordering::Relaxed);

            let handles = WS_HANDLES.get_or_init(|| Mutex::new(HashMap::new()));
            handles.lock().unwrap().insert(handle_id, WsHandle { tx });

            // Fire "open" event before spawning the recv loop
            fire_ws_event(
                &on_event,
                handle_id,
                "open",
                &JsValue::from_str(&negotiated),
            );

            // Spawn background recv/send loop
            wasm_bindgen_futures::spawn_local(ws_task(handle_id, ws_stream, rx, on_event));

            Ok(JsValue::from(handle_id))
        }
        .await;

        result.map_err(Into::into)
    })
}

/// Send data over an open WebSocket (string → text, Uint8Array/ArrayBuffer → binary).
#[wasm_bindgen(js_name = "wsSend")]
#[cfg(target_arch = "wasm32")]
pub fn ws_send(handle_id: u32, data: JsValue) -> Result<(), JsValue> {
    use async_tungstenite::tungstenite::Message;

    let msg = if let Some(s) = data.as_string() {
        let preview = if s.len() <= 120 {
            &s
        } else {
            &s[..s.floor_char_boundary(120)]
        };
        nym_wasm_utils::console_log!("[ws:{handle_id}] send text ({} bytes): {preview}", s.len());
        Message::Text(s)
    } else if let Some(arr) = data.dyn_ref::<js_sys::Uint8Array>() {
        let v = arr.to_vec();
        nym_wasm_utils::console_log!(
            "[ws:{handle_id}] send binary ({} bytes): {}",
            v.len(),
            hex_preview(&v, 32)
        );
        Message::Binary(v)
    } else if let Some(buf) = data.dyn_ref::<js_sys::ArrayBuffer>() {
        let v = js_sys::Uint8Array::new(buf).to_vec();
        nym_wasm_utils::console_log!(
            "[ws:{handle_id}] send binary ({} bytes): {}",
            v.len(),
            hex_preview(&v, 32)
        );
        Message::Binary(v)
    } else {
        return Err(JsValue::from_str(
            "unsupported data type (expected string, Uint8Array, or ArrayBuffer)",
        ));
    };

    send_ws_command(handle_id, WsCommand::Send(msg))
}

/// Close an open WebSocket with a status code and reason.
#[wasm_bindgen(js_name = "wsClose")]
#[cfg(target_arch = "wasm32")]
pub fn ws_close(handle_id: u32, code: u16, reason: String) -> Result<(), JsValue> {
    send_ws_command(handle_id, WsCommand::Close(code, reason))
}

/// Push a command to a WebSocket handle.
#[cfg(target_arch = "wasm32")]
fn send_ws_command(handle_id: u32, cmd: WsCommand) -> Result<(), JsValue> {
    let handles = WS_HANDLES
        .get()
        .ok_or_else(|| JsValue::from_str("no active WebSocket connections"))?;
    let guard = handles.lock().unwrap();
    let handle = guard
        .get(&handle_id)
        .ok_or_else(|| JsValue::from_str(&format!("WebSocket handle {handle_id} not found")))?;

    handle
        .tx
        .unbounded_send(cmd)
        .map_err(|_| JsValue::from_str("WebSocket background task has stopped"))
}

/// Background task: reads from the WebSocket and dispatches JS commands.
/// Ping/pong is handled automatically by tungstenite.
#[cfg(target_arch = "wasm32")]
async fn ws_task(
    handle_id: u32,
    ws: async_tungstenite::WebSocketStream<tunnel::PooledConn>,
    rx: futures::channel::mpsc::UnboundedReceiver<WsCommand>,
    on_event: js_sys::Function,
) {
    use async_tungstenite::tungstenite::Message;
    use futures::{select, SinkExt, StreamExt};

    nym_wasm_utils::console_log!("[ws:{handle_id}] background task started");

    let (mut sink, stream) = ws.split();
    let mut stream = stream.fuse();
    let mut rx = rx.fuse();

    loop {
        select! {
            msg = stream.next() => match msg {
                Some(Ok(Message::Text(s))) => {
                    let preview = if s.len() <= 120 { &s } else { &s[..s.floor_char_boundary(120)] };
                    nym_wasm_utils::console_log!("[ws:{handle_id}] recv text ({} bytes): {preview}", s.len());
                    fire_ws_event(&on_event, handle_id, "text", &JsValue::from_str(&s));
                }
                Some(Ok(Message::Binary(b))) => {
                    nym_wasm_utils::console_log!("[ws:{handle_id}] recv binary ({} bytes): {}", b.len(), hex_preview(&b, 32));
                    fire_ws_event(
                        &on_event,
                        handle_id,
                        "binary",
                        &js_sys::Uint8Array::from(b.as_slice()).into(),
                    );
                }
                Some(Ok(Message::Close(frame))) => {
                    let info = frame
                        .map(|f| format!("{} {}", f.code, f.reason))
                        .unwrap_or_else(|| "1005".into());
                    nym_wasm_utils::console_log!("[ws:{handle_id}] recv close ({info})");
                    fire_ws_event(&on_event, handle_id, "close", &JsValue::from_str(&info));
                    ws_cleanup(handle_id);
                    return;
                }
                Some(Ok(_)) => continue, // Ping/Pong handled internally
                Some(Err(e)) => {
                    nym_wasm_utils::console_error!("[ws:{handle_id}] error: {e}");
                    fire_ws_event(
                        &on_event,
                        handle_id,
                        "error",
                        &JsValue::from_str(&e.to_string()),
                    );
                    ws_cleanup(handle_id);
                    return;
                }
                None => {
                    nym_wasm_utils::console_log!("[ws:{handle_id}] connection lost");
                    fire_ws_event(
                        &on_event,
                        handle_id,
                        "close",
                        &JsValue::from_str("1006 connection lost"),
                    );
                    ws_cleanup(handle_id);
                    return;
                }
            },
            cmd = rx.next() => match cmd {
                Some(WsCommand::Send(msg)) => {
                    if let Err(e) = sink.send(msg).await {
                        nym_wasm_utils::console_error!("[ws:{handle_id}] send error: {e}");
                        fire_ws_event(
                            &on_event,
                            handle_id,
                            "error",
                            &JsValue::from_str(&e.to_string()),
                        );
                        ws_cleanup(handle_id);
                        return;
                    }
                }
                Some(WsCommand::Close(code, reason)) => {
                    nym_wasm_utils::console_log!("[ws:{handle_id}] closing ({code} {reason})");
                    let info = format!("{code} {reason}");
                    let frame = async_tungstenite::tungstenite::protocol::CloseFrame {
                        code: code.into(),
                        reason: reason.into(),
                    };
                    let _ = sink.send(Message::Close(Some(frame))).await;
                    fire_ws_event(
                        &on_event,
                        handle_id,
                        "close",
                        &JsValue::from_str(&info),
                    );
                    ws_cleanup(handle_id);
                    return;
                }
                None => {
                    nym_wasm_utils::console_log!("[ws:{handle_id}] command channel dropped, closing");
                    let _ = sink.close().await;
                    ws_cleanup(handle_id);
                    return;
                }
            }
        }
    }
}

/// Parse a JS value into WebSocket sub-protocol strings.
#[cfg(target_arch = "wasm32")]
fn parse_protocols(val: &JsValue) -> Vec<String> {
    if val.is_undefined() || val.is_null() {
        return Vec::new();
    }
    if let Some(s) = val.as_string() {
        return vec![s];
    }
    if let Some(arr) = val.dyn_ref::<js_sys::Array>() {
        return (0..arr.length())
            .filter_map(|i| arr.get(i).as_string())
            .collect();
    }
    Vec::new()
}

/// Fire a WebSocket event: `onEvent(handleId, type, data)`.
#[cfg(target_arch = "wasm32")]
fn fire_ws_event(on_event: &js_sys::Function, handle_id: u32, event_type: &str, data: &JsValue) {
    if let Err(e) = on_event.call3(
        &JsValue::NULL,
        &JsValue::from(handle_id),
        &JsValue::from_str(event_type),
        data,
    ) {
        nym_wasm_utils::console_error!("[ws:{handle_id}] callback error: {e:?}");
    }
}

/// Hex preview of a buffer (truncated with `...` suffix).
#[cfg(target_arch = "wasm32")]
fn hex_preview(buf: &[u8], max_bytes: usize) -> String {
    let len = buf.len().min(max_bytes);
    let hex: String = buf[..len]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if buf.len() > max_bytes {
        format!("{hex} ...")
    } else {
        hex
    }
}

/// Remove a WebSocket handle from the global map.
#[cfg(target_arch = "wasm32")]
fn ws_cleanup(handle_id: u32) {
    nym_wasm_utils::console_log!("[ws:{handle_id}] cleanup");
    if let Some(handles) = WS_HANDLES.get() {
        handles.lock().unwrap().remove(&handle_id);
    }
}
