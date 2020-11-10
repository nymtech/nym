use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, stream::Stream};

pub(crate) struct MetricsWebsocket {
    metrics_address: String,
    metrics_upstream: WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>,
}

impl MetricsWebsocket {
    pub async fn connect(metrics_address: &str) -> Result<MetricsWebsocket, WebsocketError> {
        let ws_stream = match connect_async(metrics_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(e) => return Err(WebsocketError::NetworkError(e)),
        };

        Ok(MetricsWebsocket {
            metrics_address: metrics_address.to_string(),
            metrics_upstream: ws_stream,
        })
    }

    async fn on_message() {}
}

#[derive(Debug)]
pub enum WebsocketError {
    NetworkError(WsError),
}
