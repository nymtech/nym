# Socks Proxy

If you are looking at implementing Nym as a transport layer for a crypto wallet or desktop app, this is probably the best place to start if they can speak SOCKS5, 4a, or 4.

> You can find this code [here](https://github.com/nymtech/nym/blob/master/sdk/rust/nym-sdk/examples/socks5.rs)

```rust
use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    nym_bin_common::logging::setup_logging();

    println!("Connecting receiver");
    let mut receiving_client = mixnet::MixnetClient::connect_new().await.unwrap();

    let socks5_config = mixnet::Socks5::new(receiving_client.nym_address().to_string());
    let sending_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .socks5_config(socks5_config)
        .build()
        .unwrap();

    println!("Connecting sender");
    let sending_client = sending_client.connect_to_mixnet_via_socks5().await.unwrap();

    let proxy = reqwest::Proxy::all(sending_client.socks5_url()).unwrap();
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
```
