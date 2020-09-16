mod network;

#[tokio::main]
async fn main() {
    let uri = "ws://localhost:1977";
    println!("Starting socks5 service provider:");
    let network_monitor = network::Monitor::new(uri.into());
    network_monitor.run().await;
}
