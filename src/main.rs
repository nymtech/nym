use crate::node::MixNode;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::net::{SocketAddr, ToSocketAddrs};
use std::process;

mod mix_peer;
mod node;

//
//fn run(matches: ArgMatches) -> Result<(), String> {
//    // ...
//    match matches.subcommand() {
//        ("analyse", Some(m)) => run_analyse(m, &logger),
//        ("verify", Some(m)) => run_verify(m, &logger),
//        _ => Ok(()),
//    }
//}
//
//fn run_analyse(matches: &ArgMatches, parent_logger: &slog::Logger) -> Result<(), String> {
//    let logger = parent_logger.new(o!("command" => "analyse"));
//    let input = matches.value_of("input-file").unwrap();
//    debug!(logger, "analysis_started"; "input_file" => input);
//    // ...
//    Ok(())
//}
//
//fn run_verify(matches: &ArgMatches, parent_logger: &slog::Logger) -> Result<(), String> {
//    let logger = parent_logger.new(o!("command" => "verify"));
//    let algorithm = value_t!(matches.value_of("algorithm"), Algorithm).unwrap();
//    debug!(logger, "verification_started"; "algorithm" => format!("{:?}", algorithm));
//    // ...
//    Ok(())
//}

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("run", Some(m)) => run(m),
        _ => Err(String::from("Unknown command")),
    }
}

fn run(matches: &ArgMatches) -> Result<(), String> {
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

    let key = match matches.value_of("keyfile") {
        Some(keyfile) => {
            println!("Todo: load keyfile from <{:?}>", keyfile);
            "dummy key1"
        }
        None => {
            println!("Todo: generate fresh sphinx keypair");
            "dummy key2"
        }
    };

    println!("The value of host is: {}", host);
    println!("The value of port is: {}", port);
    println!("The value of layer is: {}", layer);
    println!("The value of key is: {}", key);

    let socket_address = (host, port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    println!("The full combined socket address is {}", socket_address);
    Ok(())
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

    //    let mix = MixNode::new("127.0.0.1:8080", Default::default());
    //    mix.start_listening().unwrap();
}
