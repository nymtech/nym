// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::validate_bech32_address_or_exit;
use crate::config::persistence::pathfinder::MixNodePathfinder;
use crate::config::Config;
use crate::node::http::{
    description::description,
    hardware::hardware,
    not_found,
    stats::stats,
    verloc::{verloc as verlocRoute, VerlocState},
};
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::node_description::NodeDescription;
use crate::node::node_statistics::SharedNodeStats;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use ::crypto::asymmetric::{encryption, identity};
use config::NymConfig;
use log::{error, info, warn};
use mixnode_common::verloc::{self, AtomicVerlocResult, VerlocMeasurer};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use task::{wait_for_signal, ShutdownListener, ShutdownNotifier};
use version_checker::parse_version;

mod http;
mod listener;
pub(crate) mod node_description;
mod node_statistics;
mod packet_delayforwarder;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    descriptor: NodeDescription,
    identity_keypair: Arc<identity::KeyPair>,
    sphinx_keypair: Arc<encryption::KeyPair>,
}

impl MixNode {
    pub fn new(config: Config) -> Self {
        let pathfinder = MixNodePathfinder::new_from_config(&config);

        MixNode {
            descriptor: Self::load_node_description(&config),
            identity_keypair: Arc::new(Self::load_identity_keys(&pathfinder)),
            sphinx_keypair: Arc::new(Self::load_sphinx_keys(&pathfinder)),
            config,
        }
    }

    fn load_node_description(config: &Config) -> NodeDescription {
        NodeDescription::load_from_file(Config::default_config_directory(Some(&config.get_id())))
            .unwrap_or_default()
    }

    /// Loads identity keys stored on disk
    pub(crate) fn load_identity_keys(pathfinder: &MixNodePathfinder) -> identity::KeyPair {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .expect("Failed to read stored identity key files");
        identity_keypair
    }

    /// Loads Sphinx keys stored on disk
    fn load_sphinx_keys(pathfinder: &MixNodePathfinder) -> encryption::KeyPair {
        let sphinx_keypair: encryption::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ))
            .expect("Failed to read stored sphinx key files");
        sphinx_keypair
    }

    /// Signs the node config's bech32 address to produce a verification code for use in the wallet.
    /// Exits if the address isn't valid (which should protect against manual edits).
    fn generate_owner_signature(&self) -> String {
        let pathfinder = MixNodePathfinder::new_from_config(&self.config);
        let identity_keypair = Self::load_identity_keys(&pathfinder);
        let address = self.config.get_wallet_address();
        validate_bech32_address_or_exit(address);
        let verification_code = identity_keypair.private_key().sign_text(address);
        verification_code
    }

    /// Prints relevant node details to the console
    pub(crate) fn print_node_details(&self) {
        println!(
            "Identity Key: {}",
            self.identity_keypair.public_key().to_base58_string()
        );
        println!(
            "Sphinx Key: {}",
            self.sphinx_keypair.public_key().to_base58_string()
        );
        println!("Owner Signature: {}", self.generate_owner_signature());
        println!(
            "Host: {} (bind address: {})",
            self.config.get_announce_address(),
            self.config.get_listening_address()
        );
        println!("Version: {}", self.config.get_version());
        println!(
            "Mix Port: {}, Verloc port: {}, Http Port: {}\n",
            self.config.get_mix_port(),
            self.config.get_verloc_port(),
            self.config.get_http_api_port()
        );
        println!(
            "You are bonding to wallet address: {}\n\n",
            self.config.get_wallet_address()
        );
    }

    fn start_http_api(
        &self,
        atomic_verloc_result: AtomicVerlocResult,
        node_stats_pointer: SharedNodeStats,
    ) {
        info!("Starting HTTP API on http://localhost:8000");

        let mut config = rocket::config::Config::release_default();

        // bind to the same address as we are using for mixnodes
        config.address = self.config.get_listening_address();
        config.port = self.config.get_http_api_port();

        let verloc_state = VerlocState::new(atomic_verloc_result);
        let descriptor = self.descriptor.clone();

        tokio::spawn(async move {
            rocket::build()
                .configure(config)
                .mount("/", routes![verlocRoute, description, stats, hardware])
                .register("/", catchers![not_found])
                .manage(verloc_state)
                .manage(descriptor)
                .manage(node_stats_pointer)
                .launch()
                .await
        });
    }

    fn start_node_stats_controller(
        &self,
        shutdown: ShutdownListener,
    ) -> (SharedNodeStats, node_statistics::UpdateSender) {
        info!("Starting node stats controller...");
        let controller = node_statistics::Controller::new(
            self.config.get_node_stats_logging_delay(),
            self.config.get_node_stats_updating_delay(),
            shutdown,
        );
        let node_stats_pointer = controller.get_node_stats_data_pointer();
        let update_sender = controller.start();

        (node_stats_pointer, update_sender)
    }

    fn start_socket_listener(
        &self,
        node_stats_update_sender: node_statistics::UpdateSender,
        delay_forwarding_channel: PacketDelayForwardSender,
        shutdown: ShutdownListener,
    ) {
        info!("Starting socket listener...");

        let packet_processor =
            PacketProcessor::new(self.sphinx_keypair.private_key(), node_stats_update_sender);

        let connection_handler = ConnectionHandler::new(packet_processor, delay_forwarding_channel);

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_mix_port(),
        );

        Listener::new(listening_address, shutdown).start(connection_handler);
    }

    fn start_packet_delay_forwarder(
        &mut self,
        node_stats_update_sender: node_statistics::UpdateSender,
        shutdown: ShutdownListener,
    ) -> PacketDelayForwardSender {
        info!("Starting packet delay-forwarder...");

        let client_config = mixnet_client::Config::new(
            self.config.get_packet_forwarding_initial_backoff(),
            self.config.get_packet_forwarding_maximum_backoff(),
            self.config.get_initial_connection_timeout(),
            self.config.get_maximum_connection_buffer_size(),
        );

        let mut packet_forwarder = DelayForwarder::new(
            mixnet_client::Client::new(client_config),
            node_stats_update_sender,
            shutdown,
        );

        let packet_sender = packet_forwarder.sender();

        tokio::spawn(async move { packet_forwarder.run().await });
        packet_sender
    }

    fn start_verloc_measurements(&self, shutdown: ShutdownListener) -> AtomicVerlocResult {
        info!("Starting the round-trip-time measurer...");

        // this is a sanity check to make sure we didn't mess up with the minimum version at some point
        // and whether the user has run update if they're using old config
        // if this code exists in the node, it MUST BE compatible
        let config_version =
            parse_version(self.config.get_version()).expect("malformed version in the config file");
        let minimum_version = parse_version(verloc::MINIMUM_NODE_VERSION).unwrap();
        if config_version < minimum_version {
            error!("You seem to have not updated your mixnode configuration file - please run `upgrade` before attempting again");
            process::exit(1)
        }

        // use the same binding address with the HARDCODED port for time being (I don't like that approach personally)

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_verloc_port(),
        );

        let config = verloc::ConfigBuilder::new()
            .listening_address(listening_address)
            .packets_per_node(self.config.get_measurement_packets_per_node())
            .connection_timeout(self.config.get_measurement_connection_timeout())
            .packet_timeout(self.config.get_measurement_packet_timeout())
            .delay_between_packets(self.config.get_measurement_delay_between_packets())
            .tested_nodes_batch_size(self.config.get_measurement_tested_nodes_batch_size())
            .testing_interval(self.config.get_measurement_testing_interval())
            .retry_timeout(self.config.get_measurement_retry_timeout())
            .validator_api_urls(self.config.get_validator_api_endpoints())
            .build();

        let mut verloc_measurer =
            VerlocMeasurer::new(config, Arc::clone(&self.identity_keypair), shutdown);
        let atomic_verloc_results = verloc_measurer.get_verloc_results_pointer();
        tokio::spawn(async move { verloc_measurer.run().await });
        atomic_verloc_results
    }

    fn random_api_client(&self) -> validator_client::ApiClient {
        let endpoints = self.config.get_validator_api_endpoints();
        let validator_api = endpoints
            .choose(&mut thread_rng())
            .expect("The list of validator apis is empty");

        validator_client::ApiClient::new(validator_api.clone())
    }

    // TODO: ask DH whether this function still makes sense in ^0.10
    async fn check_if_same_ip_node_exists(&mut self) -> Option<String> {
        let validator_client = self.random_api_client();
        let existing_nodes = match validator_client.get_cached_mixnodes().await {
            Ok(nodes) => nodes,
            Err(err) => {
                error!(
                    "failed to grab initial network mixnodes - {err}\n \
                    Please try to startup again in few minutes",
                );
                process::exit(1);
            }
        };

        let our_host = self.config.get_announce_address();

        existing_nodes
            .iter()
            .find(|node| node.mix_node.host == our_host)
            .map(|node| node.mix_node.identity_key.clone())
    }

    async fn wait_for_interrupt(&self, mut shutdown: ShutdownNotifier) {
        wait_for_signal().await;

        log::info!("Sending shutdown");
        shutdown.signal_shutdown().ok();

        log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
        shutdown.wait_for_shutdown().await;

        log::info!("Stopping nym mixnode");
    }

    pub async fn run(&mut self) {
        info!("Starting nym mixnode");

        if let Some(duplicate_node_key) = self.check_if_same_ip_node_exists().await {
            if duplicate_node_key == self.identity_keypair.public_key().to_base58_string() {
                warn!("You seem to have bonded your mixnode before starting it - that's highly unrecommended as in the future it might result in slashing");
            } else {
                log::error!(
                    "Our announce-host is identical to an existing node's announce-host! (its key is {:?})",
                    duplicate_node_key
                );
                return;
            }
        }

        let shutdown = ShutdownNotifier::default();

        let (node_stats_pointer, node_stats_update_sender) =
            self.start_node_stats_controller(shutdown.subscribe());
        let delay_forwarding_channel = self
            .start_packet_delay_forwarder(node_stats_update_sender.clone(), shutdown.subscribe());
        self.start_socket_listener(
            node_stats_update_sender,
            delay_forwarding_channel,
            shutdown.subscribe(),
        );
        let atomic_verloc_results = self.start_verloc_measurements(shutdown.subscribe());

        // Rocket handles shutdown on it's own, but its shutdown handling should be incorporated
        // with that of the rest of the tasks.
        // Currently it's runtime is forcefully terminated once the mixnode exits.
        self.start_http_api(atomic_verloc_results, node_stats_pointer);

        info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");
        self.wait_for_interrupt(shutdown).await
    }
}
