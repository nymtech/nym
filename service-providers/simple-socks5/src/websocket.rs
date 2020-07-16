use tokio_tungstenite::connect_async;

pub struct Connection {
    uri: String,
    // ws_stream:
}

impl Connection {
    pub fn new(uri: &str) -> Connection {
        Connection {
            uri: String::from(uri),
        }
    }

    pub async fn connect(&self) -> Result<(), WebsocketConnectionError> {
        println!("* connecting to local websocket server at {}", self.uri);
        // let ws_stream = runtime.spawn(connect_async()).await;

        let ws_stream = match connect_async(&self.uri).await {
            Ok((ws_stream, _)) => ws_stream,
            Err(_e) => return Err(WebsocketConnectionError::ConnectionNotEstablished),
        };
        Ok(())
    }
}

pub enum WebsocketConnectionError {
    ConnectionNotEstablished,
}
