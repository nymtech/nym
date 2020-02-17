use crate::built_info;
use crate::provider::ClientLedger;
use directory_client::presence::providers::MixProviderPresence;
use directory_client::requests::presence_providers_post::PresenceMixProviderPoster;
use directory_client::DirectoryClient;
use futures::lock::Mutex as FMutex;
use log::{debug, error};
use std::sync::Arc;
use std::time::Duration;

pub struct NotifierConfig {
    directory_server: String,
    mix_announce_host: String,
    clients_announce_host: String,
    pub_key_string: String,
    sending_delay: Duration,
}

impl NotifierConfig {
    pub fn new(
        directory_server: String,
        mix_announce_host: String,
        clients_announce_host: String,
        pub_key_string: String,
        sending_delay: Duration,
    ) -> Self {
        NotifierConfig {
            directory_server,
            mix_announce_host,
            clients_announce_host,
            pub_key_string,
            sending_delay,
        }
    }
}

pub struct Notifier {
    net_client: directory_client::Client,
    client_ledger: Arc<FMutex<ClientLedger>>,
    sending_delay: Duration,
    client_listener: String,
    mixnet_listener: String,
    pub_key_string: String,
}

impl Notifier {
    pub fn new(config: NotifierConfig, client_ledger: Arc<FMutex<ClientLedger>>) -> Notifier {
        let directory_client_cfg = directory_client::Config {
            base_url: config.directory_server,
        };
        let net_client = directory_client::Client::new(directory_client_cfg);

        Notifier {
            client_ledger,
            net_client,
            client_listener: config.clients_announce_host,
            mixnet_listener: config.mix_announce_host,
            pub_key_string: config.pub_key_string,
            sending_delay: config.sending_delay,
        }
    }

    async fn make_presence(&self) -> MixProviderPresence {
        let unlocked_ledger = self.client_ledger.lock().await;

        MixProviderPresence {
            client_listener: self.client_listener.clone(),
            mixnet_listener: self.mixnet_listener.clone(),
            pub_key: self.pub_key_string.clone(),
            registered_clients: unlocked_ledger.current_clients(),
            last_seen: 0,
            version: built_info::PKG_VERSION.to_string(),
        }
    }

    pub fn notify(&self, presence: MixProviderPresence) {
        match self.net_client.presence_providers_post.post(&presence) {
            Err(err) => error!("failed to send presence - {:?}", err),
            Ok(_) => debug!("sent presence information"),
        }
    }

    pub async fn run(self) {
        loop {
            let presence = self.make_presence().await;
            self.notify(presence);
            tokio::time::delay_for(self.sending_delay).await;
        }
    }
}
