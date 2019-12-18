use crate::provider::presence;
use crate::provider::ServiceProvider;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::process;
use std::thread;

pub mod provider;

fn main() {
    let arg_matches = App::new("Nym Service Provider")
        .version("0.1.0")
        .author("Nymtech")
        .about("Implementation of the Loopix-based Service Provider")
        .subcommand(
            SubCommand::with_name("run")
                .about("Starts the service provider")
                .arg(
                    Arg::with_name("mixHost")
                        .long("mixHost")
                        .help("The custom host on which the service provider will be running for receiving sphinx packets")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("mixPort")
                        .long("mixPort")
                        .help("The port on which the service provider will be listening for sphinx packets")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("clientHost")
                        .long("clientHost")
                        .help("The custom host on which the service provider will be running for receiving client sfw-provider-requests")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("clientPort")
                        .long("clientPort")
                        .help("The port on which the service provider will be listening for client sfw-provider-requests")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("keyfile")
                        .short("k")
                        .long("keyfile")
                        .help("Optional path to the persistent keyfile of the node")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("storeDir")
                        .short("s")
                        .long("storeDir")
                        .help("Directory storing all packets for the clients")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("registeredLedger")
                        .short("r")
                        .long("registeredLedger")
                        .help("Directory of the ledger of registered clients")
                        .takes_value(true)
                        .required(true),
                ).arg(Arg::with_name("local")
                    .long("local")
                    .help("Flag to indicate whether the provider should run on a local deployment.")
                    .takes_value(false)
                )
        )
        .get_matches();

    if let Err(e) = execute(arg_matches) {
        println!("{}", e);
        process::exit(1);
    }
}

fn run(matches: &ArgMatches) {
    let config = new_config(matches);
    let provider = ServiceProvider::new(&config);

    // Start sending presence notifications in a separate thread
    thread::spawn(move || {
        let notifier = presence::Notifier::new(&config);
        notifier.run();
    });

    provider.start().unwrap()
}

fn execute(matches: ArgMatches) -> Result<(), String> {
    match matches.subcommand() {
        ("run", Some(m)) => Ok(run(m)),
        _ => Err(usage()),
    }
}

fn new_config(matches: &ArgMatches) -> provider::Config {
    println!("Running the service provider!");
    let is_local = matches.is_present("local");

    let directory_server = if is_local {
        "http://localhost:8080".to_string()
    } else {
        "https://directory.nymtech.net".to_string()
    };

    let mix_host = matches.value_of("mixHost").unwrap_or("0.0.0.0");
    let mix_port = match matches.value_of("mixPort").unwrap_or("8085").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let client_host = matches.value_of("clientHost").unwrap_or("0.0.0.0");
    let client_port = match matches
        .value_of("clientPort")
        .unwrap_or("9000")
        .parse::<u16>()
    {
        Ok(n) => n,
        Err(err) => panic!("Invalid port value provided - {:?}", err),
    };

    let store_dir = PathBuf::from(matches.value_of("storeDir").unwrap_or("/tmp/nym-provider"));
    let registered_client_ledger_dir = PathBuf::from(matches.value_of("registeredLedger").unwrap());
    let (secret_key, public_key) = sphinx::crypto::keygen();

    println!("The value of mix_host is: {:?}", mix_host);
    println!("The value of mix_port is: {:?}", mix_port);
    println!("The value of client_host is: {:?}", client_host);
    println!("The value of client_port is: {:?}", client_port);
    println!("The value of key is: {:?}", secret_key);
    println!("The value of store_dir is: {:?}", store_dir);
    println!(
        "The value of registered_client_ledger_dir is: {:?}",
        registered_client_ledger_dir
    );

    let mix_socket_address = (mix_host, mix_port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    let client_socket_address = (client_host, client_port)
        .to_socket_addrs()
        .expect("Failed to combine host and port")
        .next()
        .expect("Failed to extract the socket address from the iterator");

    println!(
        "The full combined mix socket address is {}",
        mix_socket_address
    );
    println!(
        "The full combined client socket address is {}",
        client_socket_address
    );

    provider::Config {
        directory_server,
        public_key,
        client_socket_address,
        mix_socket_address,
        secret_key,
        store_dir: PathBuf::from(store_dir),
    }
}

fn usage() -> String {
    banner() + "usage: --help to see available options.\n\n"
}

fn banner() -> String {
    return r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (store-and-forward provider)

    "#
    .to_string();
}
