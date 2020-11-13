use log::*;
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio::sync::broadcast;
use tokio_native_tls::TlsStream;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, stream::Stream};

pub(crate) type WsItem = Result<Message, WsError>;

/// A websocket client which subscribes to the metrics centrally collected by the metrics server.
/// All metrics messages get copied out to this dashboard instance's clients.
pub(crate) struct MetricsWebsocketClient {
    metrics_upstream: WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>,
    broadcaster: broadcast::Sender<Message>,
}

impl MetricsWebsocketClient {
    /// Connect to the upstream metrics server
    pub(crate) async fn connect(
        metrics_address: &str,
        broadcaster: broadcast::Sender<Message>,
    ) -> Result<MetricsWebsocketClient, WebsocketError> {
        let ws_stream = match connect_async(metrics_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(e) => return Err(WebsocketError::NetworkError(e)),
        };

        info!("Subscribed to metrics websocket at {}", metrics_address);

        Ok(MetricsWebsocketClient {
            metrics_upstream: ws_stream,
            broadcaster,
        })
    }

    /// When the metrics server sends a message, it should be copied out to the server and distributed
    /// to all connected clients.
    fn on_message(&self, item: WsItem) {
        let ws_message = match item {
            Ok(message) => message,
            Err(err) => {
                error!("failed to obtain valid websocket message - {}", err);
                return;
            }
        };

        match self.broadcaster.send(ws_message) {
            Ok(received) => info!("broadcasted websocket metrics data to {} clients", received),
            Err(_) => info!("no clients are currently subscribed"),
        }
    }

    pub(crate) async fn run(&mut self) {
        while let Some(incoming) = self.metrics_upstream.next().await {
            self.on_message(incoming)
        }
        info!("Our metrics server subscriber is finished!")
    }
}

#[derive(Debug)]
pub enum WebsocketError {
    NetworkError(WsError),
}
