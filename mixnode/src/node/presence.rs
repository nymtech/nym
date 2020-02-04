use crate::built_info;
use directory_client::presence::mixnodes::MixNodePresence;
use directory_client::requests::presence_mixnodes_post::PresenceMixNodesPoster;
use directory_client::DirectoryClient;
use log::{debug, error};
use std::time::Duration;

pub struct Notifier {
    pub net_client: directory_client::Client,
    presence: MixNodePresence,
}

pub struct NotifierConfig {
    directory_server: String,
    announce_host: String,
    pub_key_string: String,
    layer: u64,
}

impl NotifierConfig {
    pub fn new(
        directory_server: String,
        announce_host: String,
        pub_key_string: String,
        layer: u64,
    ) -> Self {
        NotifierConfig {
            directory_server,
            announce_host,
            pub_key_string,
            layer,
        }
    }
}

impl Notifier {
    pub fn new(config: NotifierConfig) -> Notifier {
        let directory_client_cfg = directory_client::Config {
            base_url: config.directory_server,
        };
        let net_client = directory_client::Client::new(directory_client_cfg);
        let presence = MixNodePresence {
            host: config.announce_host,
            pub_key: config.pub_key_string,
            layer: config.layer,
            last_seen: 0,
            version: built_info::PKG_VERSION.to_string(),
        };
        Notifier {
            net_client,
            presence,
        }
    }

    pub fn notify(&self) {
        match self.net_client.presence_mix_nodes_post.post(&self.presence) {
            Err(err) => error!("failed to send presence - {:?}", err),
            Ok(_) => debug!("sent presence information"),
        }
    }

    pub async fn run(self) {
        let delay_duration = Duration::from_secs(5);

        loop {
            self.notify();
            tokio::time::delay_for(delay_duration).await;
        }
    }
}
