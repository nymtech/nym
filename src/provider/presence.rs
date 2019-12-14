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
    pub fn new() -> Notifier {
        let config = directory::Config {
            base_url: "https://directory.nymtech.net/".to_string(),
        };
        let net_client = directory::Client::new(config);
        let presence = MixProviderPresence {
            host: "halpin.org:6666".to_string(),
            pub_key: "superkey".to_string(),
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
