use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};
use tokio_native_tls::TlsStream;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, stream::Stream};

pub(crate) mod client;
mod server;

#[derive(Debug)]
enum WebsocketError {
    NetworkError(WsError),
}
