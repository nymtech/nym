use examples_common::setup_logging;
use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    setup_logging();

    // Passing no config makes the client fire up an ephemeral session and figure shit out on its own
    let mut client = mixnet::Client::new(None, None).unwrap();

    // Connect to the mixnet, now we're listening for incoming
    client.connect_to_mixnet().await;

    // Be able to get our client address
    let our_address = client.nym_address().unwrap();
    println!("Our client nym address is: {our_address}");

    // Send a message throught the mixnet to ourselves
    client
        .send_str(&our_address.to_string(), "hello there")
        .await;

    println!("Waiting for message");
    client
        .on_messages(|msg| println!("Received: {}", String::from_utf8_lossy(&msg.message)))
        .await;
}
