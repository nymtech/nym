mod connection;
mod core;
mod websocket;

#[tokio::main]
async fn main() {
    let uri = "ws://localhost:1977";
    println!("Starting socks5 service provider:");
    let mut server = core::ServiceProvider::new(uri.into());
    server.run().await;
}
