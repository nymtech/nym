use futures_util::StreamExt;
use log::*;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream};

pub(crate) type WsItem = Result<Message, WsError>;
const MAX_RECONNECTION_ATTEMPTS: u32 = 10;
const RECONNECTION_BACKOFF: Duration = Duration::from_secs(10);

/// A websocket client which subscribes to the metrics centrally collected by the metrics server.
/// All metrics messages get copied out to this dashboard instance's clients.
pub(crate) struct MetricsWebsocketClient {
    metrics_address: String,
    // metrics_upstream: WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>,
    metrics_upstream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    broadcaster: broadcast::Sender<Message>,

    reconnection_attempt: u32,
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
            metrics_address: metrics_address.into(),
            metrics_upstream: ws_stream,
            broadcaster,
            reconnection_attempt: 0,
        })
    }

    async fn attempt_reconnection(&mut self) {
        info!("attempting reconnection to metrics websocket...");
        if self.reconnection_attempt >= MAX_RECONNECTION_ATTEMPTS {
            // kill the process and reset everything when service restarts
            error!("failed to re-establish websocket connection to metrics server");
            std::process::exit(1)
        }

        // use linear backoff to try to reconnect asap
        sleep(RECONNECTION_BACKOFF * self.reconnection_attempt).await;

        let ws_stream = match connect_async(&self.metrics_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(err) => {
                self.reconnection_attempt += 1;
                info!("reconnection failed... - {}", err);
                return;
            }
        };

        info!("reconnected!");

        self.reconnection_attempt = 0;
        self.metrics_upstream = ws_stream;
    }

    /// When the metrics server sends a message, it should be copied out to the server and distributed
    /// to all connected clients.
    fn on_message(&self, item: WsItem) -> Result<(), WsError> {
        let ws_message = match item {
            Ok(message) => message,
            Err(err) => {
                error!("failed to obtain valid websocket message - {}", err);
                return Err(err);
            }
        };

        match self.broadcaster.send(ws_message) {
            Ok(received) => debug!("broadcasted websocket metrics data to {} clients", received),
            Err(_) => debug!("no clients are currently subscribed"),
        }

        Ok(())
    }

    pub(crate) async fn run(&mut self) {
        loop {
            if let Some(incoming) = self.metrics_upstream.next().await {
                if let Err(_) = self.on_message(incoming) {
                    self.attempt_reconnection().await;
                }
            } else {
                self.attempt_reconnection().await;
            }
        }
    }
}

#[derive(Debug)]
pub enum WebsocketError {
    NetworkError(WsError),
}
