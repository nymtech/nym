use crate::banner;
use crate::node;
use crate::node::MixNode;
use clap::ArgMatches;
use std::net::ToSocketAddrs;

fn print_binding_warning(address: &str) {
    println!("\n##### WARNING #####");
    println!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes",
        address
    );
    println!("\n##### WARNING #####\n");
}

pub fn start(matches: &ArgMatches) {
    println!("{}", banner());
    println!("Starting mixnode...");

    let config = new_config(matches);
    println!("Public key: {}", config.public_key_string());
    println!("Directory server: {}", config.directory_server);
    println!(
        "Listening for incoming packets on {}",
        config.socket_address
    );

    let mix = MixNode::new(&config);
    mix.start(config).unwrap();
}

fn new_config(matches: &ArgMatches) -> node::Config {
    let host = matches.value_of("host").unwrap();
    if host == "localhost" || host == "127.0.0.1" || host == "0.0.0.0" {
        print_binding_warning(host);
    }

    let port = match matches.value_of("port").unwrap_or("1789").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let layer = match matches.value_of("layer").unwrap().parse::<usize>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid layer value provided - {:?}", err),
    };

    let socket_address = (host, port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    let announce_host = matches.value_of("announce-host").unwrap_or(host);
    let announce_port = matches
        .value_of("announce-port")
        .map(|port| port.parse::<u16>().unwrap())
        .unwrap_or(port);

    let announce_socket_address = (announce_host, announce_port)
        .to_socket_addrs()
        .expect("Failed to combine announce host and port")
        .next()
        .expect("Failed to extract the announce socket address from the iterator");

    let (secret_key, public_key) = sphinx::crypto::keygen();

    let directory_server = matches
        .value_of("directory")
        .unwrap_or("https://directory.nymtech.net")
        .to_string();

    node::Config {
        directory_server,
        layer,
        public_key,
        socket_address,
        announce_socket_address,
        secret_key,
    }
}
