// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::override_config;
use crate::config::{persistence::pathfinder::MixNodePathfinder, Config};
use crate::node::node_description::NodeDescription;
use crate::node::MixNode;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use log::warn;
use version_checker::is_minor_version_compatible;

pub fn command_args<'a, 'b>() -> App<'a, 'b> {
    App::new("run")
        .about("Starts the mixnode")
        .arg(
            Arg::with_name("id")
                .long("id")
                .help("Id of the nym-mixnode we want to run")
                .takes_value(true)
                .required(true),
        )
        // the rest of arguments are optional, they are used to override settings in config file
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
            Arg::with_name("validators")
                .long("validators")
                .help("Comma separated list of rest endpoints of the validators")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("mixnet-contract")
                .long("mixnet-contract")
                .help("Address of the validator contract managing the network")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("metrics-server")
                .long("metrics-server")
                .help("Server to which the node is sending all metrics data")
                .takes_value(true),
        )
}

fn show_binding_warning(address: String) {
    println!("\n##### NOTE #####");
    println!(
        "\nYou are trying to bind to {} - you might not be accessible to other nodes\n\
         You can ignore this note if you're running setup on a local network \n\
         or have set a custom 'announce-host'",
        address
    );
    println!("\n\n");
}

fn special_addresses() -> Vec<&'static str> {
    vec!["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"]
}

fn load_identity_keys(pathfinder: &MixNodePathfinder) -> identity::KeyPair {
    let identity_keypair: identity::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_identity_key().to_owned(),
        pathfinder.public_identity_key().to_owned(),
    ))
    .expect("Failed to read stored identity key files");
    identity_keypair
}

fn load_sphinx_keys(pathfinder: &MixNodePathfinder) -> encryption::KeyPair {
    let sphinx_keypair: encryption::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_encryption_key().to_owned(),
        pathfinder.public_encryption_key().to_owned(),
    ))
    .expect("Failed to read stored sphinx key files");
    sphinx_keypair
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = cfg.get_version();
    if binary_version != config_version {
        warn!("The mixnode binary has different version than what is specified in config file! {} and {}", binary_version, config_version);
        if is_minor_version_compatible(binary_version, config_version) {
            info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    } else {
        true
    }
}

pub fn execute(matches: &ArgMatches) {
    let id = matches.value_of("id").unwrap();

    println!("Starting mixnode {}...", id);

    let mut config = match Config::load_from_file(id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})", id, err);
            return;
        }
    };

    config = override_config(config, matches);

    if !version_check(&config) {
        error!("failed the local version check");
        return;
    }

    let pathfinder = MixNodePathfinder::new_from_config(&config);
    let identity_keypair = load_identity_keys(&pathfinder);
    let sphinx_keypair = load_sphinx_keys(&pathfinder);

    let listening_ip_string = config.get_listening_address().ip().to_string();
    if special_addresses().contains(&listening_ip_string.as_ref()) {
        show_binding_warning(listening_ip_string);
    }

    println!(
        "Validator servers: {:?}",
        config.get_validator_rest_endpoints()
    );
    println!("Metrics server: {}", config.get_metrics_server());

    println!(
        "Listening for incoming packets on {}",
        config.get_listening_address()
    );
    println!(
        "Announcing the following socket address: {}",
        config.get_announce_address()
    );

    println!(
        "\nTo bond your mixnode, go to https://web-wallet-finney.nymtech.net/.  You will need to provide the following:
    Identity key: {}
    Sphinx key: {}
    Host: {}
    Layer: {}
    Location: [physical location of your node's server]
    Version: {}
    ",
        identity_keypair.public_key().to_base58_string(),
        sphinx_keypair.public_key().to_base58_string(),
        config.get_announce_address(),
        config.get_layer(),
        config.get_version(),
    );

    let description =
        NodeDescription::load_from_file(Config::default_config_directory(id)).unwrap_or_default();
    MixNode::new(config, description, identity_keypair, sphinx_keypair).run();
}
