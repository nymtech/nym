use crate::commands::override_config;
use crate::config::Config;
use crate::provider;
use crate::provider::ServiceProvider;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::identity::{MixIdentityPrivateKey, MixIdentityPublicKey};
use std::net::ToSocketAddrs;
use std::path::PathBuf;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("run")
        .about("Starts the service provider")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-mixnode we want to run")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments are optional, they are used to override settings in config file
        .arg(
            Arg::with_name("config")
                .long("config")
                .help("Custom path to the nym-mixnet-client configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mix-host")
                .long("mix-host")
                .help("The custom host on which the service provider will be running for receiving sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("mix-port")
                .long("mix-port")
                .help("The port on which the service provider will be listening for sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-host")
                .long("clients-host")
                .help("The custom host on which the service provider will be running for receiving clients sfw-provider-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-port")
                .long("clients-port")
                .help("The port on which the service provider will be listening for clients sfw-provider-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("mix-announce-host")
                .long("mix-announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mix-announce-port")
                .long("mix-announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("clients-announce-host")
                .long("clients-announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("clients-announce-port")
                .long("clients-announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("inboxes")
                .long("inboxes")
                .help("Directory with inboxes where all packets for the clients are stored")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-ledger")
                .long("clients-ledger")
                .help("[UNIMPLEMENTED] Ledger file containing registered clients")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the node is sending presence data to")
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

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting mixnode {}...", id);

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);

    let mix_listening_ip_string = config.get_mix_listening_address().ip().to_string();
    if special_addresses().contains(&mix_listening_ip_string.as_ref()) {
        show_binding_warning(mix_listening_ip_string);
    }

    let clients_listening_ip_string = config.get_clients_listening_address().ip().to_string();
    if special_addresses().contains(&clients_listening_ip_string.as_ref()) {
        show_binding_warning(clients_listening_ip_string);
    }

    println!(
        "Directory server [presence]: {}",
        config.get_presence_directory_server()
    );

    println!(
        "Listening for incoming sphinx packets on {}",
        config.get_mix_listening_address()
    );
    println!(
        "Announcing the following socket address for sphinx packets: {}",
        config.get_mix_announce_address()
    );

    println!(
        "Listening for incoming clients packets on {}",
        config.get_clients_listening_address()
    );
    println!(
        "Announcing the following socket address for clients packets: {}",
        config.get_clients_announce_address()
    );

    println!(
        "Inboxes directory is: {:?}",
        config.get_clients_inboxes_dir()
    );
    println!(
        "[UNIMPLEMENTED] Registered client ledger is: {:?}",
        config.get_clients_ledger_path()
    );

    // key will be loaded directly provider in just a moment
    let key_pair = ServiceProvider::load_sphinx_keys(&config);

    // stupid temporary hack
    let private_bytes = key_pair.private_key().to_bytes();
    let public_bytes = key_pair.public_key().to_bytes();

    let old_startup_config = provider::Config {
        client_socket_address: config.get_clients_listening_address(),
        directory_server: config.get_presence_directory_server(),
        mix_socket_address: config.get_mix_listening_address(),
        // identity keys are wrapper for encryption keys so this temporary hack will work to just make it compile once
        public_key: MixIdentityPublicKey::from_bytes(public_bytes.as_ref()),
        secret_key: MixIdentityPrivateKey::from_bytes(private_bytes.as_ref()),
        store_dir: Default::default(),
    };

    let provider = ServiceProvider::new(old_startup_config);

    provider.start().unwrap()
}
