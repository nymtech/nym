use crate::{controller::Controller, websocket};
use futures::SinkExt;
use futures_util::StreamExt;
use nymsphinx::params::MessageType;
use nymsphinx::receiver::ReconstructedMessage;
use simple_socks5_requests::Request;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;

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

                let reconstructed_message = ReconstructedMessage::try_from_bytes(&data).expect("todo: error handling");
                let raw_message = reconstructed_message.message;

                // if raw_message[0] == b'{' && raw_message[1] == b'"' {
                //     println!("json: {:?}", String::from_utf8_lossy(&raw_message));
                //     continue;
                // }
                let request = Request::try_from_bytes(&raw_message).unwrap();
                let response = controller.process_request(request).await.unwrap();
                if response.is_none() { // restart the loop if we got nothing back
                    continue;
                }

                // TODO: wire SURBs in here once they're available
                let return_address = "7tVXwePpo6SM99sqM1xEp6S4T1TSpxYx97fTpEdvmF7i.GgrN8998SmwvQghNEvqtPPZCgMQqJovWBrzspMnBESsE@e3vUAo6YhB7zq3GH8B4k3iiGT4H2USjdd5ZMZoUsHdF";
                let recipient = nymsphinx::addressing::clients::Recipient::try_from_string(return_address).unwrap();

                let response_message = std::iter::once(MessageType::WithoutReplySURB as u8)
                    .chain(recipient.into_bytes().iter().cloned())
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
