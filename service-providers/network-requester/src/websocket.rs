// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream};

#[allow(clippy::upper_case_acronyms)]
pub(crate) type TSWebsocketStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Connection {
    uri: String,
}

impl Connection {
    pub fn new(uri: &str) -> Connection {
        Connection {
            uri: String::from(uri),
        }
    }

    pub async fn connect(&self) -> Result<TSWebsocketStream, WebsocketConnectionError> {
        match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => Ok(ws_stream),
            Err(_e) => Err(WebsocketConnectionError::ConnectionNotEstablished),
        }
    }
}

#[derive(Debug)]
pub enum WebsocketConnectionError {
    ConnectionNotEstablished,
}
