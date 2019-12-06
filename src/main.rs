use crate::node::MixNode;
use clap::{App, Arg, ArgMatches, SubCommand};
use curve25519_dalek::scalar::Scalar;
use std::net::ToSocketAddrs;
use std::process;

mod mix_peer;
mod node;

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("run", Some(m)) => Ok(run(m)),
        _ => Err(String::from("Unknown command")),
    }
}

fn run(matches: &ArgMatches) {
    println!("Running the mixnode!");

    let host = matches.value_of("host").unwrap_or("0.0.0.0");

    let port = match matches.value_of("port").unwrap().parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let layer = match matches.value_of("layer").unwrap().parse::<usize>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid layer value provided - {:?}", err),
    };

    let secret_key: Scalar = match matches.value_of("keyfile") {
        Some(keyfile) => {
            println!("Todo: load keyfile from <{:?}>", keyfile);
            Default::default()
        }
        None => {
            println!("Todo: generate fresh sphinx keypair");
            Default::default()
        }
    };

    println!("The value of host is: {:?}", host);
    println!("The value of port is: {:?}", port);
    println!("The value of layer is: {:?}", layer);
    println!("The value of key is: {:?}", secret_key);

    let socket_address = (host, port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    println!("The full combined socket address is {}", socket_address);

    // make sure our socket_address is equal to our predefined-hardcoded value
    assert_eq!("127.0.0.1:8080", socket_address.to_string());

    let mix = MixNode::new(socket_address, secret_key, layer);
    mix.start_listening().unwrap();
}

fn main() {
    let arg_matches = App::new("Nym Mixnode")
        .version("0.1.0")
        .author("Nymtech")
        .about("Implementation of the Loopix-based Mixnode")
        .subcommand(
            SubCommand::with_name("run")
                .about("Starts the mixnode")
                .arg(
                    Arg::with_name("host")
                        .short("h")
                        .long("host")
                        .help("The custom host on which the mixnode will be running")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("The port on which the mixnode will be listening")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("layer")
                        .short("l")
                        .long("layer")
                        .help("The mixnet layer of this particular node")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("keyfile")
                        .short("k")
                        .long("keyfile")
                        .help("Optional path to the persistent keyfile of the node")
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Err(e) = execute(arg_matches) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}
