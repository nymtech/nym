use crate::commands::override_config;
use crate::config::persistance::pathfinder::MixNodePathfinder;
use crate::config::Config;
use crate::node;
use crate::node::MixNode;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use pemstore::pemstore::PemStore;

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("run")
        .about("Starts the mixnode")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-mixnode we want to create config for.")
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
            Arg::with_name("layer")
                .long("layer")
                .help("The mixnet layer of this particular node")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("host")
                .long("host")
                .help("The custom host on which the mixnode will be running")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .help("The port on which the mixnode will be listening")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce-host")
                .long("announce-host")
                .help("The host that will be reported to the directory server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("announce-port")
                .long("announce-port")
                .help("The port that will be reported to the directory server")
                .takes_value(true),
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

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting mixnode {}...", id);

    let mut config =
        Config::load_from_file(matches.value_of("config").map(|path| path.into()), Some(id))
            .expect("Failed to load config file");

    config = override_config(config, matches);

    let listening_ip_string = config.get_listening_address().ip().to_string();
    if special_addresses().contains(&listening_ip_string.as_ref()) {
        show_binding_warning(listening_ip_string);
    }

    let sphinx_keypair = PemStore::new(MixNodePathfinder::new_from_config(&config))
        .read_encryption()
        .expect("Failed to read stored identity key files");

    println!(
        "Public encryption key: {}\nFor time being, it is identical to identity keys",
        sphinx_keypair.public_key().to_base58_string()
    );

    println!("Directory server: {}", config.get_directory_server());
    println!(
        "Listening for incoming packets on {}",
        config.get_listening_address()
    );
    println!(
        "Announcing the following socket address: {}",
        config.get_announce_address()
    );

    let old_dummy_config = node::Config {
        announce_address: config.get_announce_address(),
        directory_server: config.get_directory_server(),
        layer: config.get_layer(),
        public_key: sphinx_keypair.public_key().0,
        secret_key: sphinx_keypair.private_key().0,
        socket_address: config.get_listening_address(),
    };

    let mix = MixNode::new(&old_dummy_config);
    mix.start(old_dummy_config).unwrap();
}
