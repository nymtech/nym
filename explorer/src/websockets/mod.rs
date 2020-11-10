pub(crate) mod client;
mod server;

pub async fn subscribe(metrics_socket: &str) {
    println!("Subscribing to metrics websocket at {}", metrics_socket);
    match client::MetricsWebsocket::connect(metrics_socket).await {
        Ok(_) => println!("metrics websocket connected successfully"),
        Err(e) => println!("metrics websocket failed to connect: {:?}", e),
    };
}

pub async fn listen(port: &str) {
    println!("Starting websocket listener on port {}", port);
}
