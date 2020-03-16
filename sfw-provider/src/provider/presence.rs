use crate::built_info;
use crate::provider::ClientLedger;
use directory_client::presence::providers::MixProviderPresence;
use directory_client::requests::presence_providers_post::PresenceMixProviderPoster;
use directory_client::DirectoryClient;
use log::{debug, error};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub struct NotifierConfig {
    location: String,
    directory_server: String,
    mix_announce_host: String,
    clients_announce_host: String,
    pub_key_string: String,
    sending_delay: Duration,
}

impl NotifierConfig {
    pub fn new(
        location: String,
        directory_server: String,
        mix_announce_host: String,
        clients_announce_host: String,
        pub_key_string: String,
        sending_delay: Duration,
    ) -> Self {
        NotifierConfig {
            location,
            directory_server,
            mix_announce_host,
            clients_announce_host,
            pub_key_string,
            sending_delay,
        }
    }
}

pub struct Notifier {
    location: String,
    net_client: directory_client::Client,
    client_ledger: ClientLedger,
    sending_delay: Duration,
    client_listener: String,
    mixnet_listener: String,
    pub_key_string: String,
}

impl Notifier {
    pub fn new(config: NotifierConfig, client_ledger: ClientLedger) -> Notifier {
        let directory_client_cfg = directory_client::Config {
            base_url: config.directory_server,
        };
        let net_client = directory_client::Client::new(directory_client_cfg);

        Notifier {
            client_ledger,
            net_client,
            location: config.location,
            client_listener: config.clients_announce_host,
            mixnet_listener: config.mix_announce_host,
            pub_key_string: config.pub_key_string,
            sending_delay: config.sending_delay,
        }
    }

    async fn make_presence(&self) -> MixProviderPresence {
        MixProviderPresence {
            location: self.location.clone(),
            client_listener: self.client_listener.clone(),
            mixnet_listener: self.mixnet_listener.clone(),
            pub_key: self.pub_key_string.clone(),
            registered_clients: self.client_ledger.current_clients().await,
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

    pub fn start(self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            loop {
                // set the deadline in the future
                let sending_delay = tokio::time::delay_for(self.sending_delay);
                let presence = self.make_presence().await;
                self.notify(presence);
                // wait for however much is left
                sending_delay.await;
            }
        })
    }
}
