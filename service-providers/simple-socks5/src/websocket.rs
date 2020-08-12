use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::WebSocketStream;

pub struct Connection {
    uri: String,
}

impl Connection {
    pub fn new(uri: &str) -> Connection {
        Connection {
            uri: String::from(uri),
        }
    }

    pub async fn connect(&self) -> Result<WebSocketStream<TcpStream>, WebsocketConnectionError> {
        match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => return Ok(ws_stream),
            Err(_e) => return Err(WebsocketConnectionError::ConnectionNotEstablished),
        };
    }
}

#[derive(Debug)]
pub enum WebsocketConnectionError {
    ConnectionNotEstablished,
}
