use crate::client_handling::clients_handler::ClientsHandler;
use crate::client_handling::websocket;
use crate::commands::override_config;
use crate::config::persistence::pathfinder::GatewayPathfinder;
use crate::config::Config;
use crate::mixnet_handling;
use crate::mixnet_handling::receiver::packet_processing::PacketProcessor;
use crate::mixnet_handling::sender::PacketForwarder;
use crate::storage::ClientStorage;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::encryption;
use log::*;
use pemstore::pemstore::PemStore;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub fn command_args<'a, 'b>() -> clap::App<'a, 'b> {
    App::new("run")
        .about("Starts the gateway")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the gateway we want to run")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments are optional, they are used to override settings in config file
        .arg(
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this gateway")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .help("Custom path to the nym gateway configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mix-host")
                .long("mix-host")
                .help("The custom host on which the gateway will be running for receiving sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("mix-port")
                .long("mix-port")
                .help("The port on which the gateway will be listening for sphinx packets")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-host")
                .long("clients-host")
                .help("The custom host on which the gateway will be running for receiving clients gateway-requests")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("clients-port")
                .long("clients-port")
                .help("The port on which the gateway will be listening for clients gateway-requests")
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
                .help("Ledger file containing registered clients")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("directory")
                .long("directory")
                .help("Address of the directory server the gateway is sending presence data to")
                .takes_value(true),
        )
}

fn show_binding_warning(address: String) {
    println!("\n##### NOTE #####");
    println!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes\n\
         You can ignore this warning if you're running setup on a local network \n\
         or have set a custom 'announce-host'",
        address
    );
    println!("\n\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

fn load_sphinx_keys(config_file: &Config) -> encryption::KeyPair {
    let sphinx_keypair = PemStore::new(GatewayPathfinder::new_from_config(&config_file))
        .read_encryption()
        .expect("Failed to read stored sphinx key files");
    println!(
        "Public key: {}\n",
        sphinx_keypair.public_key().to_base58_string()
    );
    sphinx_keypair
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting gateway {}...", id);

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

    // TODO: define them in config
    let initial_reconnection_backoff = Duration::from_millis(10_000);
    let maximum_reconnection_backoff = Duration::from_millis(300_000);
    let initial_connection_timeout = Duration::from_millis(1500);

    // very temporary will be moved into 'Gateway' struct within few next commits
    // this is literally what #[tokio::main] is doing anyway (well, not 'literally', it's
    // a bit of simplification from my side, but the end result is the same)
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let keypair = load_sphinx_keys(&config);

        let arced_sk = Arc::new(keypair.private_key().to_owned());

        // TODO: this should really be a proper DB, right now it will be most likely a bottleneck,
        // due to possible frequent independent writes
        let client_storage = ClientStorage::new(
            config.get_message_retrieval_limit() as usize,
            config.get_stored_messages_filename_length(),
            config.get_clients_inboxes_dir(),
        );

        let (_, forwarding_channel) = PacketForwarder::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
        )
        .start();

        let (_, clients_handler_sender) = ClientsHandler::new(
            Arc::clone(&arced_sk),
            config.get_clients_ledger_path(),
            client_storage.clone(),
        )
        .start();

        let packet_processor =
            PacketProcessor::new(arced_sk, clients_handler_sender.clone(), client_storage);

        websocket::Listener::new(config.get_clients_listening_address())
            .start(clients_handler_sender, forwarding_channel);
        mixnet_handling::Listener::new(config.get_mix_listening_address()).start(packet_processor);

        info!("All up and running!");

        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }

        println!(
            "Received SIGINT - the gateway will terminate now (threads are not YET nicely stopped)"
        );
    });
}

//
//#[tokio::main]
//async fn main() {
//    dotenv::dotenv().ok();
//    setup_logging();
//    // TODO: assume config is parsed here, keys are loaded, etc
//    // ALL OF BELOW WILL BE DONE VIA CONFIG
//    let keypair = crypto::encryption::KeyPair::new();
//    let clients_addr = "127.0.0.1:9000".parse().unwrap();
//    let mix_addr = "127.0.0.1:1789".parse().unwrap();
//    let inbox_store_dir: PathBuf = "foomp".into();
//    let ledger_path: PathBuf = "foomp2".into();
//    let message_retrieval_limit = 1000;
//    let filename_len = 16;
//    let initial_reconnection_backoff = Duration::from_millis(10_000);
//    let maximum_reconnection_backoff = Duration::from_millis(300_000);
//    let initial_connection_timeout = Duration::from_millis(1500);
//    // ALL OF ABOVE WILL HAVE BEEN DONE VIA CONFIG
//
//    let arced_sk = Arc::new(keypair.private_key().to_owned());
//
//    // TODO: this should really be a proper DB, right now it will be most likely a bottleneck,
//    // due to possible frequent independent writes
//    let client_storage = ClientStorage::new(message_retrieval_limit, filename_len, inbox_store_dir);
//
//    let (_, forwarding_channel) = PacketForwarder::new(
//        initial_reconnection_backoff,
//        maximum_reconnection_backoff,
//        initial_connection_timeout,
//    )
//    .start();
//
//    let (_, clients_handler_sender) =
//        ClientsHandler::new(Arc::clone(&arced_sk), ledger_path, client_storage.clone()).start();
//
//    let packet_processor =
//        PacketProcessor::new(arced_sk, clients_handler_sender.clone(), client_storage);
//
//    websocket::Listener::new(clients_addr).start(clients_handler_sender, forwarding_channel);
//    mixnet_handling::Listener::new(mix_addr).start(packet_processor);
//
//    info!("All up and running!");
//
//    if let Err(e) = tokio::signal::ctrl_c().await {
//        error!(
//            "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
//            e
//        );
//    }
//
//    println!(
//        "Received SIGINT - the gateway will terminate now (threads are not YET nicely stopped)"
//    );
//}
//
