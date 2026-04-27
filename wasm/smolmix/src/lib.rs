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

// All modules gated on wasm32 so workspace-level `cargo check` (which targets
// the host triple) sees an empty crate rather than failing on js-sys imports.
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
mod lp;
#[cfg(target_arch = "wasm32")]
mod reactor;
#[cfg(target_arch = "wasm32")]
mod tls;
#[cfg(target_arch = "wasm32")]
mod tunnel;
#[cfg(target_arch = "wasm32")]
mod ws;

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

/// Global tunnel singleton — initialised by `setupMixTunnel`, used by `mixFetch` and `mixSocket`.
///
/// Matches the OnceLock pattern from mix-fetch v1 (`MIX_FETCH: OnceLock<MixFetchClient>`).
/// The tunnel stays in the OnceLock even after shutdown (same as v1's `invalidated` flag).
#[cfg(target_arch = "wasm32")]
static TUNNEL: OnceLock<WasmTunnel> = OnceLock::new();

/// Active WebSocket handles — keyed by auto-incrementing ID.
///
/// Each handle holds the sender half of a command channel. The receiver half
/// lives in the background `ws_task` spawned by `mixSocket`.
#[cfg(target_arch = "wasm32")]
static WS_HANDLES: OnceLock<Mutex<HashMap<u32, WsHandle>>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static WS_NEXT_ID: AtomicU32 = AtomicU32::new(1);

/// WASM entry point — called automatically when the module loads.
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
                .map_err(|_| FetchError::Http("tunnel already initialised".into()))?;

            Ok(JsValue::UNDEFINED)
        }
        .await;
        result.map_err(Into::into)
    })
}

/// Read a boolean from a JS object, returning `default` if the key is missing or not a bool.
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

/// Execute an HTTP request through the Nym mixnet.
///
/// Drop-in replacement for `window.fetch()`. The `url` is a string URL and
/// `init` is a standard `RequestInit` object (method, headers, body).
///
/// Returns a Promise that resolves to a plain object
/// `{ body: Uint8Array, status: number, statusText: string, headers: object }`.
/// The TS layer reconstructs a native `Response` from this.
///
/// # Errors
///
/// Returns a rejected Promise if the tunnel isn't initialised, or if any
/// step (DNS, TCP, TLS, HTTP) fails.
#[wasm_bindgen(js_name = "mixFetch")]
#[cfg(target_arch = "wasm32")]
pub fn mix_fetch(url: String, init: JsValue) -> js_sys::Promise {
    future_to_promise(async move {
        let tunnel = TUNNEL.get().ok_or(FetchError::NotConnected)?;
        fetch::fetch(tunnel, &url, &init).await.map_err(Into::into)
    })
}

/// Gracefully disconnect from the Nym mixnet.
///
/// The tunnel singleton remains in the OnceLock but becomes unusable.
/// Subsequent `mixFetch` / `mixSocket` calls will fail until a page reload.
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
    Send(ws::WsMessage),
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
            let path = match parsed.query() {
                Some(q) => format!("{}?{}", parsed.path(), q),
                None => parsed.path().to_string(),
            };
            let protocol_list = parse_protocols(&protocols);

            nym_wasm_utils::console_log!("[ws] connecting to {url}");

            // DNS → TCP → TLS (reuse the fetch pipeline)
            let mut conn = fetch::new_connection(tunnel, host, port, is_tls).await?;

            // WebSocket upgrade handshake
            let negotiated = ws::upgrade(&mut conn, host, &path, &protocol_list).await?;

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
            let ws_conn = ws::WsConnection::new(conn);
            wasm_bindgen_futures::spawn_local(ws_task(handle_id, ws_conn, rx, on_event));

            Ok(JsValue::from(handle_id))
        }
        .await;

        result.map_err(Into::into)
    })
}

/// Send data over an open WebSocket.
///
/// Accepts a string (→ text frame) or Uint8Array/ArrayBuffer (→ binary frame).
/// Non-blocking: queues the message for the background task.
#[wasm_bindgen(js_name = "wsSend")]
#[cfg(target_arch = "wasm32")]
pub fn ws_send(handle_id: u32, data: JsValue) -> Result<(), JsValue> {
    let msg = if let Some(s) = data.as_string() {
        ws::WsMessage::Text(s)
    } else if let Some(arr) = data.dyn_ref::<js_sys::Uint8Array>() {
        ws::WsMessage::Binary(arr.to_vec())
    } else if let Some(buf) = data.dyn_ref::<js_sys::ArrayBuffer>() {
        ws::WsMessage::Binary(js_sys::Uint8Array::new(buf).to_vec())
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

/// Push a command to a WebSocket handle's channel.
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

/// Background task: reads from the WebSocket and processes commands from JS.
///
/// Splits the connection into independent read/write halves so the reader
/// task is never cancelled mid-frame. The reader runs in its own
/// `spawn_local` task, pushing messages through a channel. The main loop
/// uses `select!` to handle received messages and JS commands concurrently.
#[cfg(target_arch = "wasm32")]
async fn ws_task(
    handle_id: u32,
    ws: ws::WsConnection,
    rx: futures::channel::mpsc::UnboundedReceiver<WsCommand>,
    on_event: js_sys::Function,
) {
    use futures::{select, StreamExt};

    nym_wasm_utils::console_log!("[ws:{handle_id}] background task started");

    let (mut reader, mut writer) = ws.split();

    // Channel from reader sub-task → main loop
    let (msg_tx, msg_rx) =
        futures::channel::mpsc::unbounded::<Result<ws::WsMessage, crate::FetchError>>();

    // Reader sub-task: loops on recv(), never cancelled mid-frame
    wasm_bindgen_futures::spawn_local(async move {
        loop {
            match reader.recv().await {
                Ok(msg) => {
                    if msg_tx.unbounded_send(Ok(msg)).is_err() {
                        return; // main task dropped its receiver
                    }
                }
                Err(e) => {
                    let _ = msg_tx.unbounded_send(Err(e));
                    return;
                }
            }
        }
    });

    let mut msg_rx = msg_rx.fuse();
    let mut rx = rx.fuse();

    loop {
        select! {
            result = msg_rx.next() => {
                match result {
                    Some(Ok(msg)) => {
                        let (event_type, data) = match &msg {
                            ws::WsMessage::Text(s) => {
                                ("text", JsValue::from_str(s))
                            }
                            ws::WsMessage::Binary(b) => (
                                "binary",
                                js_sys::Uint8Array::from(b.as_slice()).into(),
                            ),
                        };
                        fire_ws_event(&on_event, handle_id, event_type, &data);
                    }
                    Some(Err(e)) => {
                        nym_wasm_utils::console_log!(
                            "[ws:{handle_id}] recv error/close: {e}"
                        );
                        fire_ws_event(
                            &on_event,
                            handle_id,
                            "close",
                            &JsValue::from_str(&e.to_string()),
                        );
                        ws_cleanup(handle_id);
                        return;
                    }
                    None => {
                        // Reader task exited without sending an error
                        nym_wasm_utils::console_log!(
                            "[ws:{handle_id}] reader task exited"
                        );
                        fire_ws_event(
                            &on_event,
                            handle_id,
                            "close",
                            &JsValue::from_str("1006 connection lost"),
                        );
                        ws_cleanup(handle_id);
                        return;
                    }
                }
            }
            cmd = rx.next() => {
                match cmd {
                    Some(WsCommand::Send(ref msg)) => {
                        let desc = match msg {
                            ws::WsMessage::Text(s) => {
                                format!("text {} bytes", s.len())
                            }
                            ws::WsMessage::Binary(b) => {
                                format!("binary {} bytes", b.len())
                            }
                        };
                        nym_wasm_utils::console_log!(
                            "[ws:{handle_id}] cmd send ({desc})"
                        );
                        if let Err(e) = writer.send(msg).await {
                            nym_wasm_utils::console_log!(
                                "[ws:{handle_id}] send error: {e}"
                            );
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
                        nym_wasm_utils::console_log!(
                            "[ws:{handle_id}] cmd close ({code} {reason})"
                        );
                        let _ = writer.close(code, &reason).await;
                        fire_ws_event(
                            &on_event,
                            handle_id,
                            "close",
                            &JsValue::from_str(&format!("{code} {reason}")),
                        );
                        ws_cleanup(handle_id);
                        return;
                    }
                    None => {
                        nym_wasm_utils::console_log!(
                            "[ws:{handle_id}] channel closed — sending close"
                        );
                        let _ = writer.close(1001, "going away").await;
                        ws_cleanup(handle_id);
                        return;
                    }
                }
            }
        }
    }
}

/// Parse a JS value into a list of WebSocket sub-protocol strings.
///
/// Accepts: `undefined` → empty, `"proto"` → single, `["a", "b"]` → list.
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
    let _ = on_event.call3(
        &JsValue::NULL,
        &JsValue::from(handle_id),
        &JsValue::from_str(event_type),
        data,
    );
}

/// Remove a WebSocket handle from the global map.
#[cfg(target_arch = "wasm32")]
fn ws_cleanup(handle_id: u32) {
    if let Some(handles) = WS_HANDLES.get() {
        handles.lock().unwrap().remove(&handle_id);
    }
}
