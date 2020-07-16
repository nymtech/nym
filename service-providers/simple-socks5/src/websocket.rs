use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, tungstenite::client::AutoStream};

pub struct Connection {
    uri: String,
    // ws_stream:
}

impl Connection {
    pub fn new(uri: &str) -> Connection {
        println!("* setting up websocket connection");
        Connection {
            uri: String::from(uri),
        }
    }

    pub async fn connect(&self) -> Result<WebSocketStream<TcpStream>, WebsocketConnectionError> {
        println!("* connecting to local websocket server at {}", self.uri);
        // let ws_stream = runtime.spawn(connect_async()).await;

        let ws_stream = match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => return Ok(ws_stream),
            Err(_e) => return Err(WebsocketConnectionError::ConnectionNotEstablished),
        };
    }
}

#[derive(Debug)]
pub enum WebsocketConnectionError {
    ConnectionNotEstablished,
}
