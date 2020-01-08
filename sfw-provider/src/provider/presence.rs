use crate::provider::{ClientLedger, Config};
use curve25519_dalek::montgomery::MontgomeryPoint;
use directory_client::presence::MixProviderPresence;
use directory_client::requests::presence_providers_post::PresenceMixProviderPoster;
use directory_client::DirectoryClient;
use futures::lock::Mutex as FMutex;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

pub struct Notifier {
    pub net_client: directory_client::Client,
    client_ledger: Arc<FMutex<ClientLedger>>,
    host: String,
    pub_key: String,
}

impl Notifier {
    pub fn new(
        directory_server_address: String,
        host: SocketAddr,
        pub_key: MontgomeryPoint,
        client_ledger: Arc<FMutex<ClientLedger>>,
    ) -> Notifier {
        let directory_config = directory_client::Config {
            base_url: directory_server_address,
        };
        let net_client = directory_client::Client::new(directory_config);

        Notifier {
            net_client,
            host: host.to_string(),
            pub_key: Config::public_key_string(pub_key),
            client_ledger,
        }
    }

    async fn make_presence(&self) -> MixProviderPresence {
        let unlocked_ledger = self.client_ledger.lock().await;

        MixProviderPresence {
            host: self.host.clone(),
            pub_key: self.pub_key.clone(),
            registered_clients: unlocked_ledger.current_clients(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn notify(&self, presence: MixProviderPresence) {
        self.net_client
            .presence_providers_post
            .post(&presence)
            .unwrap();
    }

    pub async fn run(self) {
        loop {
            let presence = self.make_presence().await;
            self.notify(presence);
            let delay_duration = Duration::from_secs(5);
            tokio::time::delay_for(delay_duration).await;
        }
    }
}
