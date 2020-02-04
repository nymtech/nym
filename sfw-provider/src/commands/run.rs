use crate::provider;
use crate::provider::ServiceProvider;
use clap::{App, Arg, ArgMatches};
use std::net::ToSocketAddrs;
use std::path::PathBuf;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("run")
        .about("Starts the service provider")
        .arg(
            Arg::with_name("mixHost")
                .long("mixHost")
                .help("The custom host on which the service provider will be running for receiving sphinx packets")
                .takes_value(true)
                .required(true),
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
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("clientPort")
                .long("clientPort")
                .help("The port on which the service provider will be listening for client sfw-provider-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("storeDir")
                .short("s")
                .long("storeDir")
                .help("Directory storing all packets for the clients")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("registeredLedger")
                .short("r")
                .long("registeredLedger")
                .help("Directory of the ledger of registered clients")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the node is sending presence and metrics to")
                .takes_value(true),
        )
}

fn show_binding_warning(address: String) {
    println!("\n##### WARNING #####");
    println!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes\n\
         You can ignore this warning if you're running setup on a local network \n\
         or have set a custom 'announce-host'",
        address
    );
    println!("\n##### WARNING #####\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

fn new_config(matches: &ArgMatches) -> provider::Config {
    let directory_server = matches
        .value_of("directory")
        .unwrap_or("https://directory.nymtech.net")
        .to_string();

    let mix_host = matches.value_of("mixHost").unwrap();
    let mix_port = match matches.value_of("mixPort").unwrap_or("8085").parse::<u16>() {
        Ok(n) => n,
        Err(err) => panic!("Invalid mix host port value provided - {:?}", err),
    };

    let client_host = matches.value_of("clientHost").unwrap();
    let client_port = match matches
        .value_of("clientPort")
        .unwrap_or("9000")
        .parse::<u16>()
    {
        Ok(n) => n,
        Err(err) => panic!("Invalid client port value provided - {:?}", err),
    };

    if special_addresses().contains(&mix_host) {
        show_binding_warning(mix_host.parse().unwrap());
    }

    if special_addresses().contains(&client_host) {
        show_binding_warning(client_host.parse().unwrap());
    }

    let key_pair = crypto::identity::MixIdentityKeyPair::new(); // TODO: persist this so keypairs don't change every restart
    let store_dir = PathBuf::from(
        matches
            .value_of("storeDir")
            .unwrap_or("/tmp/nym-provider/inboxes"),
    );
    let registered_client_ledger_dir = PathBuf::from(
        matches
            .value_of("registeredLedger")
            .unwrap_or("/tmp/nym-provider/registered_clients"),
    );

    println!("store_dir is: {:?}", store_dir);
    println!(
        "registered_client_ledger_dir is: {:?}",
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

    println!("Listening for mixnet packets on {}", mix_socket_address);
    println!("Listening for client requests on {}", client_socket_address);

    provider::Config {
        mix_socket_address,
        directory_server,
        public_key: key_pair.public_key,
        client_socket_address,
        secret_key: key_pair.private_key,
        store_dir: PathBuf::from(store_dir),
    }
}

pub fn execute(matches: &ArgMatches) {
    let config = new_config(matches);
    let provider = ServiceProvider::new(config);

    provider.start().unwrap()
}
