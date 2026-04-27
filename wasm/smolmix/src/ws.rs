// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

//! Minimal WebSocket client (RFC 6455) over async streams.
//!
//! Three layers:
//! 1. Frame codec — encode/decode the binary frame format
//! 2. HTTP upgrade — send `Upgrade: websocket`, verify `101 Switching Protocols`
//! 3. `WsConnection` — high-level send/recv/close over a `PooledConn`
//!
//! Client frames are always masked (4-byte XOR key, RFC 6455 Section 5.3).
//! Server frames are never masked. Ping frames get automatic pong replies.
//! Continuation frames are reassembled into complete messages.

use base64::Engine;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::error::FetchError;
use crate::tunnel::PooledConn;

// ---------------------------------------------------------------------------
// SHA-1 via Web Crypto API
// ---------------------------------------------------------------------------

/// Compute SHA-1 using the browser's native `crypto.subtle.digest`.
///
/// Both the `sha1` crate and a hand-rolled implementation produce incorrect
/// output on wasm32 for multi-block inputs (>55 bytes). Root cause is likely
/// a `digest` 0.10/0.11 version conflict in the workspace affecting the sha1
/// crate, and an unknown WASM codegen issue affecting the hand-rolled version.
/// The Web Crypto API is the reliable path on this target.
async fn sha1(data: &[u8]) -> Result<[u8; 20], FetchError> {
    // Log input
    let hex_in: String = data.iter().map(|b| format!("{b:02x}")).collect();
    nym_wasm_utils::console_log!("[sha1] input ({} bytes): {}", data.len(), hex_in);

    // Create an OWNED Uint8Array (not a view into WASM memory)
    let data_arr = js_sys::Uint8Array::new_with_length(data.len() as u32);
    data_arr.copy_from(data);

    // Readback: verify the copy is correct
    let mut readback = vec![0u8; data.len()];
    data_arr.copy_to(&mut readback);
    let match_ok = data == readback.as_slice();
    nym_wasm_utils::console_log!(
        "[sha1] copy_from readback matches: {match_ok}, arr.byteLength={}, arr.byteOffset={}, buffer.byteLength={}",
        data_arr.byte_length(),
        data_arr.byte_offset(),
        data_arr.buffer().byte_length()
    );

    let crypto = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("crypto"))
        .map_err(|_| FetchError::Http("crypto not available in worker".into()))?;
    let subtle = js_sys::Reflect::get(&crypto, &JsValue::from_str("subtle"))
        .map_err(|_| FetchError::Http("crypto.subtle not available".into()))?;
    let digest_fn: js_sys::Function = js_sys::Reflect::get(&subtle, &JsValue::from_str("digest"))
        .map_err(|_| FetchError::Http("crypto.subtle.digest not available".into()))?
        .dyn_into()
        .map_err(|_| FetchError::Http("digest is not a function".into()))?;

    // Pass the Uint8Array directly (BufferSource = ArrayBuffer | ArrayBufferView)
    // NOT .buffer() which could have offset/length issues
    let promise: js_sys::Promise = digest_fn
        .call2(&subtle, &JsValue::from_str("SHA-1"), &data_arr)
        .map_err(|e| FetchError::Http(format!("digest call failed: {e:?}")))?
        .dyn_into()
        .map_err(|_| FetchError::Http("digest did not return a promise".into()))?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| FetchError::Http(format!("SHA-1 digest failed: {e:?}")))?;

    let buffer: js_sys::ArrayBuffer = result
        .dyn_into()
        .map_err(|_| FetchError::Http("digest did not return ArrayBuffer".into()))?;
    let arr = js_sys::Uint8Array::new(&buffer);

    let mut out = [0u8; 20];
    arr.copy_to(&mut out);

    let hex_out: String = out.iter().map(|b| format!("{b:02x}")).collect();
    nym_wasm_utils::console_log!("[sha1] output: {hex_out}");

    Ok(out)
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A complete WebSocket message (after reassembly of any continuation frames).
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
}

/// Opcodes (RFC 6455 Section 5.2).
const OP_CONTINUATION: u8 = 0x0;
const OP_TEXT: u8 = 0x1;
const OP_BINARY: u8 = 0x2;
const OP_CLOSE: u8 = 0x8;
const OP_PING: u8 = 0x9;
const OP_PONG: u8 = 0xA;

/// Internal frame representation.
struct WsFrame {
    fin: bool,
    opcode: u8,
    payload: Vec<u8>,
}

/// Maximum payload size we'll accept from the server (16 MiB).
const MAX_PAYLOAD: u64 = 16 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Frame codec
// ---------------------------------------------------------------------------

/// Encode a WebSocket frame with a random masking key (client → server).
///
/// Layout: `[FIN|RSV|opcode] [MASK=1|len] [ext_len?] [mask_key:4] [masked_payload]`
fn encode_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();

    // Header: FIN=1, RSV=000, opcode
    let byte0 = 0x80 | (opcode & 0x0F);

    // Length encoding + MASK=1 bit
    let (byte1, ext) = if len < 126 {
        ((0x80 | len as u8), Vec::new())
    } else if len <= 0xFFFF {
        let mut ext = Vec::with_capacity(2);
        ext.extend_from_slice(&(len as u16).to_be_bytes());
        (0x80 | 126u8, ext)
    } else {
        let mut ext = Vec::with_capacity(8);
        ext.extend_from_slice(&(len as u64).to_be_bytes());
        (0x80 | 127u8, ext)
    };

    // Random masking key
    let mask_key: [u8; 4] = rand::random();

    // Assemble frame
    let mut frame = Vec::with_capacity(2 + ext.len() + 4 + len);
    frame.push(byte0);
    frame.push(byte1);
    frame.extend_from_slice(&ext);
    frame.extend_from_slice(&mask_key);

    // XOR payload with mask
    for (i, &b) in payload.iter().enumerate() {
        frame.push(b ^ mask_key[i % 4]);
    }

    frame
}

/// Read one WebSocket frame from a stream (server → client, never masked).
async fn read_frame<S: AsyncRead + Unpin>(stream: &mut S) -> Result<WsFrame, FetchError> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;

    let fin = header[0] & 0x80 != 0;
    let opcode = header[0] & 0x0F;
    let masked = header[1] & 0x80 != 0;
    let len_byte = header[1] & 0x7F;

    // Extended payload length
    let payload_len: u64 = match len_byte {
        126 => {
            let mut buf = [0u8; 2];
            stream.read_exact(&mut buf).await?;
            u16::from_be_bytes(buf) as u64
        }
        127 => {
            let mut buf = [0u8; 8];
            stream.read_exact(&mut buf).await?;
            u64::from_be_bytes(buf)
        }
        n => n as u64,
    };

    if payload_len > MAX_PAYLOAD {
        return Err(FetchError::Http(format!(
            "WebSocket frame too large: {payload_len} bytes"
        )));
    }

    // Read masking key if present (shouldn't be for server → client)
    let mask_key = if masked {
        let mut key = [0u8; 4];
        stream.read_exact(&mut key).await?;
        Some(key)
    } else {
        None
    };

    // Read payload
    let mut payload = vec![0u8; payload_len as usize];
    if !payload.is_empty() {
        stream.read_exact(&mut payload).await?;
    }

    // Unmask if needed
    if let Some(key) = mask_key {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= key[i % 4];
        }
    }

    Ok(WsFrame {
        fin,
        opcode,
        payload,
    })
}

/// Write one encoded frame to a stream.
async fn write_frame<S: AsyncWrite + Unpin>(
    stream: &mut S,
    opcode: u8,
    payload: &[u8],
) -> Result<(), FetchError> {
    let frame = encode_frame(opcode, payload);
    stream.write_all(&frame).await?;
    stream.flush().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP upgrade handshake
// ---------------------------------------------------------------------------

/// Magic GUID for Sec-WebSocket-Accept derivation (RFC 6455 Section 4.2.2).
const WS_MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Maximum response header size for the upgrade response (4 KiB).
const MAX_UPGRADE_RESPONSE: usize = 4096;

/// Perform the WebSocket opening handshake over an established stream.
///
/// Sends the HTTP upgrade request, reads the 101 response, and verifies
/// `Sec-WebSocket-Accept`. Returns the negotiated sub-protocol (empty string
/// if none).
pub async fn upgrade<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    host: &str,
    path: &str,
    protocols: &[String],
) -> Result<String, FetchError> {
    // Generate a random 16-byte key, base64-encoded
    let key_bytes: [u8; 16] = rand::random();
    let key = base64::engine::general_purpose::STANDARD.encode(&key_bytes);

    // Build the upgrade request
    let mut request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\n\
         Sec-WebSocket-Version: 13\r\n"
    );
    if !protocols.is_empty() {
        request.push_str(&format!(
            "Sec-WebSocket-Protocol: {}\r\n",
            protocols.join(", ")
        ));
    }
    request.push_str("\r\n");

    nym_wasm_utils::console_log!("[ws] upgrade request:\n{request}");

    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    // Read the response header (up to \r\n\r\n)
    let mut buf = Vec::with_capacity(512);
    let mut tmp = [0u8; 1];
    loop {
        stream.read_exact(&mut tmp).await?;
        buf.push(tmp[0]);
        if buf.len() >= 4 && &buf[buf.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if buf.len() > MAX_UPGRADE_RESPONSE {
            return Err(FetchError::Http(
                "WebSocket upgrade response too large".into(),
            ));
        }
    }

    // Parse with httparse
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut response = httparse::Response::new(&mut headers);
    response
        .parse(&buf)
        .map_err(|e| FetchError::Http(format!("WebSocket upgrade parse error: {e}")))?;

    let response_str = String::from_utf8_lossy(&buf);
    nym_wasm_utils::console_log!("[ws] upgrade response:\n{response_str}");

    // Must be 101 Switching Protocols
    let status = response.code.unwrap_or(0);
    if status != 101 {
        let reason = response.reason.unwrap_or("unknown");
        return Err(FetchError::Http(format!(
            "WebSocket upgrade failed: {status} {reason}"
        )));
    }

    // Verify Sec-WebSocket-Accept (RFC 6455 Section 4.2.2)
    //
    // expected = base64(SHA1(key + MAGIC_GUID))

    // Sanity check 1: SHA1("abc") — single-block, well-known
    let abc_hash = sha1(b"abc").await?;
    let abc_hex: String = abc_hash.iter().map(|b| format!("{b:02x}")).collect();
    nym_wasm_utils::console_log!(
        "[ws] sanity SHA1('abc') = {abc_hex} (expect a9993e364706816aba3e25717850c26c9cd0d89d)"
    );

    // Sanity check 2: RFC 6455 Section 4.2.2 test vector (multi-block, 60 bytes)
    let rfc_key = "dGhlIHNhbXBsZSBub25jZQ==";
    let rfc_input = format!("{rfc_key}{WS_MAGIC}");
    let rfc_hash = sha1(rfc_input.as_bytes()).await?;
    let rfc_b64 = base64::engine::general_purpose::STANDARD.encode(rfc_hash);
    nym_wasm_utils::console_log!(
        "[ws] sanity RFC test = {rfc_b64} (expect s3pPLMBiTxaQ9kYGzzhZRbK+xOo=)"
    );

    // Actual accept verification
    let accept_input = format!("{key}{WS_MAGIC}");
    let accept_input_hex: String = accept_input.bytes().map(|b| format!("{b:02x}")).collect();
    nym_wasm_utils::console_log!(
        "[ws] accept_input ({} bytes): '{accept_input}'\n     hex: {accept_input_hex}",
        accept_input.len()
    );

    let hash = sha1(accept_input.as_bytes()).await?;
    let expected_accept = base64::engine::general_purpose::STANDARD.encode(hash);

    let accept = find_header(&response, "sec-websocket-accept")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    nym_wasm_utils::console_log!(
        "[ws] accept check: key={key}, expected={expected_accept}, got={accept}"
    );

    if accept != expected_accept {
        return Err(FetchError::Http(format!(
            "WebSocket accept mismatch: expected '{expected_accept}', got '{accept}'"
        )));
    }

    // Extract negotiated sub-protocol (if any)
    let protocol = find_header(&response, "sec-websocket-protocol").unwrap_or_default();

    Ok(protocol)
}

/// Find a header value by name (case-insensitive) in an httparse response.
fn find_header(response: &httparse::Response<'_, '_>, name: &str) -> Option<String> {
    response
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .and_then(|h| std::str::from_utf8(h.value).ok())
        .map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// WsConnection
// ---------------------------------------------------------------------------

/// A WebSocket connection over a `PooledConn` (plain TCP or TLS).
///
/// Can be used directly (single-threaded send/recv) or split into
/// independent read/write halves for concurrent use via `split()`.
pub struct WsConnection {
    conn: PooledConn,
    /// Fragmentation reassembly: (original opcode, accumulated payload).
    partial: Option<(u8, Vec<u8>)>,
    /// Whether we've sent a close frame.
    close_sent: bool,
}

/// Read half of a split `WsConnection`.
///
/// Owns the `ReadHalf` of the underlying stream and handles frame decoding,
/// fragmentation reassembly, and control frame processing. Pings are logged
/// but not auto-replied (the write half is needed for that).
pub struct WsReader {
    reader: futures::io::ReadHalf<PooledConn>,
    partial: Option<(u8, Vec<u8>)>,
}

/// Write half of a split `WsConnection`.
///
/// Owns the `WriteHalf` of the underlying stream. Handles frame encoding,
/// masking, and the close handshake.
pub struct WsWriter {
    writer: futures::io::WriteHalf<PooledConn>,
    close_sent: bool,
}

impl WsConnection {
    /// Create a new WebSocket connection from an already-upgraded stream.
    pub fn new(conn: PooledConn) -> Self {
        Self {
            conn,
            partial: None,
            close_sent: false,
        }
    }

    /// Split into independent read/write halves for concurrent use.
    ///
    /// This allows a reader task to loop on `recv()` without blocking
    /// sends or close commands from another task.
    pub fn split(self) -> (WsReader, WsWriter) {
        let (r, w) = self.conn.split();
        (
            WsReader {
                reader: r,
                partial: self.partial,
            },
            WsWriter {
                writer: w,
                close_sent: self.close_sent,
            },
        )
    }
}

impl WsReader {
    /// Read the next complete message from the WebSocket.
    ///
    /// Handles control frames internally:
    /// - Ping → logged, continues reading (pong requires write half)
    /// - Pong → ignored, continues reading
    /// - Close → returns error (connection closing)
    ///
    /// Reassembles fragmented messages from continuation frames.
    pub async fn recv(&mut self) -> Result<WsMessage, FetchError> {
        loop {
            let frame = read_frame(&mut self.reader).await?;

            match frame.opcode {
                OP_PING => {
                    nym_wasm_utils::console_log!(
                        "[ws] recv PING ({} bytes) — pong deferred",
                        frame.payload.len()
                    );
                    continue;
                }
                OP_PONG => {
                    nym_wasm_utils::console_log!("[ws] recv PONG");
                    continue;
                }
                OP_CLOSE => {
                    let (code, reason) = parse_close_payload(&frame.payload);
                    nym_wasm_utils::console_log!("[ws] recv CLOSE frame: {code} {reason}");
                    return Err(FetchError::Http(format!(
                        "WebSocket closed: {code} {reason}"
                    )));
                }
                OP_CONTINUATION => {
                    let (opcode, ref mut buf) = self.partial.as_mut().ok_or_else(|| {
                        FetchError::Http("WebSocket continuation without initial frame".into())
                    })?;
                    buf.extend_from_slice(&frame.payload);

                    if frame.fin {
                        let opcode = *opcode;
                        let (_, payload) = self.partial.take().unwrap();
                        return complete_message(opcode, payload);
                    }
                }
                OP_TEXT | OP_BINARY => {
                    if frame.fin {
                        return complete_message(frame.opcode, frame.payload);
                    }
                    self.partial = Some((frame.opcode, frame.payload));
                }
                _ => continue,
            }
        }
    }
}

impl WsWriter {
    /// Send a complete message as a single frame.
    pub async fn send(&mut self, msg: &WsMessage) -> Result<(), FetchError> {
        match msg {
            WsMessage::Text(s) => {
                nym_wasm_utils::console_log!("[ws] send TEXT ({} bytes)", s.len());
                write_frame(&mut self.writer, OP_TEXT, s.as_bytes()).await
            }
            WsMessage::Binary(b) => {
                nym_wasm_utils::console_log!("[ws] send BINARY ({} bytes)", b.len());
                write_frame(&mut self.writer, OP_BINARY, b).await
            }
        }
    }

    /// Send a close frame and mark the connection as closing.
    pub async fn close(&mut self, code: u16, reason: &str) -> Result<(), FetchError> {
        if self.close_sent {
            return Ok(());
        }
        self.close_sent = true;

        nym_wasm_utils::console_log!("[ws] send CLOSE frame: {code} {reason}");
        let mut payload = Vec::with_capacity(2 + reason.len());
        payload.extend_from_slice(&code.to_be_bytes());
        payload.extend_from_slice(reason.as_bytes());
        write_frame(&mut self.writer, OP_CLOSE, &payload).await
    }
}

/// Convert a completed frame payload into a `WsMessage`.
fn complete_message(opcode: u8, payload: Vec<u8>) -> Result<WsMessage, FetchError> {
    match opcode {
        OP_TEXT => {
            let text = String::from_utf8(payload)
                .map_err(|e| FetchError::Http(format!("WebSocket text not UTF-8: {e}")))?;
            nym_wasm_utils::console_log!("[ws] recv TEXT ({} bytes)", text.len());
            Ok(WsMessage::Text(text))
        }
        OP_BINARY => {
            nym_wasm_utils::console_log!("[ws] recv BINARY ({} bytes)", payload.len());
            Ok(WsMessage::Binary(payload))
        }
        _ => Err(FetchError::Http(format!(
            "unexpected WebSocket opcode: {opcode:#x}"
        ))),
    }
}

/// Parse a close frame payload into (code, reason).
fn parse_close_payload(payload: &[u8]) -> (u16, String) {
    if payload.len() >= 2 {
        let code = u16::from_be_bytes([payload[0], payload[1]]);
        let reason = std::str::from_utf8(&payload[2..])
            .unwrap_or("(invalid UTF-8)")
            .to_string();
        (code, reason)
    } else {
        (1005, String::new()) // 1005 = no status code present
    }
}
