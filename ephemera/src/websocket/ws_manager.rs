use std::net::SocketAddr;

use anyhow::Result;
use futures_util::SinkExt;
use log::{debug, error, info};
use tokio::sync::broadcast::error::RecvError;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::api::types::ApiBlock;
use crate::block::types::block::Block;

pub struct WsConnection {
    socket: WebSocketStream<TcpStream>,
    pending_messages_rx: broadcast::Receiver<Message>,
    address: SocketAddr,
}

impl WsConnection {
    pub fn new(
        socket: WebSocketStream<TcpStream>,
        pending_messages_rx: broadcast::Receiver<Message>,
        address: SocketAddr,
    ) -> WsConnection {
        WsConnection {
            socket,
            pending_messages_rx,
            address,
        }
    }

    pub async fn accept_messages(mut self) {
        loop {
            match self.pending_messages_rx.recv().await {
                Ok(msg) => {
                    debug!("Sending message to {}", self.address);
                    if let Err(err) = self.socket.send(msg).await {
                        error!("Error sending message to websocket client: {:?}", err);
                    }
                }
                Err(e) => {
                    if RecvError::Closed == e {
                        error!("Error while receiving message from broadcast channel: {:?}, closing connection", e);
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct WsMessageBroadcaster {
    pub(crate) pending_messages_tx: broadcast::Sender<Message>,
}

impl WsMessageBroadcaster {
    pub(crate) fn new(pending_messages_tx: broadcast::Sender<Message>) -> WsMessageBroadcaster {
        WsMessageBroadcaster {
            pending_messages_tx,
        }
    }

    pub(crate) fn send_block(&self, block: &Block) -> Result<()> {
        debug!("Sending block {} to websocket clients", block.header.hash);
        let json = serde_json::to_string::<ApiBlock>(block.into())?;
        let msg = Message::Text(json);
        self.pending_messages_tx.send(msg)?;
        Ok(())
    }
}

pub(crate) struct WsManager {
    pub(crate) listener: Option<TcpListener>,
    pub(crate) ws_address: String,
    pub(crate) pending_messages_tx: broadcast::Sender<Message>,
    _pending_messages_rcv: broadcast::Receiver<Message>,
}

impl WsManager {
    #[allow(clippy::used_underscore_binding)]
    pub(crate) fn new(address: String) -> (WsManager, WsMessageBroadcaster) {
        let (pending_messages_tx, _pending_messages_rcv) = broadcast::channel(1000);
        let ws_message_broadcast = WsMessageBroadcaster::new(pending_messages_tx.clone());
        let manager = WsManager {
            listener: None,
            ws_address: address,
            pending_messages_tx,
            _pending_messages_rcv,
        };
        (manager, ws_message_broadcast)
    }

    pub(crate) async fn listen(&mut self) -> Result<()> {
        let listener = TcpListener::bind(&self.ws_address).await?;
        info!("Listening for websocket connections on {}", self.ws_address);
        self.listener = Some(listener);
        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        let listener = self.listener.take().expect("Listener not set");
        loop {
            tokio::select! {
                res = listener.accept() => {
                    match res {
                        Ok((stream, addr)) => {
                            debug!("Accepted websocket connection from: {}", addr);
                            self.handle_connection(stream, addr);
                        }
                        Err(err) => {
                            return Err(err.into());
                        }
                    }
                }
            }
        }
    }

    pub fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) {
        let pending_messages_rx = self.pending_messages_tx.subscribe();
        tokio::spawn(async move {
            match tokio_tungstenite::accept_async(stream).await {
                Ok(ws_stream) => {
                    let connection = WsConnection::new(ws_stream, pending_messages_rx, addr);
                    connection.accept_messages().await;
                }
                Err(err) => {
                    error!("Error accepting websocket connection: {:?}", err);
                }
            }
            debug!("Websocket connection closed");
        });
    }
}
