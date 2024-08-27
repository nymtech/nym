mod db;
mod error;
mod logging;

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    logging::init().expect("Failed to init logger");

    tracing::info!("Started server");
}
