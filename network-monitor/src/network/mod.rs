// use futures::stream::SplitSink;
// use futures::{SinkExt, StreamExt};
use log::info;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;

mod websocket;

pub struct Monitor {
    websocket_uri: String,
}

impl Monitor {
    pub fn new(websocket_uri: &str) -> Monitor {
        let ws = websocket_uri.to_string();
        Monitor { websocket_uri: ws }
    }

    pub async fn run(self) {
        let mut websocket_stream = self.connect_websocket(&self.websocket_uri).await;
        let me = websocket::get_self_address(&mut websocket_stream).await;
        println!("Retrieved self address:  {:?}", me);

        // split the websocket so that we could read and write from separate threads
        // let (websocket_writer, mut websocket_reader) = websocket_stream.split();
    }

    // Make the websocket connection so we can receive incoming Mixnet messages.
    async fn connect_websocket(&self, uri: &str) -> WebSocketStream<TcpStream> {
        let ws_stream = match websocket::Connection::new(uri).connect().await {
            Ok(ws_stream) => {
                info!("* connected to local websocket server at {}", uri);
                ws_stream
            }
            Err(WebsocketConnectionError::ConnectionNotEstablished) => {
                panic!("Error: websocket connection attempt failed, is the Nym client running?")
            }
        };
        return ws_stream;
    }
}

#[cfg(test)]
mod constructing {
    use super::*;

    #[test]
    fn works() {
        let network_monitor = Monitor::new("ws://localhost:1977");
        assert_eq!("ws://localhost:1977", network_monitor.websocket_uri);
    }
}
