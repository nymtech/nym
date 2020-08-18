use crate::{controller::Controller, websocket};
use futures::channel::mpsc;
use futures::SinkExt;
use futures_util::StreamExt;
use nymsphinx::addressing::clients::Recipient;
use simple_socks5_requests::{Request, Response};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;
use websocket_requests::{requests::ClientRequest, responses::ServerResponse};

pub struct ServiceProvider {}

impl ServiceProvider {
    pub fn new() -> ServiceProvider {
        ServiceProvider {}
    }

    /// Start all subsystems
    pub async fn run(&mut self) {
        let websocket_stream = self.connect_websocket("ws://localhost:1977").await;
        let (mut websocket_writer, mut websocket_reader) = websocket_stream.split();
        let controller = Controller::new();

        let (sender, mut reader) = mpsc::unbounded::<Response>();

        let response_reader_future = async move {
            // TODO: wire SURBs in here once they're available
            let return_address = "7tVXwePpo6SM99sqM1xEp6S4T1TSpxYx97fTpEdvmF7i.GgrN8998SmwvQghNEvqtPPZCgMQqJovWBrzspMnBESsE@e3vUAo6YhB7zq3GH8B4k3iiGT4H2USjdd5ZMZoUsHdF";
            let recipient = Recipient::try_from_base58_string(return_address).unwrap();
            while let Some(response) = reader.next().await {
                // make 'request' to native-websocket client
                let response_message = ClientRequest::Send {
                    recipient: recipient.clone(),
                    message: response.into_bytes(),
                    with_reply_surb: false,
                };

                let message = Message::Binary(response_message.serialize());
                websocket_writer.send(message).await.unwrap();
            }
        };
        tokio::spawn(response_reader_future);

        println!("\nAll systems go. Press CTRL-C to stop the server.");
        while let Some(msg) = websocket_reader.next().await {
            let data = msg.unwrap().into_data();
            let received = match ServerResponse::deserialize(&data).expect("todo: error handling") {
                ServerResponse::Received(received) => received,
                ServerResponse::Error(err) => {
                    panic!("received error from native client! - {}", err)
                }
                _ => unimplemented!("probably should never be reached?"),
            };

            let raw_message = received.message;
            let request = Request::try_from_bytes(&raw_message).unwrap();

            let mut controller_local_pointer = controller.clone();
            let response_sender_clone = sender.clone();
            tokio::spawn(async move {
                if let Ok(response_option) = controller_local_pointer.process_request(request).await
                {
                    if let Some(response) = response_option {
                        // if we have an actual response - send it through the mixnet!
                        response_sender_clone
                            .unbounded_send(response)
                            .expect("channel got closed?");
                    }
                };
            });
        }
    }

    // /// Keep running until the user hits CTRL-C.
    // pub fn run_forever(&mut self) {
    //     if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
    //         println!("Stopping with error: {:?}", e);
    //     }
    //     println!("\nStopping...");
    // }

    // Make the websocket connection so we can receive incoming Mixnet messages.
    async fn connect_websocket(&mut self, uri: &str) -> WebSocketStream<TcpStream> {
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
    }
}
