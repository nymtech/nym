mod network;

#[tokio::main]
async fn main() {
    let websocket_uri = "ws://localhost:1977";
    let directory_uri = "https://directory.nymtech.net";
    println!("Starting network monitor:");
    let network_monitor = network::Monitor::new(directory_uri, websocket_uri);
    network_monitor.run().await;
}
