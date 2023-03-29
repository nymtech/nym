use nym_sdk::mixnet;
use nym_socks5_client_core::config::Socks5;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    println!("Connecting receiver");
    let mut receiving_client = mixnet::MixnetClient::connect_new().await.unwrap();

    let socks5_config = Socks5::new(receiving_client.nym_address().to_string());
    let client_config = mixnet::Config::new(None, Some(socks5_config), None);
    let sending_client = mixnet::MixnetClientBuilder::new()
        .config(client_config)
        .build::<mixnet::EmptyReplyStorage>()
        .await
        .unwrap();

    println!("Connecting sender");
    let mut sending_client = sending_client.connect_to_mixnet().await.unwrap();

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
                "Received socks5 message requesting for endpoint: {}",
                String::from_utf8_lossy(&r.message[10..27])
            );
        }
    }

    receiving_client.disconnect().await;
    sending_client.disconnect().await;
}
