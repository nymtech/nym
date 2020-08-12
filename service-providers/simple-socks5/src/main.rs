mod connection;
mod controller;
mod core;
mod websocket;

fn main() {
    println!("Starting socks5 service provider:");
    let mut server = core::ServiceProvider::new();
    server.start();
    server.run_forever();
}
