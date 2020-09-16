// use futures::stream::SplitSink;
// use futures::{SinkExt, StreamExt};
use directory_client::{Client, DirectoryClient};
use log::info;

mod websocket;

pub struct Monitor {
    directory_uri: String,
    websocket_uri: String,
}

impl Monitor {
    pub fn new(directory_uri: &str, websocket_uri: &str) -> Monitor {
        Monitor {
            directory_uri: directory_uri.to_string(),
            websocket_uri: websocket_uri.to_string(),
        }
    }

    pub async fn run(self) {
        let connection = websocket::Connection::new(&self.websocket_uri).await;
        let me = connection.get_self_address().await;
        info!("Retrieved self address:  {:?}", me.to_string());

        let config = directory_client::Config::new(self.directory_uri);
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
        let network_monitor = Monitor::new("https://directory.nymtech.net", "ws://localhost:1977");
        assert_eq!(
            "https://directory.nymtech.net",
            network_monitor.directory_uri
        );
        assert_eq!("ws://localhost:1977", network_monitor.websocket_uri);
    }
}
