use nym_sdk::mixnet;

#[tokio::main]
async fn main() {
    // here's what I'd actually like to write
    let client = mixnet::Client::new(None); // passing no config makes the client fire up an ephemeral session and figure shit out on its own

    let show_receive = move || println!("got a message from the mixnet: {}", message); // might need to bury this in a struct as a `FnMut`, see https://stackoverflow.com/questions/41081240/idiomatic-callbacks-in-rust
    client.on_receive(show_receive); // have some way to pipe any received info to a function for processing

    // connect to the mixnet, now we're listening for incoming
    client.connect_to_mixnet();

    // be able to get our client address
    println!("Our client address is {}", client.nym_address);

    // send important string info up the pipe to a buddy
    client.send_str("foo.bar@blah", "flappappa");

    // send some bytes to a buddy
    client.send_bytes("foo.bar@blah", "flappappa".as_bytes().to_vec());
}
