mod network;

fn main() {
    let websocket_uri = "ws://localhost:1977";
    let directory_uri = "https://directory.nymtech.net";
    let good_topology = network::good_topology::construct();

    println!("Starting network monitor:");
    let mut gateway_client =
        network::clients::new_gateway_client(network::good_topology::gateway());
    gateway_client.authenticate_and_start();
    let mut network_monitor =
        network::Monitor::new(directory_uri, good_topology, gateway_client, websocket_uri);
    network_monitor.run();
}
