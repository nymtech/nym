use crate::websocket;
use tokio::runtime::Runtime;

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

    pub fn start(&self) {
        self.connect_websocket("ws://localhost:1977");
    }

    fn connect_websocket(&self, uri: &str) {
        let ws = websocket::Connection::new(uri); // any use of `new` is suspicious. Should this live up in main and then get wired in as a trait? might be nice to test that way...
        let ws_stream = ws.connect();
    }
}
