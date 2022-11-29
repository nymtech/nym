#[tokio::main]
async fn main() {
    // specify some config options
    let config = mixnet::Config {
        option: value,
        option2: value2,
        keyfiles: "~/.flappapapappa/keys/",
    };

    let client = mixnet::Client::new(&config); // passing a config allows the user to set values

    let show_receive = move || println!("got a message from the mixnet: {}", message); // might need to bury this in a struct as a `FnOnce`, see https://stackoverflow.com/questions/41081240/idiomatic-callbacks-in-rust
    client.on_receive(show_receive); // have some way to pipe any received info to a function for processing

    // connect to the mixnet, now we're listening for incoming
    client.connect_to_mixnet();

    // be able to get our client address
    println!("Our client address is {}", client.nym_address);

    // send important info up the pipe to a buddy
    client.send("foo.bar@blah", "flappappa");
}
