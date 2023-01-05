use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    logging::setup_logging();

    // We can set a few options
    let user_chosen_gateway_id = None;
    let nym_api_endpointgs = vec![];
    let config = mixnet::Config::new(user_chosen_gateway_id, nym_api_endpointgs);

    let mut client = mixnet::Client::new(Some(config), None).unwrap();

    // In this we want to provide our own gateway config struct, and handle persisting this info to disk
    // ourselves (e.g., as part of our own configuration file).
    // NOTE: gateway shared key is written to disk according to the path given earlier
    // Checks if we have a shared gateway key loaded
    let first_run = true;
    if first_run {
        client.register_with_gateway().await.unwrap();
        write_to_storage(client.get_keys(), client.get_gateway_endpoint().unwrap());
    } else {
        let (keys, gateway_config) = read_from_storage();
        client.set_keys(&keys);
        client.set_gateway_endpoint(&gateway_config);
    }

    // Connect to the mixnet, now we're listening for incoming
    client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    println!("Our client address is {}", client.nym_address().unwrap());

    // Send important info up the pipe to a buddy
    client.send_str("foo.bar@blah", "flappappa").await;
}

fn write_to_storage(_keys: &mixnet::Keys, _gateway_config: &mixnet::GatewayEndpointConfig) {
    todo!();
}

fn read_from_storage() -> (mixnet::Keys, mixnet::GatewayEndpointConfig) {
    todo!();
}
