mod network;

#[tokio::main]
async fn main() {
    let uri = "ws://localhost:1977";
    println!("Starting network monitor:");
    let network_monitor = network::Monitor::new(uri.into());
    network_monitor.run().await;
}
