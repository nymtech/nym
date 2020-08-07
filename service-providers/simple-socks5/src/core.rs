use crate::{controller::Controller, websocket};
use futures::SinkExt;
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;
use simple_socks5_requests::Request;

pub struct ServiceProvider {
    runtime: Runtime,
}

impl ServiceProvider {
    pub fn new() -> ServiceProvider {
        let runtime = Runtime::new().unwrap();
        ServiceProvider { runtime }
    }

    /// Start all subsystems
    pub fn start(&mut self) {
        let websocket_stream = self.connect_websocket("ws://localhost:1977");
        let (mut websocket_writer, mut websocket_reader) = websocket_stream.split();
        let mut controller = Controller::new();

        self.runtime.block_on(async {
            println!("\nAll systems go. Press CTRL-C to stop the server.");
            while let Some(msg) = websocket_reader.next().await {
                let data = msg.unwrap().into_data();
                if data[0] == b'{' && data[1] == b'"' {
                    println!("json: {:?}", String::from_utf8_lossy(&data));
                    continue;
                }

                let request = Request::try_from_bytes(&data).unwrap();
                let response = controller.process_request(request).await.unwrap();
                if response.is_none() { // restart the loop if we got nothing back
                    continue;
                }
                
                // TODO: wire SURBs in here once they're available
                let return_address = "4QC5D8auMbVpFVBfiZnVtQVUPiNUV9FMnpb81cauFpEp@GYCqU48ndXke9o2434i7zEGv1sWg1cNVswWJfRnY1VTB";
                let recipient = nymsphinx::addressing::clients::Recipient::try_from_string(return_address).unwrap();

                let response_message = recipient.into_bytes()
                    .iter()
                    .cloned()
                    .chain(response.unwrap().into_bytes().into_iter())
                    .collect();

                let message = Message::Binary(response_message);
                websocket_writer.send(message).await.unwrap();
            }
        });
    }

    /// Keep running until the user hits CTRL-C.
    pub fn run_forever(&mut self) {
        if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
            println!("Stopping with error: {:?}", e);
        }
        println!("\nStopping...");
    }

    // Make the websocket connection so we can receive incoming Mixnet messages.
    fn connect_websocket(&mut self, uri: &str) -> WebSocketStream<TcpStream> {
        self.runtime.block_on(async {
            let ws_stream = match websocket::Connection::new(uri).connect().await {
                Ok(ws_stream) => {
                    println!("* connected to local websocket server at {}", uri);
                    ws_stream
                }
                Err(WebsocketConnectionError::ConnectionNotEstablished) => {
                    panic!("Error: websocket connection attempt failed, is the Nym client running?")
                }
            };
            return ws_stream;
        })
    }
}
