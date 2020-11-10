use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, stream::Stream};

/// A websocket client which subscribes to the metrics centrally collected by the metrics server.
/// All metrics messages get copied out to this dashboard instance's clients.
pub(crate) struct MetricsWebsocketClient {
    metrics_address: String,
    metrics_upstream: WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>,
}

impl MetricsWebsocketClient {
    /// Connect to the upstream metrics server
    pub async fn connect(metrics_address: &str) -> Result<MetricsWebsocketClient, WebsocketError> {
        let ws_stream = match connect_async(metrics_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(e) => return Err(WebsocketError::NetworkError(e)),
        };

        println!("Subscribed to metrics websocket at {}", metrics_address);

        Ok(MetricsWebsocketClient {
            metrics_address: metrics_address.to_string(),
            metrics_upstream: ws_stream,
        })
    }

    /// When the metrics server sends a message, it should be copied out to the server and distributed
    /// to all connected clients.
    async fn on_message() {}
}

#[derive(Debug)]
pub enum WebsocketError {
    NetworkError(WsError),
}
