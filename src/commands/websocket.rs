use crate::banner;
use crate::clients::NymClient;
use clap::ArgMatches;
use std::net::ToSocketAddrs;

pub fn execute(matches: &ArgMatches) {
    let port = match matches.value_of("port").unwrap().parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    println!("{}", banner());
    println!("Starting websocket on port: {:?}", port);
    println!("Listening for messages...");

    let socket_address = ("127.0.0.1", port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    let is_local = matches.is_present("local");
    println!("Starting client, local: {:?}", is_local);

    // todo: to be taken from config or something
    let my_address = [42u8; 32];
    let is_local = true;
    let client = NymClient::new(my_address, is_local);

    client.start(socket_address).unwrap();
}
