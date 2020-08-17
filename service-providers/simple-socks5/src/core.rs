use crate::{controller::Controller, websocket};
use futures::SinkExt;
use futures_util::StreamExt;
use nymsphinx::addressing::clients::Recipient;
use simple_socks5_requests::Request;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};
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
                let received = match ServerResponse::deserialize(&data).expect("todo: error handling") {
                    ServerResponse::Received(received) => received,
                    ServerResponse::Error(err) => {
                        panic!("received error from native client! - {}", err)
                    },
                    _ => unimplemented!("probably should never be reached?")
                };

                let raw_message = received.message;
                let request = Request::try_from_bytes(&raw_message).unwrap();
                let response = match controller.process_request(request).await {
                    Ok(response_option) => match response_option {
                        None => continue, // restart the loop if we got nothing back
                        Some(response) => response,
                    },
                    Err(err) => {
                        eprintln!("just some error - {:?}", err);
                        continue
                    }
                };

                // TODO: wire SURBs in here once they're available
                let return_address = "7tVXwePpo6SM99sqM1xEp6S4T1TSpxYx97fTpEdvmF7i.GgrN8998SmwvQghNEvqtPPZCgMQqJovWBrzspMnBESsE@e3vUAo6YhB7zq3GH8B4k3iiGT4H2USjdd5ZMZoUsHdF";
                let recipient = Recipient::try_from_base58_string(return_address).unwrap();

                // make 'request' to native-websocket client
                let response_message = ClientRequest::Send {
                    recipient,
                    message: response.into_bytes(),
                    with_reply_surb: false
                };

                let message = Message::Binary(response_message.serialize());
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
