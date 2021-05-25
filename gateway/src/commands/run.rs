// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::override_config;
use crate::config::persistence::pathfinder::GatewayPathfinder;
use crate::config::Config;
use crate::node::Gateway;
use clap::{App, Arg, ArgMatches};
use config::NymConfig;
use crypto::asymmetric::{encryption, identity};
use log::*;
use version_checker::is_minor_version_compatible;

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

fn load_sphinx_keys(pathfinder: &GatewayPathfinder) -> encryption::KeyPair {
    let sphinx_keypair: encryption::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_encryption_key().to_owned(),
        pathfinder.public_encryption_key().to_owned(),
    ))
    .expect("Failed to read stored sphinx key files");
    println!(
        "Public sphinx key: {}\n",
        sphinx_keypair.public_key().to_base58_string()
    );
    sphinx_keypair
}

fn load_identity_keys(pathfinder: &GatewayPathfinder) -> identity::KeyPair {
    let identity_keypair: identity::KeyPair = pemstore::load_keypair(&pemstore::KeyPairPath::new(
        pathfinder.private_identity_key().to_owned(),
        pathfinder.public_identity_key().to_owned(),
    ))
    .expect("Failed to read stored identity key files");
    println!(
        "Public identity key: {}\n",
        identity_keypair.public_key().to_base58_string()
    );
    identity_keypair
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

    println!("Starting gateway {}...", id);

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

    let pathfinder = GatewayPathfinder::new_from_config(&config);
    let sphinx_keypair = load_sphinx_keys(&pathfinder);
    let identity = load_identity_keys(&pathfinder);

    let mix_listening_ip_string = config.get_mix_listening_address().ip().to_string();
    if special_addresses().contains(&mix_listening_ip_string.as_ref()) {
        show_binding_warning(mix_listening_ip_string);
    }

    let clients_listening_ip_string = config.get_clients_listening_address().ip().to_string();
    if special_addresses().contains(&clients_listening_ip_string.as_ref()) {
        show_binding_warning(clients_listening_ip_string);
    }

    println!(
        "Validator servers: {:?}",
        config.get_validator_rest_endpoints()
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
        "Clients ledger is stored at: {:?}",
        config.get_clients_ledger_path()
    );

    Gateway::new(config, sphinx_keypair, identity).run();
}
