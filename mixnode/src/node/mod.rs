use crate::config::persistance::pathfinder::MixNodePathfinder;
use crate::config::Config;
use crate::node::packet_processing::PacketProcessor;
use crypto::encryption;
use log::*;
use pemstore::pemstore::PemStore;
use tokio::runtime::Runtime;

mod listener;
mod metrics;
pub(crate) mod packet_processing;
mod presence;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    runtime: Runtime,
    config: Config,
    sphinx_keypair: encryption::KeyPair,
}

impl MixNode {
    fn load_sphinx_keys(config_file: &Config) -> encryption::KeyPair {
        let sphinx_keypair = PemStore::new(MixNodePathfinder::new_from_config(&config_file))
            .read_encryption()
            .expect("Failed to read stored sphinx key files");
        println!(
            "Public encryption key: {}\nFor time being, it is identical to identity keys",
            sphinx_keypair.public_key().to_base58_string()
        );
        sphinx_keypair
    }

    pub fn new(config: Config) -> Self {
        let sphinx_keypair = Self::load_sphinx_keys(&config);

        MixNode {
            runtime: Runtime::new().unwrap(),
            config,
            sphinx_keypair,
        }
    }

    pub fn start_presence_notifier(&self) {
        let notifier_config = presence::NotifierConfig::new(
            self.config.get_presence_directory_server(),
            self.config.get_announce_address(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_layer(),
            self.config.get_presence_sending_delay(),
        );
        presence::Notifier::new(notifier_config).start(self.runtime.handle());
    }

    pub fn start_metrics_reporter(&self) -> metrics::MetricsReporter {
        metrics::MetricsController::new(
            self.config.get_metrics_directory_server(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_metrics_sending_delay(),
        )
        .start(self.runtime.handle())
    }

    pub fn start_socket_listener(&self, metrics_reporter: metrics::MetricsReporter) {
        // this is the only location where our private key is going to be copied
        // it will be held in memory owned by `MixNode` and inside an Arc of `PacketProcessor`
        let packet_processor =
            PacketProcessor::new(self.sphinx_keypair.private_key().clone(), metrics_reporter);

        listener::run_socket_listener(
            self.runtime.handle(),
            self.config.get_listening_address(),
            packet_processor,
        );
    }

    pub fn run(&mut self) {
        self.start_presence_notifier();
        let metrics_reporter = self.start_metrics_reporter();
        self.start_socket_listener(metrics_reporter);

        if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }

        println!(
            "Received SIGINT - the mixnode will terminate now (threads are not YET nicely stopped)"
        );
    }
}
