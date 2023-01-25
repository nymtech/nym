use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    logging::setup_logging();

    // We can set a few options
    let user_chosen_gateway_id = None;
    let nym_api_endpoints = vec!["https://validator.nymtech.net/api/".parse().unwrap()];

    let config = mixnet::Config::new(user_chosen_gateway_id, nym_api_endpoints);

    let mut client = mixnet::MixnetClient::builder(Some(config), None)
        .await
        .unwrap();

    // Just some plain data to pretend we have some external storage that the application
    // implementer is using.
    let mut mock_storage = MockStorage::empty();

    // In this we want to provide our own gateway config struct, and handle persisting this info to disk
    // ourselves (e.g., as part of our own configuration file).
    let first_run = true;
    if first_run {
        client.register_with_gateway().await.unwrap();
        mock_storage.write(client.get_keys(), client.get_gateway_endpoint().unwrap());
    } else {
        let (keys, gateway_config) = mock_storage.read();
        client.set_keys(keys);
        client.set_gateway_endpoint(gateway_config);
    }

    // Connect to the mixnet, now we're listening for incoming
    let client = client.connect_to_mixnet().await.unwrap();

    // Be able to get our client address
    println!("Our client address is {}", client.nym_address());

    // Send important info up the pipe to a buddy
    let recipient = mixnet::Recipient::try_from_base58_string("foo.bar@blah").unwrap();
    client.send_str(recipient, "flappappa").await;
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
        todo!();
    }

    fn empty() -> Self {
        Self {
            gateway_config: None,
            keys: None,
        }
    }
}
