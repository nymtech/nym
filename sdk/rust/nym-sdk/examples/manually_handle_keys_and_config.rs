use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    logging::setup_logging();

    let user_chosen_gateway_id = None;
    let nym_api_endpoints = vec!["https://validator.nymtech.net/api/".parse().unwrap()];
    let config = mixnet::Config::new(user_chosen_gateway_id, nym_api_endpoints);

    // Just some plain data to pretend we have some external storage that the application
    // implementer is using.
    let mut mock_storage = MockStorage::empty();

    let first_run = true;

    let client = if first_run {
        // Create a client without a storage backend
        let mut client = mixnet::MixnetClientBuilder::new()
            .config(config)
            .build::<mixnet::EmptyReplyStorage>()
            .await
            .unwrap();

        // In this we want to provide our own gateway config struct, and handle persisting this info to disk
        // ourselves (e.g., as part of our own configuration file).
        client.register_and_authenticate_gateway().await.unwrap();
        mock_storage.write(client.get_keys(), client.get_gateway_endpoint().unwrap());
        client
    } else {
        let (keys, gateway_config) = mock_storage.read();

        // Create a client without a storage backend, but with explicitly set keys and gateway
        // configuration. This creates the client in a registered state.
        let client = mixnet::MixnetClientBuilder::new()
            .config(config)
            .keys(keys)
            .gateway_config(gateway_config)
            .build::<mixnet::EmptyReplyStorage>()
            .await
            .unwrap();
        client
    };

    // Connect to the mixnet, now we're listening for incoming
    let mut client = client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    let our_address = client.nym_address();
    println!("Our client nym address is: {our_address}");

    // Send important info up the pipe to a buddy
    client.send_str(*our_address, "hello there").await;

    println!("Waiting for message");
    if let Some(received) = client.wait_for_messages().await {
        for r in received {
            println!("Received: {}", String::from_utf8_lossy(&r.message));
        }
    }

    client.disconnect().await;
}

#[allow(unused)]
struct MockStorage {
    pub gateway_config: Option<mixnet::GatewayEndpointConfig>,
    pub keys: Option<Vec<u8>>,
}

impl MockStorage {
    fn read(&self) -> (mixnet::Keys, mixnet::GatewayEndpointConfig) {
        todo!();
    }

    fn write(&mut self, _keys: mixnet::KeysArc, _gateway_config: &mixnet::GatewayEndpointConfig) {
        log::info!("todo");
    }

    fn empty() -> Self {
        Self {
            gateway_config: None,
            keys: None,
        }
    }
}
