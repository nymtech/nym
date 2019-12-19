use crate::provider;
use nym_client::clients::directory;
use nym_client::clients::directory::DirectoryClient;
use std::time::Duration;

use crate::provider::Config;
use curve25519_dalek::montgomery::MontgomeryPoint;
use nym_client::clients::directory::presence::MixProviderPresence;
use nym_client::clients::directory::requests::presence_providers_post::PresenceMixProviderPoster;
use std::net::SocketAddr;

pub struct Notifier {
    pub net_client: directory::Client,
    presence: MixProviderPresence,
}

impl Notifier {
    pub fn new(
        directory_server_address: String,
        host: SocketAddr,
        pub_key: MontgomeryPoint,
    ) -> Notifier {
        let directory_config = directory::Config {
            base_url: directory_server_address,
        };
        let net_client = directory::Client::new(directory_config);
        let presence = MixProviderPresence {
            host: host.to_string(),
            pub_key: Config::public_key_string(pub_key),
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

    pub async fn run(self) {
        loop {
            self.notify();
            let delay_duration = Duration::from_secs(5);
            tokio::time::delay_for(delay_duration).await;
        }
    }
}
