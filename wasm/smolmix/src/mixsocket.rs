// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>

//! `mixSocket`: drop-in `WebSocket` over the Nym mixnet tunnel.
//!
//! Performs DNS → TCP → TLS → HTTP 101 upgrade via the same pipeline as
//! `mixFetch`, then spawns a per-connection background task that bridges
//! JS callbacks to `async-tungstenite`'s sink/stream halves.
//!
//! Two static maps anchor the lifetime of every active connection:
//! - [`WS_HANDLES`] stores the command-channel sender keyed by handle id,
//! - [`WS_NEXT_ID`] hands out monotonically increasing u32 ids.
//!
//! The JS side never holds a `WebSocket` object; it holds an integer handle
//! and reaches the background task through `wsSend(id, data)` / `wsClose(id)`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{future_to_promise, spawn_local};

use crate::error::FetchError;
use crate::fetch;
use crate::stream::PooledConn;
use crate::util;
use crate::TUNNEL;

/// Active WebSocket handles: sender half of each connection's command channel.
static WS_HANDLES: OnceLock<Mutex<HashMap<u32, WsHandle>>> = OnceLock::new();
static WS_NEXT_ID: AtomicU32 = AtomicU32::new(1);

struct WsHandle {
    tx: futures::channel::mpsc::UnboundedSender<WsCommand>,
}

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

            util::debug_log!("[ws] connecting to {url}");

            let conn = fetch::new_connection(tunnel, host, port, is_tls).await?;

            let mut request = url.into_client_request()?;
            if !protocol_list.is_empty() {
                request.headers_mut().insert(
                    "Sec-WebSocket-Protocol",
                    protocol_list.join(", ").parse().unwrap(),
                );
            }

            // HTTP 101 upgrade; tungstenite handles key gen + accept verification
            let (ws_stream, response) = async_tungstenite::client_async(request, conn).await?;

            let negotiated = response
                .headers()
                .get("sec-websocket-protocol")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            util::debug_log!("[ws] upgrade complete (protocol={negotiated:?})");

            let (tx, rx) = futures::channel::mpsc::unbounded();
            let handle_id = WS_NEXT_ID.fetch_add(1, Ordering::Relaxed);

            let handles = WS_HANDLES.get_or_init(|| Mutex::new(HashMap::new()));
            handles.lock().unwrap().insert(handle_id, WsHandle { tx });

            // Fire "open" before spawning the recv loop so JS sees it first.
            fire_ws_event(
                &on_event,
                handle_id,
                "open",
                &JsValue::from_str(&negotiated),
            );

            spawn_local(ws_task(handle_id, ws_stream, rx, on_event));

            Ok(JsValue::from(handle_id))
        }
        .await;

        result.map_err(Into::into)
    })
}

/// Send data over an open WebSocket (string → text, Uint8Array/ArrayBuffer → binary).
#[wasm_bindgen(js_name = "wsSend")]
pub fn ws_send(handle_id: u32, data: JsValue) -> Result<(), JsValue> {
    use async_tungstenite::tungstenite::Message;

    let msg = if let Some(s) = data.as_string() {
        let preview = if s.len() <= 120 {
            &s
        } else {
            &s[..s.floor_char_boundary(120)]
        };
        util::debug_log!("[ws:{handle_id}] send text ({} bytes): {preview}", s.len());
        Message::Text(s)
    } else if let Some(arr) = data.dyn_ref::<js_sys::Uint8Array>() {
        let v = arr.to_vec();
        util::debug_log!(
            "[ws:{handle_id}] send binary ({} bytes): {}",
            v.len(),
            util::hex_preview(&v, 32)
        );
        Message::Binary(v)
    } else if let Some(buf) = data.dyn_ref::<js_sys::ArrayBuffer>() {
        let v = js_sys::Uint8Array::new(buf).to_vec();
        util::debug_log!(
            "[ws:{handle_id}] send binary ({} bytes): {}",
            v.len(),
            util::hex_preview(&v, 32)
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
pub fn ws_close(handle_id: u32, code: u16, reason: String) -> Result<(), JsValue> {
    send_ws_command(handle_id, WsCommand::Close(code, reason))
}

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
async fn ws_task(
    handle_id: u32,
    ws: async_tungstenite::WebSocketStream<PooledConn>,
    rx: futures::channel::mpsc::UnboundedReceiver<WsCommand>,
    on_event: js_sys::Function,
) {
    use async_tungstenite::tungstenite::Message;
    use futures::{select, SinkExt, StreamExt};

    util::debug_log!("[ws:{handle_id}] background task started");

    let (mut sink, stream) = ws.split();
    let mut stream = stream.fuse();
    let mut rx = rx.fuse();

    loop {
        select! {
            msg = stream.next() => match msg {
                Some(Ok(Message::Text(s))) => {
                    let preview = if s.len() <= 120 { &s } else { &s[..s.floor_char_boundary(120)] };
                    util::debug_log!("[ws:{handle_id}] recv text ({} bytes): {preview}", s.len());
                    fire_ws_event(&on_event, handle_id, "text", &JsValue::from_str(&s));
                }
                Some(Ok(Message::Binary(b))) => {
                    util::debug_log!("[ws:{handle_id}] recv binary ({} bytes): {}", b.len(), util::hex_preview(&b, 32));
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
                    util::debug_log!("[ws:{handle_id}] recv close ({info})");
                    fire_ws_event(&on_event, handle_id, "close", &JsValue::from_str(&info));
                    ws_cleanup(handle_id);
                    return;
                }
                Some(Ok(_)) => continue, // Ping/Pong handled internally
                Some(Err(e)) => {
                    util::debug_error!("[ws:{handle_id}] error: {e}");
                    fire_ws_event(&on_event, handle_id, "error", &JsValue::from_str(&e.to_string()));
                    ws_cleanup(handle_id);
                    return;
                }
                None => {
                    util::debug_log!("[ws:{handle_id}] connection lost");
                    fire_ws_event(&on_event, handle_id, "close", &JsValue::from_str("1006 connection lost"));
                    ws_cleanup(handle_id);
                    return;
                }
            },
            cmd = rx.next() => match cmd {
                Some(WsCommand::Send(msg)) => {
                    if let Err(e) = sink.send(msg).await {
                        util::debug_error!("[ws:{handle_id}] send error: {e}");
                        fire_ws_event(&on_event, handle_id, "error", &JsValue::from_str(&e.to_string()));
                        ws_cleanup(handle_id);
                        return;
                    }
                }
                Some(WsCommand::Close(code, reason)) => {
                    util::debug_log!("[ws:{handle_id}] closing ({code} {reason})");
                    let info = format!("{code} {reason}");
                    let frame = async_tungstenite::tungstenite::protocol::CloseFrame {
                        code: code.into(),
                        reason: reason.into(),
                    };
                    let _ = sink.send(Message::Close(Some(frame))).await;
                    fire_ws_event(&on_event, handle_id, "close", &JsValue::from_str(&info));
                    ws_cleanup(handle_id);
                    return;
                }
                None => {
                    util::debug_log!("[ws:{handle_id}] command channel dropped, closing");
                    let _ = sink.close().await;
                    ws_cleanup(handle_id);
                    return;
                }
            }
        }
    }
}

/// Parse a JS value into WebSocket sub-protocol strings.
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
fn fire_ws_event(on_event: &js_sys::Function, handle_id: u32, event_type: &str, data: &JsValue) {
    if let Err(e) = on_event.call3(
        &JsValue::NULL,
        &JsValue::from(handle_id),
        &JsValue::from_str(event_type),
        data,
    ) {
        util::debug_error!("[ws:{handle_id}] callback error: {e:?}");
    }
}

/// Remove a WebSocket handle from the global map.
fn ws_cleanup(handle_id: u32) {
    util::debug_log!("[ws:{handle_id}] cleanup");
    if let Some(handles) = WS_HANDLES.get() {
        handles.lock().unwrap().remove(&handle_id);
    }
}
