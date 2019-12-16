use nym_client::clients::directory;
use nym_client::clients::directory::DirectoryClient;
use nym_client::identity::mixnet::KeyPair;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use nym_client::clients::directory::presence::MixProviderPresence;
use nym_client::clients::directory::requests::presence_providers_post::PresenceMixProviderPoster;

pub struct Notifier {
    pub net_client: directory::Client,
    presence: MixProviderPresence,
}

impl Notifier {
    pub fn new(is_local: bool, host: SocketAddr, key_pair: &KeyPair) -> Notifier {
        let url = if is_local {
            "http://localhost:8080".to_string()
        } else {
            "https://directory.nymtech.net".to_string()
        };

        let key_bytes = key_pair.public.to_bytes().to_vec();
        let b64 = base64::encode_config(&key_bytes, base64::URL_SAFE);
        let public_key64 = b64.to_string();

        let config = directory::Config { base_url: url };
        let net_client = directory::Client::new(config);
        let presence = MixProviderPresence {
            host: host.to_string(),
            pub_key: public_key64,
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
