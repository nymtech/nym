use crate::{proxy, websocket};
use futures::SinkExt;
use futures_util::StreamExt;
use proxy::request::Request;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use websocket::WebsocketConnectionError;
pub struct Server {
    runtime: Runtime,
}

impl Server {
    pub fn new() -> Server {
        let runtime = Runtime::new().unwrap();
        Server { runtime }
    }

    /// Start all subsystems
    pub fn start(&mut self) {
        let websocket_stream = self.connect_websocket("ws://localhost:1977");
        let (mut write, mut read) = websocket_stream.split();
        self.runtime.block_on(async {
            println!("\nAll systems go. Press CTRL-C to stop the server.");
            while let Some(msg) = read.next().await {
                let data = msg.unwrap().into_data();
                if data[0] == b'{' && data[1] == b'"' {
                    println!("json: {:?}", String::from_utf8_lossy(&data));
                    continue
                }

                println!("Socks5 requester received a new request message: {:?}", String::from_utf8_lossy(&data));
                let request = Request::new(data);
                let response = request.run().await.unwrap();
                let temp_socks5_recipient_address = "4QC5D8auMbVpFVBfiZnVtQVUPiNUV9FMnpb81cauFpEp@GYCqU48ndXke9o2434i7zEGv1sWg1cNVswWJfRnY1VTB";
                let foo = nymsphinx::addressing::clients::Recipient::try_from_string(temp_socks5_recipient_address).unwrap();

                let foomp =  foo.into_bytes()
                .iter()
                .cloned()
                .chain(response.serialize().into_iter())
                .collect();

                let message = Message::Binary(foomp);
                write.send(message).await.unwrap();
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

    /// TODO: use of `new` is suspicious. Should this live up in start and then get wired in as a trait? might be nice to test that way...
    /// TODO: later on, once we have all this shit connected up, try and *use*
    /// the websocket::Connection instead of the stream directly, to wire things
    /// together. I have a feeling this might make testing substantially easier
    /// so that we can have small testable units of logic rather than a bunch
    /// of un-testable hose pipes wired together (as in most Rust example
    /// code that's available on the internet).
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
