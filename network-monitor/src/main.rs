mod network;

#[tokio::main]
async fn main() {
    let websocket_uri = "ws://localhost:1977";
    let directory_uri = "https://directory.nymtech.net";

    // I haven't checked these, they're packed at random (I need to go grab our real ones from the server)
    let good_mixnodes = vec![
        "CQVy5fkf4M7EdmoLvH5MJEygqiPbfavUM3NH9eGDK1kt",
        "GjpuFBVzk8KiNsydAaiZG3rZKsoDtv7djCRY1QatKkS5",
        "EV2MTs7DBi95USRNM3hM8QBRiCoYNnXBzs67YHivv3Fh",
    ];
    println!("Starting network monitor:");
    let network_monitor = network::Monitor::new(directory_uri, good_mixnodes, websocket_uri);
    network_monitor.run().await;
}
