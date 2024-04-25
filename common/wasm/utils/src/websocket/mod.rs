// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::console_log;
use futures::{Sink, Stream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use gloo_utils::errors::JsError;
use std::fmt::{self, Formatter};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tungstenite::{Error as WsError, Message as WsMessage}; // use tungstenite Message and Error types for easier compatibility with `ClientHandshake`

// Unfortunately this can't be cleanly done with TryFrom/TryInto traits as both are foreign types
fn into_tungstenite_message(msg: gloo_net::websocket::Message) -> WsMessage {
    match msg {
        Message::Text(text) => WsMessage::Text(text),
        Message::Bytes(bytes) => WsMessage::Binary(bytes),
    }
}

fn map_ws_error(err: WebSocketError) -> WsError {
    match err {
        // TODO: are we preserving correct semantics?
        WebSocketError::ConnectionError => WsError::ConnectionClosed,
        WebSocketError::ConnectionClose(_event) => WsError::ConnectionClosed,
        WebSocketError::MessageSendError(err) => {
            WsError::Io(io::Error::new(io::ErrorKind::Other, err.to_string()))
        }
        _ => WsError::Io(io::Error::new(io::ErrorKind::Other, "new websocket error")),
    }
}

fn try_from_tungstenite_message(msg: WsMessage) -> Result<gloo_net::websocket::Message, WsError> {
    match msg {
        WsMessage::Text(text) => Ok(gloo_net::websocket::Message::Text(text)),
        WsMessage::Binary(bytes) => Ok(gloo_net::websocket::Message::Bytes(bytes)),
        _ => Err(WsError::Io(io::Error::from(io::ErrorKind::InvalidInput))),
    }
}

// Safety: when compiled to wasm32 everything is going to be running on a single thread and so there
// is no shared memory right now.
//
// ************************************
// SUPER IMPORTANT TODO: ONCE WASM IN RUST MATURES AND BECOMES MULTI-THREADED THIS MIGHT
// LEAD TO RUNTIME MEMORY CORRUPTION!!
// ************************************
//
// note: https://github.com/rustwasm/gloo/issues/109
unsafe impl Send for JSWebsocket {}

#[allow(clippy::upper_case_acronyms)]
pub struct JSWebsocket {
    inner: WebSocket,
}

impl JSWebsocket {
    pub fn new(url: &str) -> Result<Self, JsError> {
        let inner = WebSocket::open(url)?;
        console_log!("Websocket to {:?} is now open!", url);
        Ok(JSWebsocket { inner })
    }

    pub async fn close(self, code: Option<u16>, reason: Option<&str>) -> Result<(), JsError> {
        self.inner.close(code, reason)
    }
}

impl Stream for JSWebsocket {
    type Item = Result<WsMessage, WsError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx).map(|maybe_item| {
            maybe_item.map(|item| item.map(into_tungstenite_message).map_err(map_ws_error))
        })
    }
}

impl fmt::Debug for JSWebsocket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "JSWebSocket")
    }
}

impl Sink<WsMessage> for JSWebsocket {
    type Error = WsError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner)
            .poll_ready(cx)
            .map(|ready_res| ready_res.map_err(map_ws_error))
    }

    fn start_send(mut self: Pin<&mut Self>, item: WsMessage) -> Result<(), Self::Error> {
        let item = try_from_tungstenite_message(item)?;

        Pin::new(&mut self.inner)
            .start_send(item)
            .map_err(map_ws_error)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner)
            .poll_flush(cx)
            .map(|flush_res| flush_res.map_err(map_ws_error))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.inner)
            .poll_close(cx)
            .map(|close_res| close_res.map_err(map_ws_error))
    }
}
