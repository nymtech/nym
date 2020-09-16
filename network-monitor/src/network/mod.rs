// use futures::stream::SplitSink;
// use futures::{SinkExt, StreamExt};
use log::info;

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
        let mut connection = websocket::Connection::new(&self.websocket_uri);
        &connection.connect().await;
        let me = connection.get_self_address().await;
        // println!("Retrieved self address:  {:?}", me);

        // split the websocket so that we could read and write from separate threads
        // let (websocket_writer, mut websocket_reader) = websocket_stream.split();
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
