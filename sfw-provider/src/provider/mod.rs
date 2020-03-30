use crate::config::persistence::pathfinder::ProviderPathfinder;
use crate::config::Config;
use crate::provider::client_handling::ledger::ClientLedger;
use crate::provider::storage::ClientStorage;
use crypto::encryption;
use log::*;
use pemstore::pemstore::PemStore;
use tokio::runtime::Runtime;

mod client_handling;
mod mix_handling;
pub mod presence;
mod storage;

pub struct ServiceProvider {
    runtime: Runtime,
    config: Config,
    sphinx_keypair: encryption::KeyPair,
    registered_clients_ledger: ClientLedger,
}

impl ServiceProvider {
    fn load_sphinx_keys(config_file: &Config) -> encryption::KeyPair {
        let sphinx_keypair = PemStore::new(ProviderPathfinder::new_from_config(&config_file))
            .read_encryption()
            .expect("Failed to read stored sphinx key files");
        println!(
            "Public key: {}\n",
            sphinx_keypair.public_key().to_base58_string()
        );
        sphinx_keypair
    }

    pub fn new(config: Config) -> Self {
        let sphinx_keypair = Self::load_sphinx_keys(&config);
        let registered_clients_ledger = ClientLedger::load(config.get_clients_ledger_path());
        ServiceProvider {
            runtime: Runtime::new().unwrap(),
            config,
            sphinx_keypair,
            registered_clients_ledger,
        }
    }

    fn start_presence_notifier(&self) {
        info!("Starting presence notifier...");
        let notifier_config = presence::NotifierConfig::new(
            self.config.get_location(),
            self.config.get_presence_directory_server(),
            self.config.get_mix_announce_address(),
            self.config.get_clients_announce_address(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_presence_sending_delay(),
        );
        presence::Notifier::new(notifier_config, self.registered_clients_ledger.clone())
            .start(self.runtime.handle());
    }

    fn start_mix_socket_listener(&self, client_storage: ClientStorage) {
        info!("Starting mix socket listener...");
        let packet_processor = mix_handling::packet_processing::PacketProcessor::new(
            self.sphinx_keypair.private_key().clone(),
            client_storage,
        );

        mix_handling::listener::run_mix_socket_listener(
            self.runtime.handle(),
            self.config.get_mix_listening_address(),
            packet_processor,
        );
    }

    fn start_client_socket_listener(&self, client_storage: ClientStorage) {
        info!("Starting client socket listener...");
        let packet_processor = client_handling::request_processing::RequestProcessor::new(
            self.sphinx_keypair.private_key().clone(),
            client_storage,
            self.registered_clients_ledger.clone(),
        );

        client_handling::listener::run_client_socket_listener(
            self.runtime.handle(),
            self.config.get_clients_listening_address(),
            packet_processor,
        );
    }

    pub fn run(&mut self) {
        // A possible future optimisation, depending on bottlenecks and resource usage:
        // considering, presumably, there will be more mix packets received than client requests:
        // create 2 separate runtimes - one with bigger threadpool dedicated solely for
        // the mix handling and the other one for the rest of tasks

        let client_storage = ClientStorage::new(
            self.config.get_message_retrieval_limit() as usize,
            self.config.get_stored_messages_filename_length(),
            self.config.get_clients_inboxes_dir(),
        );

        self.start_presence_notifier();
        self.start_mix_socket_listener(client_storage.clone());
        self.start_client_socket_listener(client_storage);

        if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }

        println!(
            "Received SIGINT - the provider will terminate now (threads are not YET nicely stopped)"
        );
    }
}
