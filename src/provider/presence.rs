use crate::provider;
use nym_client::clients::directory;
use nym_client::clients::directory::DirectoryClient;
use std::thread;
use std::time::Duration;

use nym_client::clients::directory::presence::MixProviderPresence;
use nym_client::clients::directory::requests::presence_providers_post::PresenceMixProviderPoster;

pub struct Notifier {
    pub net_client: directory::Client,
    presence: MixProviderPresence,
}

impl Notifier {
    pub fn new(config: &provider::Config) -> Notifier {
        let directory_config = directory::Config {
            base_url: config.directory_server.clone(),
        };
        let net_client = directory::Client::new(directory_config);
        let presence = MixProviderPresence {
            host: config.mix_socket_address.to_string(),
            pub_key: config.public_key_string(),
            registered_clients: vec![],
        };
        Notifier {
            net_client,
            presence,
        }
    }

    pub fn notify(&self) {
        self.net_client
            .presence_providers_post
            .post(&self.presence)
            .unwrap();
    }

    pub fn run(&self) {
        loop {
            self.notify();
            thread::sleep(Duration::from_secs(5));
        }
    }
}
