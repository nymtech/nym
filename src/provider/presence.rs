use crate::provider::{ClientLedger, Config};
use curve25519_dalek::montgomery::MontgomeryPoint;
use futures::lock::Mutex as FMutex;
use nym_client::clients::directory;
use nym_client::clients::directory::presence::MixProviderPresence;
use nym_client::clients::directory::requests::presence_providers_post::PresenceMixProviderPoster;
use nym_client::clients::directory::DirectoryClient;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

pub struct Notifier {
    pub net_client: directory::Client,
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
        let directory_config = directory::Config {
            base_url: directory_server_address,
        };
        let net_client = directory::Client::new(directory_config);

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
