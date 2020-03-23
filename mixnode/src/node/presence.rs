use crate::built_info;
use directory_client::presence::mixnodes::MixNodePresence;
use directory_client::requests::presence_mixnodes_post::PresenceMixNodesPoster;
use directory_client::DirectoryClient;
use log::{debug, error};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

pub struct NotifierConfig {
    location: String,
    directory_server: String,
    announce_host: String,
    pub_key_string: String,
    layer: u64,
    sending_delay: Duration,
}

impl NotifierConfig {
    pub fn new(
        location: String,
        directory_server: String,
        announce_host: String,
        pub_key_string: String,
        layer: u64,
        sending_delay: Duration,
    ) -> Self {
        NotifierConfig {
            location,
            directory_server,
            announce_host,
            pub_key_string,
            layer,
            sending_delay,
        }
    }
}

pub struct Notifier {
    net_client: directory_client::Client,
    presence: MixNodePresence,
    sending_delay: Duration,
}

impl Notifier {
    pub fn new(config: NotifierConfig) -> Notifier {
        let directory_client_cfg = directory_client::Config {
            base_url: config.directory_server,
        };
        let net_client = directory_client::Client::new(directory_client_cfg);
        let presence = MixNodePresence {
            location: config.location,
            host: config.announce_host,
            pub_key: config.pub_key_string,
            layer: config.layer,
            last_seen: 0,
            version: built_info::PKG_VERSION.to_string(),
        };
        Notifier {
            net_client,
            presence,
            sending_delay: config.sending_delay,
        }
    }

    fn notify(&self) {
        match self.net_client.presence_mix_nodes_post.post(&self.presence) {
            Err(err) => error!("failed to send presence - {:?}", err),
            Ok(_) => debug!("sent presence information"),
        }
    }

    pub fn start(self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            loop {
                // set the deadline in the future
                let sending_delay = tokio::time::delay_for(self.sending_delay);
                self.notify();
                // wait for however much is left
                sending_delay.await;
            }
        })
    }
}
