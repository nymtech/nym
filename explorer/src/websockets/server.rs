use futures_util::{SinkExt, StreamExt};
use log::*;
use std::{io::Error as IoError, net::SocketAddr};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

pub struct DashboardWebsocketServer {
    sender: broadcast::Sender<Message>,
    addr: String,
}

impl DashboardWebsocketServer {
    pub fn new(port: u16, sender: broadcast::Sender<Message>) -> DashboardWebsocketServer {
        let addr = format!("[::]:{}", port);
        DashboardWebsocketServer { sender, addr }
    }

    pub async fn start(self) -> Result<(), IoError> {
        let try_socket = TcpListener::bind(&self.addr).await;

        let listener = try_socket?;
        info!("starting to listen on {}", self.addr);
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(Self::handle_connection(
                stream,
                addr,
                self.sender.subscribe(),
            ));
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        receiver: broadcast::Receiver<Message>,
    ) {
        let mut ws_stream = match accept_async(stream).await {
            Ok(ws_stream) => ws_stream,
            Err(err) => {
                warn!(
                    "error while performing the websocket handshake with {} - {:?}",
                    addr, err
                );
                return;
            }
        };

        info!("client connected from {}", addr);
        let mut broadcast_stream = BroadcastStream::new(receiver);
        while let Some(message) = broadcast_stream.next().await {
            let message = message.expect("the websocket broadcaster is dead!");
            if let Err(err) = ws_stream.send(message).await {
                warn!(
                    "failed to send subscribed message back to client ({}) - {}",
                    addr, err
                );
                return;
            } else {
                info!("sent message to {}", addr)
            }
        }
    }
}
