use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    println!("Connecting receiver");
    let mut receiving_client = mixnet::MixnetClient::connect_new().await.unwrap();

    let config = mixnet::Config {
        socks5_service_provider: Some(receiving_client.nym_address().to_string()),
        ..Default::default()
    };
    let sending_client = mixnet::MixnetClientBuilder::new()
        .config(config)
        .build::<mixnet::EmptyReplyStorage>()
        .await
        .unwrap();

    println!("Connecting sender");
    let mut sending_client = sending_client.connect_to_mixnet().await.unwrap();
    // wait until socks5 server is started
    println!("Wait 5 seconds for the socks5 setup to be done");
    std::thread::sleep(std::time::Duration::from_secs(5));

    let proxy = reqwest::Proxy::all(format!(
        "socks5h://127.0.0.1:{}",
        nym_network_defaults::DEFAULT_SOCKS5_LISTENING_PORT
    ))
    .unwrap();
    let reqwest_client = reqwest::Client::builder().proxy(proxy).build().unwrap();
    tokio::spawn(async move {
        println!("Sending socks5-wrapped http request");
        // Message should be sent through the mixnet, via socks5
        // We don't expect to get anything, as there is no network requester on the other end
        reqwest_client.get("https://nymtech.net").send().await.ok()
    });

    println!("Waiting for message");
    if let Some(received) = receiving_client.wait_for_messages().await {
        for r in received {
            println!(
                "Received socks5 message: {}",
                String::from_utf8_lossy(&r.message)
            );
        }
    }

    receiving_client.disconnect().await;
    sending_client.disconnect().await;
}
