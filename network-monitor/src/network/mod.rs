// use futures::stream::SplitSink;
// use futures::{SinkExt, StreamExt};
use directory_client::{Client, DirectoryClient};
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
        let connection = websocket::Connection::new(&self.websocket_uri).await;
        let me = connection.get_self_address().await;
        info!("Retrieved self address:  {:?}", me.to_string());

        let config = directory_client::Config::new("https://directory.nymtech.net".to_string());
        let directory: Client = DirectoryClient::new(config);
        let topology = directory.get_topology().await;
        info!("Topology is: {:?}", topology);
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
