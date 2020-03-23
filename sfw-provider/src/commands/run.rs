use crate::commands::override_config;
use crate::config::Config;
use crate::provider::ServiceProvider;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;

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
            Arg::with_name("location")
                .long("location")
                .help("Optional geographical location of this node")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .help("Custom path to the nym-provider configuration file")
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

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting sfw-provider {}...", id);

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

    ServiceProvider::new(config).run();
}
