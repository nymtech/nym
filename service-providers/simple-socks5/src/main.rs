mod core;
mod proxy;
mod websocket;

fn main() {
    println!("Starting socks5 service provider:");
    let mut server = core::Server::new();
    server.start();
    server.run_forever();
}
