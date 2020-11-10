use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, stream::Stream};

pub async fn subscribe() {
    println!("Starting websocket...");
    let metrics_socket = "wss://qa-metrics.nymtech.net/ws";
    println!(
        "connnecting to upstream metrics websocket at {}",
        metrics_socket
    );
    match MetricsWebsocket::connect(metrics_socket).await {
        Ok(_) => println!("metrics websocket connected successfully"),
        Err(e) => println!("metrics websocket failed to connect: {:?}", e),
    };
}

struct MetricsWebsocket {
    metrics_address: String,
    metrics_upstream: WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>,
}

impl MetricsWebsocket {
    async fn connect(metrics_address: &str) -> Result<MetricsWebsocket, super::WebsocketError> {
        let ws_stream = match connect_async(metrics_address).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(e) => return Err(super::WebsocketError::NetworkError(e)),
        };

        Ok(MetricsWebsocket {
            metrics_address: metrics_address.to_string(),
            metrics_upstream: ws_stream,
        })
    }

    async fn on_message() {}
}
