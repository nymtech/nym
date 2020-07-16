use crate::websocket;
use tokio::runtime::Runtime;
use websocket::WebsocketConnectionError;

pub struct Server {
    runtime: Runtime,
}

impl Server {
    pub fn new() -> Server {
        let runtime = Runtime::new().unwrap();
        Server { runtime }
    }

    /// Keep running until the user hits CTRL-C.
    pub fn run_forever(&mut self) {
        if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
            println!("Stopping with error: {:?}", e);
        }
        println!("\nStopping...");
    }

    pub fn start(&mut self) {
        self.connect_websocket("ws://localhost:1977");
        println!("\nAll systems go. Press CTRL-C to stop the server.");
    }

    /// any use of `new` is suspicious. Should this live up in main and then get wired in as a trait? might be nice to test that way...
    fn connect_websocket(&mut self, uri: &str) {
        self.runtime.block_on(async {
            let ws = websocket::Connection::new(uri);
            match ws.connect().await {
                Ok(_) => println!("* connected to local websocket server at {}", uri),
                Err(WebsocketConnectionError::ConnectionNotEstablished) => {
                    panic!("Error: websocket connection attempt failed, is the Nym client running?")
                }
            };
        })
    }
}
