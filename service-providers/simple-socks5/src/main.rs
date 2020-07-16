use crate::core::Server;

mod core;
mod socks5_proxy;
mod websocket;

fn main() {
    let mut server = Server::new();
    println!("Starting socks5 service provider:");
    server.start();
    server.run_forever();
}
